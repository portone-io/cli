use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::adapters::{render_mcp, validate_destination};
use super::model::{Agent, Bundle, Destination, REPOSITORY, SKILL_NAMES, Scope};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    version: u32,
    scope: Scope,
    targets: Vec<Destination>,
    artifacts: Vec<Artifact>,
    #[serde(skip)]
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum ArtifactKind {
    Skill { name: String },
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: PathBuf,
    #[serde(rename = "content")]
    kind: ArtifactKind,
    last_success: Option<LastSuccess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LastSuccess {
    commit: String,
    content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallState {
    Unchanged,
    WouldUpdate,
    Updated,
    Failed,
}

#[derive(Debug)]
pub struct InstallItem {
    pub path: PathBuf,
    pub state: InstallState,
}

#[derive(Debug, Default)]
pub struct InstallReport {
    pub items: Vec<InstallItem>,
    pub errors: Vec<String>,
    pub successful_agents: Vec<Agent>,
}

pub struct PreparedInstall {
    receipt: Receipt,
    changes: Vec<Change>,
    agents: Vec<Agent>,
}

struct Change {
    index: usize,
    path: PathBuf,
    content: Content,
    changed: bool,
    success: LastSuccess,
    agents: Vec<Agent>,
}

enum Content {
    File(Vec<u8>),
    Directory(BTreeMap<PathBuf, Vec<u8>>),
}

impl Receipt {
    pub fn new(scope: Scope, root: &Path) -> Self {
        Self {
            version: 1,
            scope,
            root: root.to_path_buf(),
            targets: vec![],
            artifacts: vec![],
        }
    }

    pub fn load(path: &Path, scope: Scope, root: &Path) -> Result<Option<Self>> {
        check_path(path)?;
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Cannot read receipt {}", path.display()));
            }
        };
        let mut receipt: Self = serde_json::from_slice(&bytes)
            .with_context(|| format!("Invalid receipt {}", path.display()))?;
        receipt.root = root.to_path_buf();
        ensure!(
            receipt.version == 1,
            "Unsupported setup receipt version {}",
            receipt.version
        );
        ensure!(receipt.scope == scope, "Setup receipt scope mismatch");
        receipt.validate()?;
        Ok(Some(receipt))
    }

    pub fn agents(&self) -> Vec<Agent> {
        self.targets.iter().map(|target| target.agent).collect()
    }

    pub fn add_targets(&mut self, targets: &[Destination]) -> Result<()> {
        let mut candidate = self.clone();
        for target in targets {
            let saved = Destination {
                agent: target.agent,
                skills_dir: self.store_path(&target.skills_dir)?,
                mcp_path: self.store_path(&target.mcp_path)?,
            };
            // Existing captured user paths remain authoritative if host environment changes.
            if !candidate
                .targets
                .iter()
                .any(|existing| existing.agent == saved.agent)
            {
                candidate.targets.push(saved);
            }
        }
        candidate.validate_targets()?;
        let expected = candidate.expected_artifacts()?;
        for artifact in expected {
            if !candidate
                .artifacts
                .iter()
                .any(|existing| existing.path == artifact.path)
            {
                candidate.artifacts.push(artifact);
            }
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn prepare(&self, bundle: &Bundle, filter: &[Agent]) -> Result<PreparedInstall> {
        self.validate()?;
        ensure!(
            bundle.revision.repository == REPOSITORY,
            "Only official PortOne sources can be installed"
        );
        ensure!(
            valid_hex(&bundle.revision.commit, 40),
            "Invalid source commit"
        );
        ensure!(
            bundle.skills.len() == SKILL_NAMES.len(),
            "Expected four official PortOne skills"
        );
        for name in SKILL_NAMES {
            let skill = bundle
                .skills
                .get(name)
                .with_context(|| format!("Missing official skill {name}"))?;
            ensure!(
                skill.files.contains_key(Path::new("SKILL.md")),
                "Missing SKILL.md for {name}"
            );
            for path in skill.files.keys() {
                validate_relative(path)?;
            }
        }
        let configured = self.agents();
        let mut agents = Vec::new();
        for agent in if filter.is_empty() {
            &configured
        } else {
            filter
        } {
            if configured.contains(agent) && !agents.contains(agent) {
                agents.push(*agent);
            }
        }
        let mut changes = Vec::new();
        for (index, artifact) in self.artifacts.iter().enumerate() {
            let owners: Vec<_> = self
                .targets
                .iter()
                .filter(|target| owns(target, artifact))
                .map(|target| target.agent)
                .collect();
            if !owners.iter().any(|owner| agents.contains(owner)) {
                continue;
            }
            let path = self.resolve(&artifact.path);
            check_path(&path)?;
            let content = match &artifact.kind {
                ArtifactKind::Skill { name } => {
                    Content::Directory(bundle.skills[name].files.clone())
                }
                ArtifactKind::Mcp => {
                    let existing = read_optional(&path)?;
                    let existing = existing
                        .as_deref()
                        .map(std::str::from_utf8)
                        .transpose()
                        .with_context(|| {
                            format!("MCP configuration is not UTF-8: {}", path.display())
                        })?;
                    Content::File(
                        render_mcp(existing, &owners, &bundle.mcp, cfg!(windows))
                            .with_context(|| {
                                format!("Cannot update MCP configuration {}", path.display())
                            })?
                            .into_bytes(),
                    )
                }
            };
            let changed = match &content {
                Content::File(bytes) => read_optional(&path)?.as_ref() != Some(bytes),
                Content::Directory(files) => read_tree(&path)?
                    .is_none_or(|tree| tree.has_empty_directories || &tree.files != files),
            };
            let content_hash = hash_content(&content);
            changes.push(Change {
                index,
                path,
                content,
                changed,
                success: LastSuccess {
                    commit: bundle.revision.commit.clone(),
                    content_hash,
                },
                agents: owners,
            });
        }
        changes.sort_by_key(|change| {
            agents
                .iter()
                .position(|agent| change.agents.contains(agent))
        });
        Ok(PreparedInstall {
            receipt: self.clone(),
            changes,
            agents,
        })
    }

    fn store_path(&self, path: &Path) -> Result<PathBuf> {
        ensure!(
            path.is_absolute(),
            "Installation destination must be absolute: {}",
            path.display()
        );
        let path = match self.scope {
            Scope::Project => path
                .strip_prefix(&self.root)
                .context("Project destination is outside project root")?
                .to_path_buf(),
            Scope::User => path.to_path_buf(),
        };
        self.validate_path(&path)?;
        Ok(path)
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        match self.scope {
            Scope::Project => self.root.join(path),
            Scope::User => path.to_path_buf(),
        }
    }

    fn validate_path(&self, path: &Path) -> Result<()> {
        match self.scope {
            Scope::Project => validate_relative(path),
            Scope::User => {
                ensure!(path.is_absolute(), "User destination must be absolute");
                ensure!(
                    !path
                        .components()
                        .any(|part| matches!(part, Component::ParentDir | Component::CurDir)),
                    "Unsafe user destination"
                );
                Ok(())
            }
        }
    }

    fn validate_targets(&self) -> Result<()> {
        ensure!(self.root.is_absolute(), "Setup root must be absolute");
        let mut agents = BTreeSet::new();
        for target in &self.targets {
            ensure!(
                agents.insert(target.agent),
                "Duplicate assistant in setup receipt"
            );
            self.validate_path(&target.skills_dir)?;
            self.validate_path(&target.mcp_path)?;
            validate_destination(target, self.scope)?;
        }
        Ok(())
    }

    fn expected_artifacts(&self) -> Result<Vec<Artifact>> {
        let mut artifacts = Vec::<Artifact>::new();
        for target in &self.targets {
            for (path, kind) in SKILL_NAMES
                .into_iter()
                .map(|name| {
                    (
                        target.skills_dir.join(name),
                        ArtifactKind::Skill { name: name.into() },
                    )
                })
                .chain(std::iter::once((
                    target.mcp_path.clone(),
                    ArtifactKind::Mcp,
                )))
            {
                if let Some(previous) = artifacts.iter().find(|artifact| artifact.path == path) {
                    ensure!(previous.kind == kind, "Conflicting artifact destinations");
                } else {
                    artifacts.push(Artifact {
                        path,
                        kind,
                        last_success: None,
                    });
                }
            }
        }
        for artifact in &artifacts {
            let path = &artifact.path;
            ensure!(
                !artifacts
                    .iter()
                    .map(|artifact| &artifact.path)
                    .any(|other| path != other && other.starts_with(path)),
                "Overlapping artifact destinations: {}",
                path.display()
            );
        }
        Ok(artifacts)
    }

    fn validate(&self) -> Result<()> {
        self.validate_targets()?;
        let expected = self.expected_artifacts()?;
        ensure!(
            expected.len() == self.artifacts.len(),
            "Receipt artifact ownership mismatch"
        );
        let mut paths = BTreeSet::new();
        for artifact in &self.artifacts {
            ensure!(paths.insert(&artifact.path), "Duplicate receipt artifact");
            ensure!(
                expected
                    .iter()
                    .any(|item| item.path == artifact.path && item.kind == artifact.kind),
                "Receipt artifact is not owned by a configured assistant"
            );
            if let Some(success) = &artifact.last_success {
                ensure!(
                    valid_hex(&success.commit, 40) && valid_hex(&success.content_hash, 64),
                    "Invalid artifact success state"
                );
            }
        }
        Ok(())
    }

    fn save(&self, path: &Path) -> Result<()> {
        check_path(path)?;
        let parent = path.parent().context("Receipt path has no parent")?;
        fs::create_dir_all(parent)?;
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        if let Ok(metadata) = fs::metadata(path) {
            staged.as_file().set_permissions(metadata.permissions())?;
        }
        serde_json::to_writer_pretty(staged.as_file_mut(), self)?;
        staged.write_all(b"\n")?;
        staged.as_file().sync_all()?;
        staged
            .persist(path)
            .map_err(|error| error.error)
            .with_context(|| format!("Cannot save setup receipt {}", path.display()))?;
        Ok(())
    }
}

impl PreparedInstall {
    pub fn apply(mut self, receipt_path: &Path, dry_run: bool) -> InstallReport {
        let mut report = InstallReport::default();
        if !dry_run && let Err(error) = self.receipt.save(receipt_path) {
            report.errors.push(format!("{error:#}"));
            return report;
        }
        let mut failed_agents = BTreeSet::<Agent>::new();
        for change in &self.changes {
            let state = if dry_run {
                if change.changed {
                    InstallState::WouldUpdate
                } else {
                    InstallState::Unchanged
                }
            } else {
                let previous = self.receipt.artifacts[change.index].last_success.clone();
                let mut replacement = match change
                    .changed
                    .then(|| Replacement::install(&change.path, &change.content))
                    .transpose()
                {
                    Ok(replacement) => replacement,
                    Err(error) => {
                        report
                            .errors
                            .push(format!("{}: {error:#}", change.path.display()));
                        failed_agents.extend(change.agents.iter().copied());
                        report.items.push(InstallItem {
                            path: change.path.clone(),
                            state: InstallState::Failed,
                        });
                        continue;
                    }
                };
                self.receipt.artifacts[change.index].last_success = Some(change.success.clone());
                if let Err(error) = self.receipt.save(receipt_path) {
                    self.receipt.artifacts[change.index].last_success = previous;
                    report.errors.push(format!("{error:#}"));
                    if let Some(replacement) = &mut replacement
                        && let Err(error) = replacement.rollback()
                    {
                        report.errors.push(format!("Rollback failed: {error:#}"));
                    }
                    report.items.push(InstallItem {
                        path: change.path.clone(),
                        state: InstallState::Failed,
                    });
                    // A receipt failure stops this scope: later artifacts cannot be tracked safely.
                    return report;
                }
                if change.changed {
                    InstallState::Updated
                } else {
                    InstallState::Unchanged
                }
            };
            report.items.push(InstallItem {
                path: change.path.clone(),
                state,
            });
        }
        report.successful_agents = self
            .agents
            .into_iter()
            .filter(|agent| !failed_agents.contains(agent))
            .collect();
        report
    }
}

fn owns(target: &Destination, artifact: &Artifact) -> bool {
    match &artifact.kind {
        ArtifactKind::Mcp => target.mcp_path == artifact.path,
        ArtifactKind::Skill { name } => target.skills_dir.join(name) == artifact.path,
    }
}

fn valid_hex(text: &str, length: usize) -> bool {
    text.len() == length && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_relative(path: &Path) -> Result<()> {
    ensure!(
        !path.as_os_str().is_empty()
            && path
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
        "Unsafe relative path: {}",
        path.display()
    );
    ensure!(
        !path.to_string_lossy().contains('\\'),
        "Unsafe path separator"
    );
    Ok(())
}

fn check_path(path: &Path) -> Result<()> {
    ensure!(
        !path
            .components()
            .any(|part| matches!(part, Component::ParentDir)),
        "Path traversal is not allowed"
    );
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink() || is_system_alias(ancestor),
                "Refusing symlink: {}",
                ancestor.display()
            ),
            Err(error)
                if error.kind() == ErrorKind::NotFound
                    || error.kind() == ErrorKind::NotADirectory => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Cannot inspect {}", ancestor.display()));
            }
        }
    }
    Ok(())
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("Cannot read {}", path.display())),
    }
}

