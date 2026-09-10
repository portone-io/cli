use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail, ensure};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use url::Url;

use super::model::{Bundle, McpServer, REPOSITORY, Revision, SKILL_NAMES, SkillBundle};

const GITHUB_API: &str = "https://api.github.com/";
const MAX_METADATA_BYTES: usize = 2 * 1024 * 1024;
const MAX_BLOB_BYTES: usize = 4 * 1024 * 1024;
const MAX_BLOB_RESPONSE_BYTES: usize = 6 * 1024 * 1024;
const MAX_BUNDLE_BYTES: usize = 16 * 1024 * 1024;
const MAX_FILES: usize = 512;

pub struct SourceClient {
    agent: ureq::Agent,
    repository_url: Url,
    token: Option<String>,
}

impl SourceClient {
    pub fn new() -> anyhow::Result<Self> {
        let token = token_from(|name| std::env::var(name).ok());
        Self::from_api_url(GITHUB_API, token)
    }

    #[cfg(test)]
    pub(crate) fn for_test(api_url: &str, token: Option<String>) -> anyhow::Result<Self> {
        Self::from_api_url(api_url, token)
    }

    fn from_api_url(api_url: &str, token: Option<String>) -> anyhow::Result<Self> {
        let mut api_url = Url::parse(api_url).context("invalid GitHub API URL")?;
        ensure!(
            matches!(api_url.scheme(), "http" | "https") && api_url.host_str().is_some(),
            "GitHub API URL must be HTTP(S) with a host"
        );
        if !api_url.path().ends_with('/') {
            let path = format!("{}/", api_url.path());
            api_url.set_path(&path);
        }
        let repository_url = api_url
            .join(&format!("repos/{REPOSITORY}/"))
            .context("failed to construct GitHub repository URL")?;

        if let Some(token) = token.as_deref() {
            let authorization = format!("Bearer {token}");
            ureq::http::HeaderValue::from_str(&authorization)
                .map_err(|_| anyhow::anyhow!("invalid GitHub token header value"))?;
        }

        let tls = ureq::tls::TlsConfig::builder()
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(30)))
            .max_redirects(0)
            .redirect_auth_headers(ureq::config::RedirectAuthHeaders::Never)
            .tls_config(tls)
            .build();

        Ok(Self {
            agent: ureq::Agent::new_with_config(config),
            repository_url,
            token,
        })
    }

    pub fn fetch_bundle(&self) -> anyhow::Result<Bundle> {
        let reference = self.resolve_reference()?;
        let commit_url = self.endpoint(&["commits", &reference])?;
        let commit: Commit = self.get_json(commit_url, MAX_METADATA_BYTES, "resolve commit")?;
        validate_commit_sha(&commit.sha)?;
        validate_commit_sha(&commit.commit.tree.sha)?;

        let mut tree_url = self.endpoint(&["git", "trees", &commit.commit.tree.sha])?;
        tree_url.query_pairs_mut().append_pair("recursive", "1");
        let tree: GitTree = self.get_json(tree_url, MAX_METADATA_BYTES, "fetch repository tree")?;
        ensure!(
            !tree.truncated,
            "GitHub returned a truncated repository tree"
        );
        ensure!(
            tree.sha == commit.commit.tree.sha,
            "GitHub returned a different repository tree"
        );

        let selected = select_files(tree.tree)?;
        let mut total_bytes = 0usize;
        let mut skills = BTreeMap::new();

        for name in SKILL_NAMES {
            let selected_skill = selected
                .skills
                .get(name)
                .with_context(|| format!("missing skill directory: skills/{name}"))?;
            let mut files = BTreeMap::new();
            for file in &selected_skill.files {
                let bytes = self.fetch_blob(file)?;
                total_bytes = total_bytes
                    .checked_add(bytes.len())
                    .context("bundle byte count overflow")?;
                ensure!(total_bytes <= MAX_BUNDLE_BYTES, "skill bundle is too large");
                ensure!(
                    files.insert(file.relative_path.clone(), bytes).is_none(),
                    "duplicate skill file path: {}",
                    file.relative_path.display()
                );
            }

            let skill_file = files
                .get(Path::new("SKILL.md"))
                .with_context(|| format!("missing skills/{name}/SKILL.md"))?;
            validate_skill_frontmatter(skill_file, name)?;
            skills.insert(
                name.to_string(),
                SkillBundle {
                    tree_sha: selected_skill.tree_sha.clone(),
                    files,
                },
            );
        }

        let mcp_bytes = self.fetch_blob(&selected.mcp)?;
        total_bytes = total_bytes
            .checked_add(mcp_bytes.len())
            .context("bundle byte count overflow")?;
        ensure!(total_bytes <= MAX_BUNDLE_BYTES, "skill bundle is too large");
        let mcp = parse_mcp(&mcp_bytes)?;

        Ok(Bundle {
            revision: Revision {
                repository: REPOSITORY.to_string(),
                reference,
                commit: commit.sha,
            },
            skills,
            mcp,
        })
    }

    fn resolve_reference(&self) -> anyhow::Result<String> {
        let release_url = self.endpoint(&["releases", "latest"])?;
        let (status, body) = self.get(release_url, MAX_METADATA_BYTES)?;
        let reference = match status {
            200..=299 => {
                let release: Release = decode_json(&body, "latest release")?;
                release.tag_name
            }
            404 => {
                let repository_url = self.endpoint(&[])?;
                let repository: Repository =
                    self.get_json(repository_url, MAX_METADATA_BYTES, "fetch repository")?;
                repository.default_branch
            }
            _ => return Err(status_error(status, &body, "fetch latest release")),
        };
        ensure!(!reference.is_empty(), "GitHub returned an empty reference");
        ensure!(reference.len() <= 255, "GitHub reference is too long");
        ensure!(
            !reference.chars().any(char::is_control),
            "GitHub reference contains control characters"
        );
        Ok(reference)
    }

    fn fetch_blob(&self, file: &SelectedFile) -> anyhow::Result<Vec<u8>> {
        ensure!(
            file.size <= MAX_BLOB_BYTES,
            "blob is too large: {}",
            file.source_path
        );
        let url = self.endpoint(&["git", "blobs", &file.sha])?;
        let blob: GitBlob = self.get_json(url, MAX_BLOB_RESPONSE_BYTES, "fetch blob")?;
        ensure!(
            blob.encoding == "base64",
            "unsupported blob encoding: {}",
            blob.encoding
        );

        let compact: Vec<u8> = blob
            .content
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect();
        let bytes = STANDARD
            .decode(compact)
            .with_context(|| format!("invalid base64 for {}", file.source_path))?;
        ensure!(
            bytes.len() <= MAX_BLOB_BYTES,
            "blob is too large: {}",
            file.source_path
        );
        ensure!(
            bytes.len() == file.size && bytes.len() == blob.size,
            "blob size mismatch for {}",
            file.source_path
        );
        Ok(bytes)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: Url,
        limit: usize,
        operation: &str,
    ) -> anyhow::Result<T> {
        let (status, body) = self.get(url, limit)?;
        ensure!(
            (200..300).contains(&status),
            "{}",
            status_error(status, &body, operation)
        );
        decode_json(&body, operation)
    }

    fn get(&self, url: Url, limit: usize) -> anyhow::Result<(u16, Vec<u8>)> {
        let mut request = self
            .agent
            .get(url.as_str())
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(
                "User-Agent",
                concat!("portone-cli/", env!("CARGO_PKG_VERSION")),
            );
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let response = request
            .call()
            .with_context(|| format!("GitHub request failed: {url}"))?;
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .into_with_config()
            .limit((limit + 1) as u64)
            .read_to_vec()
            .with_context(|| format!("failed to read GitHub response: {url}"))?;
        ensure!(body.len() <= limit, "GitHub response is too large: {url}");
        Ok((status, body))
    }

    fn endpoint(&self, segments: &[&str]) -> anyhow::Result<Url> {
        let mut url = self.repository_url.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow::anyhow!("GitHub repository URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(segments.iter().copied());
        Ok(url)
    }
}

