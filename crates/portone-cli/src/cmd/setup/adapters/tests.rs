use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jsonc_parser::{ParseOptions, parse_to_value};

use super::*;
use crate::cmd::setup::model::{Agent, McpServer, Scope};

fn path(value: &str) -> PathBuf {
    PathBuf::from(value)
}

fn server() -> McpServer {
    McpServer {
        command: "npx".into(),
        args: vec!["-y".into(), "@portone/mcp@latest".into()],
        env: BTreeMap::from([
            ("PORTONE_API_SECRET".into(), "secret".into()),
            ("PORTONE_STORE_ID".into(), "store".into()),
        ]),
    }
}

fn json_value(text: &str) -> serde_json::Value {
    fn convert(value: jsonc_parser::JsonValue<'_>) -> serde_json::Value {
        match value {
            jsonc_parser::JsonValue::Null => serde_json::Value::Null,
            jsonc_parser::JsonValue::Boolean(value) => value.into(),
            jsonc_parser::JsonValue::Number(value) => serde_json::from_str(value).unwrap(),
            jsonc_parser::JsonValue::String(value) => value.into_owned().into(),
            jsonc_parser::JsonValue::Array(values) => {
                values.into_iter().map(convert).collect::<Vec<_>>().into()
            }
            jsonc_parser::JsonValue::Object(values) => values
                .into_iter()
                .map(|(key, value)| (key.into_owned(), convert(value)))
                .collect::<serde_json::Map<_, _>>()
                .into(),
        }
    }

    convert(
        parse_to_value(text, &ParseOptions::default())
            .unwrap()
            .unwrap(),
    )
}

#[test]
fn project_destinations_cover_all_agents_and_shared_paths() {
    let env = BTreeMap::new();
    let cases = [
        (Agent::ClaudeCode, ".claude/skills", ".mcp.json"),
        (Agent::Codex, ".agents/skills", ".codex/config.toml"),
        (Agent::Cursor, ".agents/skills", ".cursor/mcp.json"),
        (Agent::GeminiCli, ".agents/skills", ".gemini/settings.json"),
        (Agent::GithubCopilot, ".agents/skills", ".mcp.json"),
        (Agent::VscodeCopilot, ".agents/skills", ".vscode/mcp.json"),
        (Agent::Opencode, ".agents/skills", "opencode.json"),
    ];

    for (agent, skills, mcp) in cases {
        let actual = resolve_destination_for_platform(
            agent,
            Scope::Project,
            Path::new("/project"),
            Path::new("/home/dev"),
            &env,
            HostPlatform::Linux,
        )
        .unwrap();
        assert_eq!(actual.agent, agent);
        assert_eq!(actual.skills_dir, path("/project").join(skills));
        assert_eq!(actual.mcp_path, path("/project").join(mcp));
    }
}

#[test]
fn user_destinations_cover_all_agents_on_linux() {
    let env = BTreeMap::new();
    let cases = [
        (Agent::ClaudeCode, ".claude/skills", ".claude.json"),
        (Agent::Codex, ".agents/skills", ".codex/config.toml"),
        (Agent::Cursor, ".cursor/skills", ".cursor/mcp.json"),
        (Agent::GeminiCli, ".gemini/skills", ".gemini/settings.json"),
        (
            Agent::GithubCopilot,
            ".copilot/skills",
            ".copilot/mcp-config.json",
        ),
        (
            Agent::VscodeCopilot,
            ".copilot/skills",
            ".config/Code/User/mcp.json",
        ),
        (
            Agent::Opencode,
            ".config/opencode/skills",
            ".config/opencode/opencode.json",
        ),
    ];

    for (agent, skills, mcp) in cases {
        let actual = resolve_destination_for_platform(
            agent,
            Scope::User,
            Path::new("/project"),
            Path::new("/home/dev"),
            &env,
            HostPlatform::Linux,
        )
        .unwrap();
        assert_eq!(actual.skills_dir, path("/home/dev").join(skills));
        assert_eq!(actual.mcp_path, path("/home/dev").join(mcp));
    }
}

