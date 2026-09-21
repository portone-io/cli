use std::path::PathBuf;

use anyhow::Result;

use super::json::render_generic;
use super::paths::{env_path, validate_destination};
use super::{
    AgentAdapter, Destination, DestinationContext, McpFormat, McpRequirements, McpServer, Scope,
    SharedMcpGroup,
};

pub(super) struct GithubCopilot;

impl AgentAdapter for GithubCopilot {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)> {
        if scope == Scope::Project {
            return Ok((
                context.project.join(".agents/skills"),
                context.project.join(".mcp.json"),
            ));
        }
        let config = env_path(context.env, "COPILOT_HOME", context.platform)?
            .unwrap_or_else(|| context.home.join(".copilot"));
        Ok((config.join("skills"), config.join("mcp-config.json")))
    }

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()> {
        validate_destination(
            target,
            scope,
            ".agents/skills",
            &[".mcp.json"],
            &["mcp-config.json"],
        )
    }

    fn mcp_requirements(&self) -> McpRequirements {
        McpRequirements {
            format: McpFormat::McpServersJson {
                include_tools: true,
                normalize_bare_map: true,
            },
            shared_group: Some(SharedMcpGroup::ClaudeCopilot),
        }
    }

    fn render(
        &self,
        existing: Option<&str>,
        server: &McpServer,
        windows: bool,
        requirements: &McpRequirements,
    ) -> Result<String> {
        render_generic(existing, server, windows, requirements.format)
    }
}
