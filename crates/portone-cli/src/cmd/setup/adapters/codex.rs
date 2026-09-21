use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use toml_edit::{Array, DocumentMut, Item, Table, value};

use super::paths::{env_path, validate_destination};
use super::{
    AgentAdapter, Destination, DestinationContext, McpFormat, McpRequirements, McpServer, Scope,
    normalized_command,
};

pub(super) struct Codex;

impl AgentAdapter for Codex {
    fn resolve_destination(
        &self,
        scope: Scope,
        context: &DestinationContext<'_>,
    ) -> Result<(PathBuf, PathBuf)> {
        if scope == Scope::Project {
            return Ok((
                context.project.join(".agents/skills"),
                context.project.join(".codex/config.toml"),
            ));
        }
        let config = env_path(context.env, "CODEX_HOME", context.platform)?
            .unwrap_or_else(|| context.home.join(".codex"));
        Ok((
            context.home.join(".agents/skills"),
            config.join("config.toml"),
        ))
    }

    fn validate_destination(&self, target: &Destination, scope: Scope) -> Result<()> {
        validate_destination(
            target,
            scope,
            ".agents/skills",
            &[".codex/config.toml"],
            &["config.toml"],
        )
    }

    fn mcp_requirements(&self) -> McpRequirements {
        McpRequirements {
            format: McpFormat::CodexToml,
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
        let mut document = existing
            .unwrap_or("")
            .parse::<DocumentMut>()
            .context("failed to parse Codex MCP TOML")?;
        if let Some(item) = document.as_table().get("mcp_servers")
            && !item.is_table_like()
        {
            bail!("Codex `mcp_servers` must be a table");
        }
        if !document.as_table().contains_key("mcp_servers") {
            document["mcp_servers"] = Item::Table(Table::new());
        }

        let (command, args) = normalized_command(server, windows);
        let mcp_servers = document["mcp_servers"]
            .as_table_like_mut()
            .expect("validated as table");
        if !mcp_servers.get("portone").is_some_and(Item::is_table_like) {
            mcp_servers.insert("portone", Item::Table(Table::new()));
        }
        let portone = mcp_servers
            .get_mut("portone")
            .and_then(Item::as_table_like_mut)
            .expect("created as table");
        for field in [
            "url",
            "http_url",
            "http_headers",
            "env_http_headers",
            "bearer_token_env_var",
            "transport",
            "type",
        ] {
            portone.remove(field);
        }
        if portone.contains_key("enabled") {
            portone.insert("enabled", value(true));
        }
        if portone.contains_key("disabled") {
            portone.insert("disabled", value(false));
        }
        portone.insert("command", value(command));
        let mut arg_array = Array::new();
        for arg in args {
            arg_array.push(arg);
        }
        portone.insert("args", value(arg_array));
        if !portone.get("env").is_some_and(Item::is_table_like) {
            portone.insert("env", Item::Table(Table::new()));
        }
        let environment = portone
            .get_mut("env")
            .and_then(Item::as_table_like_mut)
            .expect("created as table");
        for (key, env_value) in &server.env {
            environment.insert(key, value(env_value.clone()));
        }
        Ok(document.to_string())
    }
}