#[test]
fn user_environment_overrides_have_documented_meaning() {
    let env = BTreeMap::from([
        ("CLAUDE_CONFIG_DIR".into(), "/cfg/claude".into()),
        ("CODEX_HOME".into(), "/cfg/codex".into()),
        ("GEMINI_CLI_HOME".into(), "/cfg/gemini-home".into()),
        ("COPILOT_HOME".into(), "/cfg/copilot".into()),
        ("XDG_CONFIG_HOME".into(), "/cfg/xdg".into()),
    ]);

    let expected = [
        (
            Agent::ClaudeCode,
            "/cfg/claude/skills",
            "/cfg/claude/.claude.json",
        ),
        (
            Agent::Codex,
            "/home/dev/.agents/skills",
            "/cfg/codex/config.toml",
        ),
        (
            Agent::GeminiCli,
            "/cfg/gemini-home/.gemini/skills",
            "/cfg/gemini-home/.gemini/settings.json",
        ),
        (
            Agent::GithubCopilot,
            "/cfg/copilot/skills",
            "/cfg/copilot/mcp-config.json",
        ),
        (
            Agent::VscodeCopilot,
            "/home/dev/.copilot/skills",
            "/cfg/xdg/Code/User/mcp.json",
        ),
        (
            Agent::Opencode,
            "/cfg/xdg/opencode/skills",
            "/cfg/xdg/opencode/opencode.json",
        ),
    ];

    for (agent, skills, mcp) in expected {
        let actual = resolve_destination_for_platform(
            agent,
            Scope::User,
            Path::new("/project"),
            Path::new("/home/dev"),
            &env,
            HostPlatform::Linux,
        )
        .unwrap();
        assert_eq!(actual.skills_dir, path(skills));
        assert_eq!(actual.mcp_path, path(mcp));
    }
}

#[test]
fn vscode_user_config_is_platform_specific_and_honors_appdata() {
    let mac = resolve_destination_for_platform(
        Agent::VscodeCopilot,
        Scope::User,
        Path::new("/project"),
        Path::new("/Users/dev"),
        &BTreeMap::new(),
        HostPlatform::Macos,
    )
    .unwrap();
    assert_eq!(
        mac.mcp_path,
        path("/Users/dev/Library/Application Support/Code/User/mcp.json")
    );

    let windows = resolve_destination_for_platform(
        Agent::VscodeCopilot,
        Scope::User,
        Path::new("C:/project"),
        Path::new("C:/Users/dev"),
        &BTreeMap::from([("APPDATA".into(), "D:/Roaming".into())]),
        HostPlatform::Windows,
    )
    .unwrap();
    assert_eq!(windows.mcp_path, path("D:/Roaming/Code/User/mcp.json"));
}

#[test]
fn opencode_prefers_standard_jsonc_then_json_and_legacy_config() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("opencode.json"), "{}").unwrap();
    std::fs::write(project.join("opencode.jsonc"), "{}").unwrap();

    let project_destination = resolve_destination_for_platform(
        Agent::Opencode,
        Scope::Project,
        &project,
        temp.path(),
        &BTreeMap::new(),
        HostPlatform::Linux,
    )
    .unwrap();
    assert_eq!(project_destination.mcp_path, project.join("opencode.jsonc"));

    let standard = temp.path().join("xdg/opencode");
    let extra = temp.path().join("extra");
    std::fs::create_dir_all(&standard).unwrap();
    std::fs::create_dir_all(&extra).unwrap();
    std::fs::write(standard.join("config.json"), "{}").unwrap();
    std::fs::write(extra.join("opencode.json"), "{}").unwrap();
    let env = BTreeMap::from([
        (
            "XDG_CONFIG_HOME".into(),
            temp.path().join("xdg").to_string_lossy().into_owned(),
        ),
        (
            "OPENCODE_CONFIG_DIR".into(),
            extra.to_string_lossy().into_owned(),
        ),
    ]);
    let user_destination = resolve_destination_for_platform(
        Agent::Opencode,
        Scope::User,
        &project,
        temp.path(),
        &env,
        HostPlatform::Linux,
    )
    .unwrap();
    assert_eq!(user_destination.skills_dir, standard.join("skills"));
    assert_eq!(user_destination.mcp_path, standard.join("config.json"));
}

#[test]
fn relative_user_environment_roots_are_rejected() {
    for (agent, variable) in [
        (Agent::ClaudeCode, "CLAUDE_CONFIG_DIR"),
        (Agent::Codex, "CODEX_HOME"),
        (Agent::GeminiCli, "GEMINI_CLI_HOME"),
        (Agent::GithubCopilot, "COPILOT_HOME"),
        (Agent::VscodeCopilot, "XDG_CONFIG_HOME"),
        (Agent::Opencode, "XDG_CONFIG_HOME"),
    ] {
        let error = resolve_destination_for_platform(
            agent,
            Scope::User,
            Path::new("/project"),
            Path::new("/home/dev"),
            &BTreeMap::from([(variable.into(), "relative/path".into())]),
            HostPlatform::Linux,
        )
        .unwrap_err();
        assert!(error.to_string().contains(variable));
        assert!(error.to_string().contains("absolute"));
    }
}

