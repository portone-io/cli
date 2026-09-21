use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub const REPOSITORY: &str = "portone-io/cli";
pub const SKILL_NAMES: [&str; 4] = [
    "portone-cli",
    "portone-guide",
    "payment-code-generator",
    "integration-validator",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    ClaudeCode,
    Codex,
    Cursor,
    GeminiCli,
    GithubCopilot,
    VscodeCopilot,
    Opencode,
}

impl Agent {
    pub const ALL: [Self; 7] = [
        Self::ClaudeCode,
        Self::Codex,
        Self::Cursor,
        Self::GeminiCli,
        Self::GithubCopilot,
        Self::VscodeCopilot,
        Self::Opencode,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::GeminiCli => "gemini-cli",
            Self::GithubCopilot => "github-copilot",
            Self::VscodeCopilot => "vscode-copilot",
            Self::Opencode => "opencode",
        }
    }
}
impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::Cursor => "Cursor",
            Self::GeminiCli => "Gemini CLI",
            Self::GithubCopilot => "GitHub Copilot CLI",
            Self::VscodeCopilot => "VS Code Copilot",
            Self::Opencode => "OpenCode",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Project,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Destination {
    pub agent: Agent,
    pub skills_dir: PathBuf,
    pub mcp_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    pub repository: String,
    pub reference: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServer {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SkillBundle {
    pub tree_sha: String,
    pub files: BTreeMap<PathBuf, Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct Bundle {
    pub revision: Revision,
    pub skills: BTreeMap<String, SkillBundle>,
    pub mcp: McpServer,
}
