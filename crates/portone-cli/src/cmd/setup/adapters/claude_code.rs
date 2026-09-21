use std::path::PathBuf;

use anyhow::Result;

use super::json::render_generic;
use super::paths::{env_path, validate_destination};
use super::{
    AgentAdapter, Destination, DestinationContext, McpFormat, McpRequirements, McpServer, Scope,
    SharedMcpGroup,
};

pub(super) struct ClaudeCode;

impl AgentAdapter for ClaudeCode {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)> {
        if scope == Scope::Project {
            return Ok((
                context.project.join(".claude/skills"),
                context.project.join(".mcp.json"),
            ));
        }
        let override_path = env_path(context.env, "CLAUDE_CONFIG_DIR", context.platform)?;
        let config = override_path
            .clone()
            .unwrap_or_else(|| context.home.join(".claude"));
        let mcp = if override_path.is_some() {
            config.join(".claude.json")
        } else {
            context.home.join(".claude.json")
        };
        Ok((config.join("skills"), mcp))
    }

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()> {
        validate_destination(
            target,
            scope,
            ".claude/skills",
            &[".mcp.json"],
            &[".claude.json"],
        )
    }

    fn mcp_requirements(&self) -> McpRequirements {
        McpRequirements {
            format: McpFormat::McpServersJson {
                include_tools: false,
                normalize_bare_map: false,
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