fn token_from(mut get: impl FnMut(&str) -> Option<String>) -> Option<String> {
    ["GH_TOKEN", "GITHUB_TOKEN"]
        .into_iter()
        .find_map(|name| get(name).filter(|token| !token.is_empty()))
}

fn decode_json<T: for<'de> Deserialize<'de>>(body: &[u8], name: &str) -> anyhow::Result<T> {
    serde_json::from_slice(body).with_context(|| format!("invalid GitHub {name} response"))
}

fn status_error(status: u16, body: &[u8], operation: &str) -> anyhow::Error {
    let detail = serde_json::from_slice::<GitHubError>(body)
        .ok()
        .map(|error| error.message)
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| "request failed".to_string());
    anyhow::anyhow!("GitHub {operation} failed with HTTP {status}: {detail}")
}

fn validate_commit_sha(sha: &str) -> anyhow::Result<()> {
    ensure!(
        sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "GitHub returned an invalid commit SHA"
    );
    Ok(())
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

#[derive(Deserialize)]
struct Repository {
    default_branch: String,
}

#[derive(Deserialize)]
struct Commit {
    sha: String,
    commit: CommitDetails,
}

#[derive(Deserialize)]
struct CommitDetails {
    tree: CommitTree,
}

#[derive(Deserialize)]
struct CommitTree {
    sha: String,
}

#[derive(Deserialize)]
struct GitTree {
    sha: String,
    #[serde(default)]
    truncated: bool,
    tree: Vec<TreeEntry>,
}

#[derive(Deserialize)]
struct TreeEntry {
    path: String,
    mode: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<usize>,
}

#[derive(Deserialize)]
struct GitBlob {
    encoding: String,
    content: String,
    size: usize,
}

#[derive(Deserialize)]
struct GitHubError {
    message: String,
}

struct SelectedBundle {
    skills: BTreeMap<String, SelectedSkill>,
    mcp: SelectedFile,
}

struct SelectedSkill {
    tree_sha: String,
    files: Vec<SelectedFile>,
}

struct SelectedFile {
    source_path: String,
    relative_path: PathBuf,
    sha: String,
    size: usize,
}

fn select_files(entries: Vec<TreeEntry>) -> anyhow::Result<SelectedBundle> {
    let mut seen = HashSet::new();
    let mut roots = BTreeMap::new();
    let mut files: BTreeMap<String, Vec<SelectedFile>> = SKILL_NAMES
        .into_iter()
        .map(|name| (name.to_string(), Vec::new()))
        .collect();
    let mut mcp = None;
    let mut selected_file_count = 0usize;
    let mut declared_bytes = 0usize;

    for entry in entries {
        ensure!(
            seen.insert(entry.path.clone()),
            "duplicate Git tree path: {}",
            entry.path
        );
        for name in SKILL_NAMES {
            let root = format!("skills/{name}");
            if entry.path == root {
                ensure!(
                    entry.kind == "tree" && entry.mode == "040000",
                    "skill root is not a directory: {root}"
                );
                ensure!(
                    roots.insert(name.to_string(), entry.sha.clone()).is_none(),
                    "duplicate skill root: {root}"
                );
                continue;
            }
            let prefix = format!("{root}/");
            if let Some(relative) = entry.path.strip_prefix(&prefix) {
                validate_relative_path(relative)?;
                match (entry.kind.as_str(), entry.mode.as_str()) {
                    ("tree", "040000") => {}
                    ("blob", "100644" | "100755") => {
                        let size = entry
                            .size
                            .with_context(|| format!("missing blob size: {}", entry.path))?;
                        register_file(size, &mut selected_file_count, &mut declared_bytes)?;
                        files
                            .get_mut(name)
                            .expect("known skill")
                            .push(SelectedFile {
                                source_path: entry.path.clone(),
                                relative_path: PathBuf::from(relative),
                                sha: entry.sha.clone(),
                                size,
                            });
                    }
                    _ => bail!("unsupported Git tree entry: {}", entry.path),
                }
            }
        }

        if entry.path == ".mcp.json" {
            ensure!(
                entry.kind == "blob" && matches!(entry.mode.as_str(), "100644" | "100755"),
                ".mcp.json is not a regular file"
            );
            let size = entry.size.context("missing blob size: .mcp.json")?;
            register_file(size, &mut selected_file_count, &mut declared_bytes)?;
            ensure!(mcp.is_none(), "duplicate .mcp.json entry");
            mcp = Some(SelectedFile {
                source_path: entry.path,
                relative_path: PathBuf::from(".mcp.json"),
                sha: entry.sha,
                size,
            });
        }
    }

    let mut skills = BTreeMap::new();
    for name in SKILL_NAMES {
        let tree_sha = roots
            .remove(name)
            .with_context(|| format!("missing skill directory: skills/{name}"))?;
        let skill_files = files.remove(name).expect("known skill");
        ensure!(
            !skill_files.is_empty(),
            "skill directory is empty: skills/{name}"
        );
        ensure!(
            skill_files
                .iter()
                .any(|file| file.relative_path == Path::new("SKILL.md")),
            "missing skills/{name}/SKILL.md"
        );
        skills.insert(
            name.to_string(),
            SelectedSkill {
                tree_sha,
                files: skill_files,
            },
        );
    }

    Ok(SelectedBundle {
        skills,
        mcp: mcp.context("missing root .mcp.json")?,
    })
}

fn register_file(size: usize, count: &mut usize, total: &mut usize) -> anyhow::Result<()> {
    ensure!(size <= MAX_BLOB_BYTES, "blob is too large");
    *count = count.checked_add(1).context("file count overflow")?;
    ensure!(*count <= MAX_FILES, "bundle contains too many files");
    *total = total
        .checked_add(size)
        .context("bundle byte count overflow")?;
    ensure!(*total <= MAX_BUNDLE_BYTES, "skill bundle is too large");
    Ok(())
}

fn validate_relative_path(path: &str) -> anyhow::Result<()> {
    ensure!(!path.is_empty(), "empty skill file path");
    ensure!(!path.contains('\\'), "unsafe skill file path: {path}");
    ensure!(
        path.split('/')
            .all(|component| !component.is_empty() && component != "." && component != ".."),
        "unsafe skill file path: {path}"
    );
    let path = Path::new(path);
    ensure!(
        !path.is_absolute(),
        "unsafe skill file path: {}",
        path.display()
    );
    ensure!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_))),
        "unsafe skill file path: {}",
        path.display()
    );
    Ok(())
}

