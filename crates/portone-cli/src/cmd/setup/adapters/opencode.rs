use std::path::PathBuf;

use anyhow::{Context, Result};
use jsonc_parser::cst::CstInputValue;

use super::json::{input_array, object_container, parse_json, update_server_object};
use super::paths::{env_path, first_existing_or_default, validate_destination};
use super::{
    AgentAdapter, Destination, DestinationContext, McpFormat, McpRequirements, McpServer, Scope,
    normalized_command,
};

pub(super) struct Opencode;

impl AgentAdapter for Opencode {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)> {
        if scope == Scope::Project {
            let mcp = first_existing_or_default(
                [
                    context.project.join("opencode.jsonc"),
                    context.project.join("opencode.json"),
                ],
                context.project.join("opencode.json"),
            );
            return Ok((context.project.join(".agents/skills"), mcp));
        }
        let config_home = env_path(context.env, "XDG_CONFIG_HOME", context.platform)?
            .unwrap_or_else(|| context.home.join(".config"));
        let standard = config_home.join("opencode");
        let mcp = first_existing_or_default(
            [
                standard.join("opencode.jsonc"),
                standard.join("opencode.json"),
                standard.join("config.json"),
            ],
            standard.join("opencode.json"),
        );
        Ok((standard.join("skills"), mcp))
    }

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()> {
        validate_destination(
            target,
            scope,
            ".agents/skills",
            &["opencode.jsonc", "opencode.json"],
            &["opencode.jsonc", "opencode.json", "config.json"],
        )
    }

    fn mcp_requirements(&self) -> McpRequirements {
        McpRequirements {
            format: McpFormat::OpencodeJson,
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
        let root = parse_json(existing.unwrap_or("{}\n"))?;
        let object = root
            .object_value()
            .context("OpenCode JSON root must be an object")?;
        let mcp = object_container(&object, "mcp")?;
        let native_servers = match mcp.get("servers") {
            Some(property) => Some(
                property
                    .object_value()
                    .context("OpenCode `mcp.servers` must be an object")?,
            ),
            None => None,
        };
        let has_v2 = mcp.get("portone").is_some();

        if native_servers.is_none() || has_v2 {
            let fields = opencode_v2_fields(server, windows);
            update_server_object(&mcp, "portone", fields, Some(("environment", &server.env)));
        }
        if let Some(native_servers) = native_servers {
            let fields = opencode_native_fields(server, windows);
            update_server_object(
                &native_servers,
                "portone",
                fields,
                Some(("env", &server.env)),
            );
        }
        Ok(root.to_string())
    }
}

fn opencode_v2_fields(server: &McpServer, windows: bool) -> Vec<(String, CstInputValue)> {
    let (command, args) = normalized_command(server, windows);
    let mut command_and_args = vec![command];
    command_and_args.extend(args);
    vec![
        ("type".into(), "local".into()),
        ("command".into(), input_array(command_and_args)),
        ("enabled".into(), true.into()),
    ]
}

fn opencode_native_fields(server: &McpServer, windows: bool) -> Vec<(String, CstInputValue)> {
    let (command, args) = normalized_command(server, windows);
    vec![
        ("command".into(), command.into()),
        ("args".into(), input_array(args)),
        ("disabled".into(), false.into()),
    ]
}
