use std::path::PathBuf;

use anyhow::Result;

use super::json::render_generic;
use super::paths::{env_path, validate_destination};
use super::{
    AgentAdapter, Destination, DestinationContext, McpFormat, McpRequirements, McpServer, Scope,
};

pub(super) struct GeminiCli;

impl AgentAdapter for GeminiCli {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)> {
        if scope == Scope::Project {
            return Ok((
                context.project.join(".agents/skills"),
                context.project.join(".gemini/settings.json"),
            ));
        }
        let root = env_path(context.env, "GEMINI_CLI_HOME", context.platform)?
            .unwrap_or_else(|| context.home.to_path_buf());
        let config = root.join(".gemini");
        Ok((config.join("skills"), config.join("settings.json")))
    }

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()> {
        validate_destination(
            target,
            scope,
            ".agents/skills",
            &[".gemini/settings.json"],
            &["settings.json"],
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