fn validate_skill_frontmatter(bytes: &[u8], expected_name: &str) -> anyhow::Result<()> {
    let text = std::str::from_utf8(bytes)
        .with_context(|| format!("skills/{expected_name}/SKILL.md is not UTF-8"))?;
    let mut lines = text.lines();
    ensure!(
        lines.next() == Some("---"),
        "skills/{expected_name}/SKILL.md has no YAML frontmatter"
    );
    let mut name = None;
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        if let Some(value) = line.strip_prefix("name:") {
            ensure!(
                name.is_none(),
                "skills/{expected_name}/SKILL.md has duplicate name fields"
            );
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .or_else(|| {
                    value
                        .strip_prefix('\'')
                        .and_then(|value| value.strip_suffix('\''))
                })
                .unwrap_or(value);
            name = Some(value);
        }
    }
    ensure!(
        closed,
        "skills/{expected_name}/SKILL.md has unterminated YAML frontmatter"
    );
    ensure!(
        name == Some(expected_name),
        "skills/{expected_name}/SKILL.md frontmatter name does not match"
    );
    Ok(())
}

#[derive(Deserialize)]
struct McpConfig {
    #[serde(rename = "mcpServers")]
    mcp_servers: BTreeMap<String, McpDefinition>,
}

#[derive(Deserialize)]
struct McpDefinition {
    #[serde(rename = "type")]
    kind: String,
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
}

