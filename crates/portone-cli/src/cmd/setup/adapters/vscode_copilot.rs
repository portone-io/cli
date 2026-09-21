use std::path::PathBuf;

use anyhow::Result;

use super::json::{input_array, render_json_container_with_fields};
use super::paths::{env_path, validate_destination};
use super::{
    AgentAdapter, Destination, DestinationContext, HostPlatform, McpFormat, McpRequirements,
    McpServer, Scope, normalized_command,
};

pub(super) struct VscodeCopilot;

impl AgentAdapter for VscodeCopilot {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)> {
        if scope == Scope::Project {
            return Ok((
                context.project.join(".agents/skills"),
                context.project.join(".vscode/mcp.json"),
            ));
        }
        let mcp = match context.platform {
            HostPlatform::Linux => env_path(context.env, "XDG_CONFIG_HOME", context.platform)?
                .unwrap_or_else(|| context.home.join(".config"))
                .join("Code/User/mcp.json"),
            HostPlatform::Macos => context
                .home
                .join("Library/Application Support/Code/User/mcp.json"),
            HostPlatform::Windows => env_path(context.env, "APPDATA", context.platform)?
                .unwrap_or_else(|| context.home.join("AppData/Roaming"))
                .join("Code/User/mcp.json"),
        };
        Ok((context.home.join(".copilot/skills"), mcp))
    }

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()> {
        validate_destination(
            target,
            scope,
            ".agents/skills",
            &[".vscode/mcp.json"],
            &["mcp.json"],
        )
    }

    fn mcp_requirements(&self) -> McpRequirements {
        McpRequirements {
            format: McpFormat::VscodeJson,
            shared_group: None,
        }
    }

    fn render(
        &self,
        existing: Option<&str>,
        server: &McpServer,
        windows: bool,
        _requirements: &McpRequirements,
    ) -> Result<String> {
        let (command, args) = normalized_command(server, windows);
        let fields = vec![
            ("type".into(), "stdio".into()),
            ("command".into(), command.into()),
            ("args".into(), input_array(args)),
        ];
        render_json_container_with_fields(existing, "servers", fields, Some(("env", &server.env)))
    }
}