#[test]
fn jsonc_render_preserves_comments_trailing_commas_and_unrelated_secrets() {
    let existing = r#"{
  // leave this account configuration alone
  "account": { "token": "keep-me", },
  "mcpServers": {
    "other": { "command": "other", },
    "portone": {
      "command": "old",
      "url": "https://old.example/mcp",
      "httpUrl": "https://older.example/mcp",
      "type": "sse",
      "headers": { "Authorization": "old" },
      "disabled": true,
      "trust": "keep",
      "env": { "EXTRA_SECRET": "do-not-delete" },
    },
  },
}
"#;
    let rendered = render_mcp(Some(existing), &[Agent::Cursor], &server(), false).unwrap();

    assert!(rendered.contains("// leave this account configuration alone"));
    assert!(rendered.contains("\"token\": \"keep-me\""));
    assert!(rendered.contains("\"other\": { \"command\": \"other\", }"));
    let value = json_value(&rendered);
    assert_eq!(value["mcpServers"]["portone"]["command"], "npx");
    assert_eq!(
        value["mcpServers"]["portone"]["args"],
        serde_json::json!(["-y", "@portone/mcp@latest"])
    );
    assert_eq!(
        value["mcpServers"]["portone"]["env"]["PORTONE_API_SECRET"],
        "secret"
    );
    assert_eq!(value["mcpServers"]["portone"]["trust"], "keep");
    assert!(value["mcpServers"]["portone"].get("url").is_none());
    assert!(value["mcpServers"]["portone"].get("httpUrl").is_none());
    assert!(value["mcpServers"]["portone"].get("type").is_none());
    assert!(value["mcpServers"]["portone"].get("headers").is_none());
    assert_eq!(value["mcpServers"]["portone"]["disabled"], false);
    assert_eq!(
        value["mcpServers"]["portone"]["env"]["EXTRA_SECRET"],
        "do-not-delete"
    );
}

#[test]
fn claude_and_copilot_shared_file_includes_copilot_tools() {
    let rendered = render_mcp(
        None,
        &[Agent::ClaudeCode, Agent::GithubCopilot],
        &server(),
        false,
    )
    .unwrap();
    let value = json_value(&rendered);
    assert_eq!(
        value["mcpServers"]["portone"]["tools"],
        serde_json::json!(["*"])
    );
    assert_eq!(value["mcpServers"]["portone"]["command"], "npx");
}

#[test]
fn copilot_bare_server_map_is_normalized_without_losing_servers() {
    let existing = r#"{
  // existing Copilot server
  "database": { "command": "db", "env": { "TOKEN": "keep" } },
  "$schema": "https://example/schema.json",
  "inputs": [{ "id": "token" }]
}"#;
    let rendered = render_mcp(Some(existing), &[Agent::GithubCopilot], &server(), false).unwrap();
    assert!(rendered.contains("// existing Copilot server"));
    let value = json_value(&rendered);
    assert!(value.get("database").is_none());
    assert_eq!(value["$schema"], "https://example/schema.json");
    assert!(value["mcpServers"].get("$schema").is_none());
    assert_eq!(value["inputs"][0]["id"], "token");
    assert_eq!(value["mcpServers"]["database"]["command"], "db");
    assert_eq!(value["mcpServers"]["database"]["env"]["TOKEN"], "keep");
    assert_eq!(
        value["mcpServers"]["portone"]["tools"],
        serde_json::json!(["*"])
    );
}

#[test]
fn vscode_uses_servers_schema_and_windows_wraps_npx() {
    let rendered = render_mcp(None, &[Agent::VscodeCopilot], &server(), true).unwrap();
    let value = json_value(&rendered);
    assert!(value.get("mcpServers").is_none());
    assert_eq!(value["servers"]["portone"]["type"], "stdio");
    assert_eq!(value["servers"]["portone"]["command"], "cmd");
    assert_eq!(
        value["servers"]["portone"]["args"],
        serde_json::json!(["/c", "npx", "-y", "@portone/mcp@latest"])
    );
}

