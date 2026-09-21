use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail, ensure};

use super::{Destination, HostPlatform, Scope};

pub(super) fn env_path(
    env: &BTreeMap<String, String>,
    name: &str,
    platform: HostPlatform,
) -> Result<Option<PathBuf>> {
    let Some(value) = env.get(name).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !is_absolute_for_platform(value, platform) {
        bail!("environment variable {name} must contain an absolute path");
    }
    Ok(Some(PathBuf::from(value)))
}

fn is_absolute_for_platform(value: &str, platform: HostPlatform) -> bool {
    if Path::new(value).is_absolute() {
        return true;
    }
    if platform != HostPlatform::Windows {
        return false;
    }
    let bytes = value.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\'))
        || value.starts_with("\\\\")
        || value.starts_with("//")
}

pub(super) fn first_existing_or_default<const N: usize>(
    candidates: [PathBuf; N],
    default: PathBuf,
) -> PathBuf {
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or(default)
}

// Validate captured paths without resolving them against the current environment.
pub(super) fn validate_destination(
    target: &Destination,
    scope: Scope,
    project_skills: &str,
    project_mcp: &[&str],
    user_mcp: &[&str],
) -> Result<()> {
    if scope == Scope::Project {
        ensure!(
            target.skills_dir == Path::new(project_skills),
            "Unexpected project skill destination"
        );
        ensure!(
            project_mcp
                .iter()
                .any(|path| target.mcp_path == Path::new(path)),
            "Unexpected project MCP destination"
        );
    } else {
        ensure!(
            target.skills_dir.ends_with("skills"),
            "Unexpected user skill destination"
        );
        ensure!(
            target
                .mcp_path
                .file_name()
                .is_some_and(|name| user_mcp.iter().any(|file| name == *file)),
            "Unexpected user MCP destination"
        );
    }
    Ok(())
}
