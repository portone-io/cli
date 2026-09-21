use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use jsonc_parser::{JsonValue, ParseOptions, json, parse_to_value};

use super::{McpFormat, McpServer, normalized_command};

pub(super) fn input_array(values: impl IntoIterator<Item = String>) -> CstInputValue {
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

pub(super) fn render_generic(
    existing: Option<&str>,
    server: &McpServer,
    windows: bool,
    format: McpFormat,
) -> Result<String> {
    let McpFormat::McpServersJson {
        include_tools,
        normalize_bare_map,
    } = format
    else {
        bail!("incompatible MCP rendering requirements");
    };
    let normalized;
    let existing = if let (true, Some(existing)) = (normalize_bare_map, existing) {
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

pub(super) fn render_json_container_with_fields(
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

pub(super) fn parse_json(text: &str) -> Result<CstRootNode> {
    CstRootNode::parse(text, &ParseOptions::default()).context("failed to parse MCP JSON/JSONC")
}

pub(super) fn object_container(object: &CstObject, name: &str) -> Result<CstObject> {
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

pub(super) fn update_server_object(
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