#[test]
fn codex_toml_update_preserves_comments_and_unrelated_tables() {
    let existing = r#"# user comment
model = "gpt-5"

[mcp_servers.other]
command = "other"

[mcp_servers.other.env]
TOKEN = "keep"

[mcp_servers.portone]
command = "old"
trust = "keep"
url = "https://old.example/mcp"
enabled = false
http_headers = { Authorization = "old" }

[mcp_servers.portone.env]
EXTRA_SECRET = "do-not-delete"
"#;
    let rendered = render_mcp(Some(existing), &[Agent::Codex], &server(), false).unwrap();
    assert!(rendered.contains("# user comment"));
    assert!(rendered.contains("model = \"gpt-5\""));
    assert!(rendered.contains("[mcp_servers.other.env]"));
    assert!(rendered.contains("TOKEN = \"keep\""));
    let value: toml::Value = toml::from_str(&rendered).unwrap();
    assert_eq!(
        value["mcp_servers"]["portone"]["command"],
        toml::Value::String("npx".into())
    );
    assert_eq!(
        value["mcp_servers"]["portone"]["args"],
        toml::Value::Array(vec![
            toml::Value::String("-y".into()),
            toml::Value::String("@portone/mcp@latest".into()),
        ])
    );
    assert_eq!(
        value["mcp_servers"]["portone"]["env"]["PORTONE_STORE_ID"],
        toml::Value::String("store".into())
    );
    assert_eq!(
        value["mcp_servers"]["portone"]["trust"],
        toml::Value::String("keep".into())
    );
    assert!(value["mcp_servers"]["portone"].get("url").is_none());
    assert!(
        value["mcp_servers"]["portone"]
            .get("http_headers")
            .is_none()
    );
    assert_eq!(
        value["mcp_servers"]["portone"]["enabled"],
        toml::Value::Boolean(true)
    );
    assert_eq!(
        value["mcp_servers"]["portone"]["env"]["EXTRA_SECRET"],
        toml::Value::String("do-not-delete".into())
    );
}

#[test]
fn opencode_updates_v2_and_existing_native_schema() {
    let existing = r#"{
  "theme": "keep",
  "mcp": {
    "portone": {
      "type": "sse",
      "url": "https://old.example/mcp",
      "command": ["old"],
      "enabled": false
    },
    "servers": {
      "portone": {
        "type": "sse",
        "httpUrl": "https://old.example/mcp",
        "command": "old",
        "disabled": true
      },
      "other": { "command": "keep" }
    }
  }
}"#;
    let rendered = render_mcp(Some(existing), &[Agent::Opencode], &server(), false).unwrap();
    let value = json_value(&rendered);
    assert_eq!(value["theme"], "keep");
    assert_eq!(value["mcp"]["portone"]["type"], "local");
    assert_eq!(
        value["mcp"]["portone"]["command"],
        serde_json::json!(["npx", "-y", "@portone/mcp@latest"])
    );
    assert_eq!(value["mcp"]["portone"]["enabled"], true);
    assert!(value["mcp"]["portone"].get("url").is_none());
    assert_eq!(value["mcp"]["servers"]["portone"]["disabled"], false);
    assert!(value["mcp"]["servers"]["portone"].get("httpUrl").is_none());
    assert!(value["mcp"]["servers"]["portone"].get("type").is_none());
    assert_eq!(value["mcp"]["servers"]["other"]["command"], "keep");
}

#[test]
fn rendering_is_idempotent() {
    for agents in [
        vec![Agent::ClaudeCode, Agent::GithubCopilot],
        vec![Agent::Codex],
        vec![Agent::VscodeCopilot],
        vec![Agent::Opencode],
    ] {
        let first = render_mcp(None, &agents, &server(), false).unwrap();
        let second = render_mcp(Some(&first), &agents, &server(), false).unwrap();
        assert_eq!(second, first, "not idempotent for {agents:?}");
    }
}

#[test]
fn malformed_roots_containers_and_disjoint_agents_are_rejected() {
    assert!(render_mcp(Some("[1, 2]"), &[Agent::Cursor], &server(), false).is_err());
    assert!(
        render_mcp(
            Some(r#"{ "mcpServers": "wrong" }"#),
            &[Agent::Cursor],
            &server(),
            false,
        )
        .is_err()
    );
    assert!(render_mcp(None, &[Agent::Cursor, Agent::GeminiCli], &server(), false,).is_err());
    assert!(render_mcp(Some("mcp_servers = 1"), &[Agent::Codex], &server(), false).is_err());
}

#[test]
fn codex_preserves_user_environment_in_all_valid_toml_table_forms() {
    for existing in [
        "[mcp_servers.portone]\ncommand = 'old'\nenv = { EXTRA_SECRET = 'keep' }\n",
        "[mcp_servers]\nportone = { command = 'old', env = { EXTRA_SECRET = 'keep' } }\n",
        "mcp_servers = { portone = { command = 'old', env = { EXTRA_SECRET = 'keep' } }, other = { command = 'other' } }\n",
    ] {
        let rendered = render_mcp(Some(existing), &[Agent::Codex], &server(), false).unwrap();
        let value: toml::Value = toml::from_str(&rendered).unwrap();
        assert_eq!(
            value["mcp_servers"]["portone"]["env"]["EXTRA_SECRET"].as_str(),
            Some("keep"),
            "{rendered}"
        );
        assert_eq!(
            value["mcp_servers"]["portone"]["command"].as_str(),
            Some("npx")
        );
        if existing.contains("other") {
            assert_eq!(
                value["mcp_servers"]["other"]["command"].as_str(),
                Some("other")
            );
        }
    }
}