struct ExistingTree {
    files: BTreeMap<PathBuf, Vec<u8>>,
    has_empty_directories: bool,
}

fn read_tree(path: &Path) -> Result<Option<ExistingTree>> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Cannot read skill directory {}", path.display()));
        }
    };
    let mut files = BTreeMap::new();
    let mut has_empty_directories = false;
    for entry in entries {
        let entry = entry?;
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "Refusing symlink inside managed skill: {}",
            entry.path().display()
        );
        if kind.is_dir() {
            let tree = read_tree(&entry.path())?.context("Skill directory disappeared")?;
            has_empty_directories |= tree.files.is_empty() || tree.has_empty_directories;
            for (child, bytes) in tree.files {
                files.insert(PathBuf::from(entry.file_name()).join(child), bytes);
            }
        } else {
            ensure!(kind.is_file(), "Unsupported managed skill entry");
            files.insert(PathBuf::from(entry.file_name()), fs::read(entry.path())?);
        }
    }
    Ok(Some(ExistingTree {
        files,
        has_empty_directories,
    }))
}

fn hash_content(content: &Content) -> String {
    let mut hash = Sha256::new();
    match content {
        Content::File(bytes) => {
            hash.update(b"file\0");
            hash.update(bytes);
        }
        Content::Directory(files) => {
            hash.update(b"directory\0");
            for (path, bytes) in files {
                let path = path.to_string_lossy();
                hash.update((path.len() as u64).to_be_bytes());
                hash.update(path.as_bytes());
                hash.update((bytes.len() as u64).to_be_bytes());
                hash.update(bytes);
            }
        }
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_system_alias(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        let expected = match path.to_str() {
            Some("/var") => Path::new("private/var"),
            Some("/tmp") => Path::new("private/tmp"),
            _ => return false,
        };
        fs::read_link(path).is_ok_and(|target| target == expected)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        false
    }
}

struct Replacement {
    path: PathBuf,
    backup: PathBuf,
    staging: Option<tempfile::TempDir>,
    had_previous: bool,
    atomic_file: bool,
}

impl Replacement {
    fn install(path: &Path, content: &Content) -> Result<Self> {
        check_path(path)?;
        let parent = path.parent().context("Artifact has no parent")?;
        fs::create_dir_all(parent)?;
        let staging = tempfile::tempdir_in(parent)?;
        let staged = staging.path().join("new");
        match content {
            Content::File(bytes) => fs::write(&staged, bytes)?,
            Content::Directory(files) => {
                fs::create_dir(&staged)?;
                for (relative, bytes) in files {
                    validate_relative(relative)?;
                    let file = staged.join(relative);
                    fs::create_dir_all(file.parent().context("Skill file has no parent")?)?;
                    fs::write(file, bytes)?;
                }
            }
        }
        let metadata = fs::symlink_metadata(path).ok();
        if let Some(metadata) = &metadata {
            fs::set_permissions(&staged, metadata.permissions())?;
        }
        check_path(path)?;
        let backup = staging.path().join("previous");
        let had_previous = metadata.is_some();
        let atomic_file = matches!(content, Content::File(_))
            && metadata.as_ref().is_none_or(|metadata| metadata.is_file());
        if had_previous {
            if atomic_file {
                fs::copy(path, &backup)?;
            } else {
                fs::rename(path, &backup)?;
            }
        }
        let mut replacement = Self {
            path: path.to_path_buf(),
            backup,
            staging: Some(staging),
            had_previous,
            atomic_file,
        };
        if let Err(error) = fs::rename(staged, path) {
            if had_previous
                && !atomic_file
                && let Err(rollback) = fs::rename(&replacement.backup, path)
            {
                let retained = replacement.staging.take().expect("staging present").keep();
                bail!(
                    "Replacement failed: {error}; rollback failed: {rollback}; backup retained at {}",
                    retained.display()
                );
            }
            return Err(error.into());
        }
        Ok(replacement)
    }

    fn rollback(&mut self) -> Result<()> {
        let staging = self
            .staging
            .as_ref()
            .context("Missing replacement backup")?;
        let failed = staging.path().join("failed");
        let result = (|| -> Result<()> {
            if self.atomic_file {
                if self.had_previous {
                    fs::rename(&self.backup, &self.path)?;
                } else {
                    fs::remove_file(&self.path)?;
                }
            } else {
                fs::rename(&self.path, failed)?;
                if self.had_previous {
                    fs::rename(&self.backup, &self.path)?;
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            let retained = self.staging.take().expect("staging present").keep();
            bail!("{error}; backup retained at {}", retained.display());
        }
        Ok(())
    }
}