fn parse_mcp(bytes: &[u8]) -> anyhow::Result<McpServer> {
    let config: McpConfig = serde_json::from_slice(bytes).context("invalid root .mcp.json")?;
    let portone = config
        .mcp_servers
        .get("portone")
        .context("root .mcp.json has no portone server")?;
    ensure!(portone.kind == "stdio", "PortOne MCP server must use stdio");
    ensure!(!portone.command.is_empty(), "PortOne MCP command is empty");
    ensure!(
        !portone.command.contains('\0'),
        "PortOne MCP command contains NUL"
    );
    ensure!(
        portone.args.iter().all(|arg| !arg.contains('\0')),
        "PortOne MCP argument contains NUL"
    );
    ensure!(
        portone.env.iter().all(|(key, value)| !key.is_empty()
            && !key.contains(['=', '\0'])
            && !value.contains('\0')),
        "PortOne MCP environment is invalid"
    );
    Ok(McpServer {
        command: portone.command.clone(),
        args: portone.args.clone(),
        env: portone.env.clone(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use httpmock::Mock;
    use httpmock::prelude::*;
    use serde_json::{Value, json};

    use super::*;
    use crate::cmd::setup::model::SKILL_NAMES;

    const API_ROOT: &str = "/repos/portone-io/portone-cli";
    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";

    fn skill_markdown(name: &str) -> Vec<u8> {
        format!("---\nname: {name}\ndescription: Test fixture\n---\n\n# {name}\n").into_bytes()
    }

    fn mcp_json() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "mcpServers": {
                "portone": {
                    "type": "stdio",
                    "command": "npx",
                    "args": ["-y", "@portone/mcp-server@latest"],
                    "env": {"LOG_LEVEL": "info"}
                }
            }
        }))
        .unwrap()
    }

    fn encoded_blob(bytes: &[u8]) -> Value {
        json!({
            "encoding": "base64",
            "content": STANDARD.encode(bytes),
            "size": bytes.len()
        })
    }

    fn complete_tree(extra: &[Value]) -> Value {
        let mut entries = Vec::new();
        for (index, name) in SKILL_NAMES.iter().enumerate() {
            entries.push(json!({
                "path": format!("skills/{name}"),
                "mode": "040000",
                "type": "tree",
                "sha": format!("tree-{index}")
            }));
            entries.push(json!({
                "path": format!("skills/{name}/SKILL.md"),
                "mode": "100644",
                "type": "blob",
                "sha": format!("skill-{index}"),
                "size": skill_markdown(name).len()
            }));
        }
        entries.push(json!({
            "path": "skills/portone-cli/assets/logo.bin",
            "mode": "100644",
            "type": "blob",
            "sha": "binary",
            "size": 4
        }));
        entries.push(json!({
            "path": ".mcp.json",
            "mode": "100644",
            "type": "blob",
            "sha": "mcp",
            "size": mcp_json().len()
        }));
        entries.extend_from_slice(extra);
        json!({"sha": TREE, "truncated": false, "tree": entries})
    }

    fn mock_json<'a>(
        server: &'a MockServer,
        path: &str,
        status: u16,
        body: Value,
        token: Option<&str>,
    ) -> Mock<'a> {
        server.mock(|when, then| {
            if let Some(token) = token {
                when.method(GET)
                    .path(path)
                    .header("authorization", format!("Bearer {token}"));
            } else {
                when.method(GET).path(path);
            }
            then.status(status).json_body(body);
        })
    }

    fn mock_blobs<'a>(server: &'a MockServer, token: Option<&str>) -> Vec<Mock<'a>> {
        let mut mocks = Vec::new();
        for (index, name) in SKILL_NAMES.iter().enumerate() {
            mocks.push(mock_json(
                server,
                &format!("{API_ROOT}/git/blobs/skill-{index}"),
                200,
                encoded_blob(&skill_markdown(name)),
                token,
            ));
        }
        mocks.push(mock_json(
            server,
            &format!("{API_ROOT}/git/blobs/binary"),
            200,
            encoded_blob(&[0, 159, 146, 150]),
            token,
        ));
        mocks.push(mock_json(
            server,
            &format!("{API_ROOT}/git/blobs/mcp"),
            200,
            encoded_blob(&mcp_json()),
            token,
        ));
        mocks
    }

    #[test]
    fn latest_release_builds_one_commit_bundle_and_preserves_binary_files() {
        let server = MockServer::start();
        let token = "github-secret";
        let release = mock_json(
            &server,
            &format!("{API_ROOT}/releases/latest"),
            200,
            json!({"tag_name": "v1.2.3"}),
            Some(token),
        );
        let commit = mock_json(
            &server,
            &format!("{API_ROOT}/commits/v1.2.3"),
            200,
            json!({"sha": COMMIT, "commit": {"tree": {"sha": TREE}}}),
            Some(token),
        );
        let tree = server.mock(|when, then| {
            when.method(GET)
                .path(format!("{API_ROOT}/git/trees/{TREE}"))
                .query_param("recursive", "1")
                .header("authorization", format!("Bearer {token}"));
            then.status(200).json_body(complete_tree(&[]));
        });
        let blobs = mock_blobs(&server, Some(token));

        let bundle = SourceClient::for_test(&server.base_url(), Some(token.to_string()))
            .unwrap()
            .fetch_bundle()
            .unwrap();

        assert_eq!(bundle.revision.repository, "portone-io/portone-cli");
        assert_eq!(bundle.revision.reference, "v1.2.3");
        assert_eq!(bundle.revision.commit, COMMIT);
        assert_eq!(bundle.skills.len(), 4);
        assert_eq!(bundle.skills["portone-cli"].tree_sha, "tree-0");
        assert_eq!(
            bundle.skills["portone-cli"].files[&PathBuf::from("assets/logo.bin")],
            [0, 159, 146, 150]
        );
        assert_eq!(bundle.mcp.command, "npx");
        assert_eq!(bundle.mcp.args, ["-y", "@portone/mcp-server@latest"]);
        assert_eq!(
            bundle.mcp.env,
            BTreeMap::from([("LOG_LEVEL".to_string(), "info".to_string())])
        );
        release.assert();
        commit.assert();
        tree.assert();
        for mock in blobs {
            mock.assert();
        }
    }

    #[test]
    fn missing_latest_release_falls_back_to_default_branch() {
        let server = MockServer::start();
        let release = mock_json(
            &server,
            &format!("{API_ROOT}/releases/latest"),
            404,
            json!({"message": "Not Found"}),
            None,
        );
        let repository = mock_json(
            &server,
            API_ROOT,
            200,
            json!({"default_branch": "main"}),
            None,
        );
        let commit = mock_json(
            &server,
            &format!("{API_ROOT}/commits/main"),
            200,
            json!({"sha": COMMIT, "commit": {"tree": {"sha": TREE}}}),
            None,
        );
        let tree = server.mock(|when, then| {
            when.method(GET)
                .path(format!("{API_ROOT}/git/trees/{TREE}"))
                .query_param("recursive", "1");
            then.status(200).json_body(complete_tree(&[]));
        });
        let blobs = mock_blobs(&server, None);

        let bundle = SourceClient::for_test(&server.base_url(), None)
            .unwrap()
            .fetch_bundle()
            .unwrap();

        assert_eq!(bundle.revision.reference, "main");
        release.assert();
        repository.assert();
        commit.assert();
        tree.assert();
        for mock in blobs {
            mock.assert();
        }
    }

    #[test]
    fn latest_release_error_does_not_fall_back_to_a_branch() {
        let server = MockServer::start();
        let release = mock_json(
            &server,
            &format!("{API_ROOT}/releases/latest"),
            503,
            json!({"message": "temporarily unavailable"}),
            None,
        );
        let repository = mock_json(
            &server,
            API_ROOT,
            200,
            json!({"default_branch": "main"}),
            None,
        );

        let error = SourceClient::for_test(&server.base_url(), None)
            .unwrap()
            .fetch_bundle()
            .unwrap_err();

        assert!(error.to_string().contains("HTTP 503"));
        assert!(error.to_string().contains("temporarily unavailable"));
        release.assert();
        repository.assert_calls(0);
    }

    #[test]
    fn network_failure_is_reported() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let client = SourceClient::for_test(&format!("http://{address}"), None).unwrap();

        let error = client.fetch_bundle().unwrap_err();

        assert!(error.to_string().contains("GitHub request failed"));
    }

    #[test]
    fn unsafe_paths_symlinks_and_submodules_are_rejected() {
        for entry in [
            json!({
                "path": "skills/portone-cli/../escape",
                "mode": "100644",
                "type": "blob",
                "sha": "escape",
                "size": 1
            }),
            json!({
                "path": "skills/portone-cli/link",
                "mode": "120000",
                "type": "blob",
                "sha": "link",
                "size": 3
            }),
            json!({
                "path": "skills/portone-cli/dependency",
                "mode": "160000",
                "type": "commit",
                "sha": "submodule"
            }),
        ] {
            let tree: GitTree = serde_json::from_value(complete_tree(&[entry])).unwrap();
            assert!(select_files(tree.tree).is_err());
        }
    }

    #[test]
    fn every_canonical_skill_and_its_skill_markdown_are_required() {
        let mut value = complete_tree(&[]);
        value["tree"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["path"] != "skills/integration-validator");
        let tree: GitTree = serde_json::from_value(value).unwrap();
        assert!(select_files(tree.tree).is_err());

        let mut value = complete_tree(&[]);
        value["tree"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["path"] != "skills/integration-validator/SKILL.md");
        let tree: GitTree = serde_json::from_value(value).unwrap();
        assert!(select_files(tree.tree).is_err());
    }

    #[test]
    fn frontmatter_name_must_match_the_canonical_skill_id() {
        assert!(
            validate_skill_frontmatter(b"---\nname: portone-guide\n---\n", "integration-validator")
                .is_err()
        );
        assert!(
            validate_skill_frontmatter(
                b"---\nname: integration-validator\nname: integration-validator\n---\n",
                "integration-validator"
            )
            .is_err()
        );
        assert!(
            validate_skill_frontmatter(
                b"---\nname: integration-validator\n",
                "integration-validator"
            )
            .is_err()
        );
    }

    #[test]
    fn mcp_requires_a_string_only_stdio_portone_definition() {
        assert!(parse_mcp(b"{").is_err());
        for invalid in [
            json!({"mcpServers": {}}),
            json!({"mcpServers": {"portone": {
                "type": "http", "command": "npx", "args": [], "env": {}
            }}}),
            json!({"mcpServers": {"portone": {
                "type": "stdio", "command": ["npx"], "args": [], "env": {}
            }}}),
            json!({"mcpServers": {"portone": {
                "type": "stdio", "command": "npx", "args": [1], "env": {}
            }}}),
            json!({"mcpServers": {"portone": {
                "type": "stdio", "command": "npx", "args": [], "env": {"TOKEN": 1}
            }}}),
        ] {
            assert!(parse_mcp(&serde_json::to_vec(&invalid).unwrap()).is_err());
        }
    }

    #[test]
    fn github_token_lookup_uses_only_github_names_in_priority_order() {
        let mut requested = Vec::new();
        let token = token_from(|name| {
            requested.push(name.to_string());
            (name == "GITHUB_TOKEN").then(|| "fallback".to_string())
        });
        assert_eq!(token.as_deref(), Some("fallback"));
        assert_eq!(requested, ["GH_TOKEN", "GITHUB_TOKEN"]);

        requested.clear();
        let token = token_from(|name| {
            requested.push(name.to_string());
            Some("preferred".to_string())
        });
        assert_eq!(token.as_deref(), Some("preferred"));
        assert_eq!(requested, ["GH_TOKEN"]);
    }

    #[test]
    fn declared_file_limits_are_enforced_before_blob_downloads() {
        let mut tree = complete_tree(&[]);
        tree["tree"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["path"] == ".mcp.json")
            .unwrap()["size"] = json!(MAX_BLOB_BYTES + 1);
        let tree: GitTree = serde_json::from_value(tree).unwrap();
        assert!(select_files(tree.tree).is_err());
    }
}
