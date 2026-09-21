mod claude_code;
mod codex;
mod cursor;
mod gemini_cli;
mod github_copilot;
mod json;
mod opencode;
mod paths;
mod requirements;
mod vscode_copilot;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail, ensure};

use super::model::{Agent, Destination, McpServer, Scope};
use requirements::{McpFormat, McpRequirements, SharedMcpGroup};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum HostPlatform {
    Linux,
    Macos,
    Windows,
}

impl HostPlatform {
    fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Self::Windows;
        #[cfg(target_os = "macos")]
        return Self::Macos;
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        return Self::Linux;
    }
}

struct DestinationContext<'a> {
    project: &'a Path,
    home: &'a Path,
    env: &'a BTreeMap<String, String>,
    platform: HostPlatform,
}

trait AgentAdapter {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)>;

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()>;

    fn mcp_requirements(&self) -> McpRequirements;

    // Adapters in a sharing group must honor all merged requirements, not just their own.
    fn render(
        &self,
        existing: Option<&str>,
        server: &McpServer,
        windows: bool,
        requirements: &McpRequirements,
    ) -> Result<String>;
}

fn adapter(agent: Agent) -> &'static dyn AgentAdapter {
    match agent {
        Agent::ClaudeCode => &claude_code::ClaudeCode,
        Agent::Codex => &codex::Codex,
        Agent::Cursor => &cursor::Cursor,
        Agent::GeminiCli => &gemini_cli::GeminiCli,
        Agent::GithubCopilot => &github_copilot::GithubCopilot,
        Agent::VscodeCopilot => &vscode_copilot::VscodeCopilot,
        Agent::Opencode => &opencode::Opencode,
    }
}

pub fn resolve_destination(
    agent: Agent,
    scope: Scope,
    project: &Path,
    home: &Path,
    env: &BTreeMap<String, String>,
) -> Result<Destination> {
    resolve_destination_for_platform(agent, scope, project, home, env, HostPlatform::current())
}

pub(crate) fn resolve_destination_for_platform(
    agent: Agent,
    scope: Scope,
    project: &Path,
    home: &Path,
    env: &BTreeMap<String, String>,
    platform: HostPlatform,
) -> Result<Destination> {
    let context = DestinationContext {
        project,
        home,
        env,
        platform,
    };
    let (skills_dir, mcp_path) = adapter(agent).resolve_destination(scope, &context)?;
    Ok(Destination {
        agent,
        skills_dir,
        mcp_path,
    })
}

pub(super) fn validate_destination(target: &Destination, scope: Scope) -> Result<()> {
    adapter(target.agent).validate_destination(target, scope)
}

pub fn render_mcp(
    existing: Option<&str>,
    agents: &[Agent],
    server: &McpServer,
    windows: bool,
) -> Result<String> {
    if agents.is_empty() {
        bail!("at least one agent is required to render MCP configuration");
    }
    let mut owners = agents.to_vec();
    owners.sort_unstable();
    owners.dedup();
    ensure!(
        owners.len() == agents.len(),
        "agents do not share an MCP configuration format and destination"
    );

    let renderer = adapter(owners[0]);
    let mut requirements = renderer.mcp_requirements();
    for owner in &owners[1..] {
        requirements = requirements.merge(adapter(*owner).mcp_requirements())?;
    }
    // Render once with a stable representative after merging every owner's requirements.
    renderer.render(existing, server, windows, &requirements)
}

fn normalized_command(server: &McpServer, windows: bool) -> (String, Vec<String>) {
    if windows && server.command.eq_ignore_ascii_case("npx") {
        let mut args = vec!["/c".to_string(), server.command.clone()];
        args.extend(server.args.iter().cloned());
        ("cmd".to_string(), args)
    } else {
        (server.command.clone(), server.args.clone())
    }
}
