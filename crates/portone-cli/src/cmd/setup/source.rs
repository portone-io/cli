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
