use anyhow::{Result, bail, ensure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SharedMcpGroup {
    ClaudeCopilot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum McpFormat {
    McpServersJson {
        include_tools: bool,
        normalize_bare_map: bool,
    },
    CodexToml,
    VscodeJson,
    OpencodeJson,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct McpRequirements {
    pub format: McpFormat,
    pub shared_group: Option<SharedMcpGroup>,
}

impl McpRequirements {
    pub fn merge(self, other: Self) -> Result<Self> {
        ensure!(
            self.shared_group.is_some() && self.shared_group == other.shared_group,
            "agents do not share an MCP configuration format and destination"
        );
        let format = match (self.format, other.format) {
            (
                McpFormat::McpServersJson {
                    include_tools: left_tools,
                    normalize_bare_map: left_bare_map,
                },
                McpFormat::McpServersJson {
                    include_tools: right_tools,
                    normalize_bare_map: right_bare_map,
                },
            ) => McpFormat::McpServersJson {
                include_tools: left_tools || right_tools,
                normalize_bare_map: left_bare_map || right_bare_map,
            },
            (left, right) if left == right => left,
            _ => bail!("agents do not share an MCP configuration format and destination"),
        };
        Ok(Self {
            format,
            shared_group: self.shared_group,
        })
    }
}
