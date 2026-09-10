use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use jsonc_parser::{JsonValue, ParseOptions, json, parse_to_value};
use toml_edit::{Array, DocumentMut, Item, Table, value};

use super::model::{Agent, Destination, McpServer, Scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Non-host variants are exercised by the platform path tests.
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
    let (skills_dir, mcp_path) = match scope {
        Scope::Project => project_destination(agent, project),
        Scope::User => user_destination(agent, home, env, platform)?,
    };
    Ok(Destination {
        agent,
        skills_dir,
        mcp_path,
    })
}

fn project_destination(agent: Agent, project: &Path) -> (PathBuf, PathBuf) {
    let shared_skills = project.join(".agents/skills");
    match agent {
        Agent::ClaudeCode => (project.join(".claude/skills"), project.join(".mcp.json")),
        Agent::Codex => (shared_skills, project.join(".codex/config.toml")),
        Agent::Cursor => (shared_skills, project.join(".cursor/mcp.json")),
        Agent::GeminiCli => (shared_skills, project.join(".gemini/settings.json")),
        Agent::GithubCopilot => (shared_skills, project.join(".mcp.json")),
        Agent::VscodeCopilot => (shared_skills, project.join(".vscode/mcp.json")),
        Agent::Opencode => (
            shared_skills,
            first_existing_or_default(
                [
                    project.join("opencode.jsonc"),
                    project.join("opencode.json"),
                ],
                project.join("opencode.json"),
            ),
        ),
    }
}

fn user_destination(
    agent: Agent,
    home: &Path,
    env: &BTreeMap<String, String>,
    platform: HostPlatform,
) -> Result<(PathBuf, PathBuf)> {
    let destination = match agent {
        Agent::ClaudeCode => {
            let override_path = env_path(env, "CLAUDE_CONFIG_DIR", platform)?;
            let config = override_path
                .clone()
                .unwrap_or_else(|| home.join(".claude"));
            let mcp = if override_path.is_some() {
                config.join(".claude.json")
            } else {
                home.join(".claude.json")
            };
            (config.join("skills"), mcp)
        }
        Agent::Codex => {
            let config =
                env_path(env, "CODEX_HOME", platform)?.unwrap_or_else(|| home.join(".codex"));
            (home.join(".agents/skills"), config.join("config.toml"))
        }
        Agent::Cursor => (home.join(".cursor/skills"), home.join(".cursor/mcp.json")),
        Agent::GeminiCli => {
            let root =
                env_path(env, "GEMINI_CLI_HOME", platform)?.unwrap_or_else(|| home.to_path_buf());
            let config = root.join(".gemini");
            (config.join("skills"), config.join("settings.json"))
        }
        Agent::GithubCopilot => {
            let config =
                env_path(env, "COPILOT_HOME", platform)?.unwrap_or_else(|| home.join(".copilot"));
            (config.join("skills"), config.join("mcp-config.json"))
        }
        Agent::VscodeCopilot => (
            home.join(".copilot/skills"),
            vscode_user_mcp_path(home, env, platform)?,
        ),
        Agent::Opencode => {
            let config_home =
                env_path(env, "XDG_CONFIG_HOME", platform)?.unwrap_or_else(|| home.join(".config"));
            let standard = config_home.join("opencode");
            let mcp = first_existing_or_default(
                [
                    standard.join("opencode.jsonc"),
                    standard.join("opencode.json"),
                    standard.join("config.json"),
                ],
                standard.join("opencode.json"),
            );
            (standard.join("skills"), mcp)
        }
    };
    Ok(destination)
}

