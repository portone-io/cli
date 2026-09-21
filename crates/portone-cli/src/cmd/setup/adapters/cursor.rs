use std::path::PathBuf;

use anyhow::Result;

use super::json::render_generic;
use super::paths::validate_destination;
use super::{
    AgentAdapter, Destination, DestinationContext, McpFormat, McpRequirements, McpServer, Scope,
};

pub(super) struct Cursor;

impl AgentAdapter for Cursor {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)> {
        Ok(match scope {
            Scope::Project => (
                context.project.join(".agents/skills"),
                context.project.join(".cursor/mcp.json"),
            ),
            Scope::User => (
                context.home.join(".cursor/skills"),
                context.home.join(".cursor/mcp.json"),
            ),
        })
    }

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()> {
        validate_destination(
            target,
            scope,
            ".agents/skills",
            &[".cursor/mcp.json"],
            &["mcp.json"],
        )
    }

    fn mcp_requirements(&self) -> McpRequirements {
        McpRequirements {
            format: McpFormat::McpServersJson {
                include_tools: false,
                normalize_bare_map: false,
            },
            shared_group: None,
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