fn env_path(
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

fn first_existing_or_default<const N: usize>(
    candidates: [PathBuf; N],
    default: PathBuf,
) -> PathBuf {
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or(default)
}

fn vscode_user_mcp_path(
    home: &Path,
    env: &BTreeMap<String, String>,
    platform: HostPlatform,
) -> Result<PathBuf> {
    match platform {
        HostPlatform::Linux => Ok(env_path(env, "XDG_CONFIG_HOME", platform)?
            .unwrap_or_else(|| home.join(".config"))
            .join("Code/User/mcp.json")),
        HostPlatform::Macos => Ok(home.join("Library/Application Support/Code/User/mcp.json")),
        HostPlatform::Windows => Ok(env_path(env, "APPDATA", platform)?
            .unwrap_or_else(|| home.join("AppData/Roaming"))
            .join("Code/User/mcp.json")),
    }
}

pub fn render_mcp(
    existing: Option<&str>,
    agents: &[Agent],
    server: &McpServer,
    windows: bool,
) -> Result<String> {
    validate_agent_group(agents)?;
    match agents[0] {
        Agent::Codex => render_codex(existing, server, windows),
        Agent::VscodeCopilot => {
            render_json_container(existing, "servers", server, windows, JsonFlavor::Vscode)
        }
        Agent::Opencode => render_opencode(existing, server, windows),
        Agent::ClaudeCode | Agent::Cursor | Agent::GeminiCli | Agent::GithubCopilot => {
            render_generic(existing, agents, server, windows)
        }
    }
}

fn validate_agent_group(agents: &[Agent]) -> Result<()> {
    if agents.is_empty() {
        bail!("at least one agent is required to render MCP configuration");
    }
    let mut unique = agents.to_vec();
    unique.sort_unstable();
    unique.dedup();
    let shared = unique == [Agent::ClaudeCode, Agent::GithubCopilot];
    if unique.len() != agents.len() || (unique.len() > 1 && !shared) {
        bail!("agents do not share an MCP configuration format and destination");
    }
    Ok(())
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

fn input_array(values: impl IntoIterator<Item = String>) -> CstInputValue {
    CstInputValue::Array(values.into_iter().map(CstInputValue::String).collect())
}

fn generic_server_fields(
    server: &McpServer,
    windows: bool,
    include_tools: bool,
) -> Vec<(String, CstInputValue)> {
    let (command, args) = normalized_command(server, windows);
    let mut fields = vec![
        ("command".into(), command.into()),
        ("args".into(), input_array(args)),
    ];
    if include_tools {
        fields.push(("tools".into(), json!(["*"])));
    }
    fields
}

fn render_generic(
    existing: Option<&str>,
    agents: &[Agent],
    server: &McpServer,
    windows: bool,
) -> Result<String> {
    let include_tools = agents.contains(&Agent::GithubCopilot);
    let normalized;
    let existing = if let (true, Some(existing)) = (include_tools, existing) {
        normalized = normalize_copilot_bare_map(existing)?;
        Some(normalized.as_str())
    } else {
        existing
    };
    render_json_container_with_fields(
        existing,
        "mcpServers",
        generic_server_fields(server, windows, include_tools),
        Some(("env", &server.env)),
    )
}

fn normalize_copilot_bare_map(existing: &str) -> Result<String> {
    let root = parse_json(existing)?;
    let object = root
        .object_value()
        .context("MCP JSON root must be an object")?;
    if object.get("mcpServers").is_some() || object.properties().is_empty() {
        return Ok(existing.to_string());
    }
    let mut servers = Vec::new();
    for property in object.properties() {
        let Some(server) = property.object_value() else {
            continue;
        };
        if server.get("command").is_none()
            && server.get("url").is_none()
            && server.get("httpUrl").is_none()
        {
            continue;
        }
        let name = property
            .name()
            .context("bare MCP server property has no name")?
            .decoded_value()
            .context("bare MCP server property name is invalid")?;
        let value = property
            .value()
            .context("bare MCP server property has no value")?
            .to_string();
        servers.push((name, parse_input_value(&value)?));
        property.remove();
    }
    if servers.is_empty() {
        return Ok(existing.to_string());
    }
    object.append("mcpServers", CstInputValue::Object(servers));
    Ok(root.to_string())
}

fn parse_input_value(text: &str) -> Result<CstInputValue> {
    let value = parse_to_value(text, &ParseOptions::default())
        .context("failed to parse existing bare MCP server")?
        .context("existing bare MCP server has no value")?;
    Ok(json_value_to_input(value))
}

fn json_value_to_input(value: JsonValue<'_>) -> CstInputValue {
    match value {
        JsonValue::Null => CstInputValue::Null,
        JsonValue::Boolean(value) => value.into(),
        JsonValue::Number(value) => CstInputValue::Number(value.to_owned()),
        JsonValue::String(value) => value.into_owned().into(),
        JsonValue::Array(values) => {
            CstInputValue::Array(values.into_iter().map(json_value_to_input).collect())
        }
        JsonValue::Object(values) => CstInputValue::Object(
            values
                .into_iter()
                .map(|(name, value)| (name.into_owned(), json_value_to_input(value)))
                .collect(),
        ),
    }
}

#[derive(Debug, Clone, Copy)]
enum JsonFlavor {
    Vscode,
}

fn render_json_container(
    existing: Option<&str>,
    container_name: &str,
    server: &McpServer,
    windows: bool,
    flavor: JsonFlavor,
) -> Result<String> {
    let (command, args) = normalized_command(server, windows);
    let fields = match flavor {
        JsonFlavor::Vscode => vec![
            ("type".into(), "stdio".into()),
            ("command".into(), command.into()),
            ("args".into(), input_array(args)),
        ],
    };
    render_json_container_with_fields(existing, container_name, fields, Some(("env", &server.env)))
}

fn render_json_container_with_fields(
    existing: Option<&str>,
    container_name: &str,
    fields: Vec<(String, CstInputValue)>,
    environment: Option<(&str, &BTreeMap<String, String>)>,
) -> Result<String> {
    let root = parse_json(existing.unwrap_or("{}\n"))?;
    let object = root
        .object_value()
        .context("MCP JSON root must be an object")?;
    let servers = object_container(&object, container_name)?;
    update_server_object(&servers, "portone", fields, environment);
    Ok(root.to_string())
}

fn parse_json(text: &str) -> Result<CstRootNode> {
    CstRootNode::parse(text, &ParseOptions::default()).context("failed to parse MCP JSON/JSONC")
}

fn object_container(object: &CstObject, name: &str) -> Result<CstObject> {
    match object.get(name) {
        Some(property) => property
            .object_value()
            .with_context(|| format!("MCP container `{name}` must be an object")),
        None => Ok(object.object_value_or_set(name)),
    }
}

fn set_object_property(object: &CstObject, name: &str, value: CstInputValue) {
    match object.get(name) {
        Some(property) => property.set_value(value),
        None => {
            object.append(name, value);
        }
    }
}

fn object_property_or_set(object: &CstObject, name: &str) -> CstObject {
    match object.get(name) {
        Some(property) => property.object_value_or_set(),
        None => object.object_value_or_set(name),
    }
}

fn update_server_object(
    servers: &CstObject,
    name: &str,
    fields: Vec<(String, CstInputValue)>,
    environment: Option<(&str, &BTreeMap<String, String>)>,
) {
    let server = object_property_or_set(servers, name);
    for field in ["url", "httpUrl", "headers", "transport", "transportType"] {
        if let Some(property) = server.get(field) {
            property.remove();
        }
    }
    let writes_type = fields.iter().any(|(field, _)| field == "type");
    let writes_enabled = fields.iter().any(|(field, _)| field == "enabled");
    let writes_disabled = fields.iter().any(|(field, _)| field == "disabled");
    if !writes_type && let Some(property) = server.get("type") {
        property.remove();
    }
    if !writes_enabled && let Some(property) = server.get("enabled") {
        property.set_value(true.into());
    }
    if !writes_disabled && let Some(property) = server.get("disabled") {
        property.set_value(false.into());
    }
    for (field, value) in fields {
        set_object_property(&server, &field, value);
    }
    if let Some((field, values)) = environment {
        let environment = object_property_or_set(&server, field);
        for (key, value) in values {
            set_object_property(&environment, key, value.clone().into());
        }
    }
}

fn render_codex(existing: Option<&str>, server: &McpServer, windows: bool) -> Result<String> {
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

fn render_opencode(existing: Option<&str>, server: &McpServer, windows: bool) -> Result<String> {
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

#[cfg(test)]
mod tests;
