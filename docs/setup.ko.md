# 에이전트 설정

[English](setup.md) | 한국어

`portone setup`은 공식 스킬 4개(`portone-cli`, `portone-guide`,
`payment-code-generator`, `integration-validator`)를 설치하고 PortOne MCP
서버를 설정합니다. Node.js와 `npx`가 필요하며, 서버는
`npx -y @portone/mcp-server@latest`로 실행합니다.

## 설치

대화형으로 실행하거나 에이전트와 범위를 지정합니다.

```sh
portone setup
portone setup --agent codex --scope project
portone setup --agent claude-code,cursor --scope user
```

`--agent`는 쉼표로 구분하거나 반복해서 지정할 수 있습니다.
`--scope`는 `project` 또는 `user`이며, 비대화형 실행에는 두 옵션이 모두 사용해야 합니다.

## 설치 위치

프로젝트 범위에서는 다음 경로에 저장합니다.

| 에이전트 ID | 스킬 | MCP 설정 |
| --- | --- | --- |
| `claude-code` | `.claude/skills` | `.mcp.json` |
| `codex` | `.agents/skills` | `.codex/config.toml` |
| `cursor` | `.agents/skills` | `.cursor/mcp.json` |
| `gemini-cli` | `.agents/skills` | `.gemini/settings.json` |
| `github-copilot` | `.agents/skills` | `.mcp.json` |
| `vscode-copilot` | `.agents/skills` | `.vscode/mcp.json` |
| `opencode` | `.agents/skills` | `opencode.jsonc`가 있으면 사용, 없으면 `opencode.json` |

사용자 범위에서는 각 에이전트의 표준 설정 디렉터리를 사용하며,
해당하는 경우 `CODEX_HOME`, `CLAUDE_CONFIG_DIR`, `GEMINI_CLI_HOME`,
`COPILOT_HOME`, `XDG_CONFIG_HOME`을 따릅니다. OpenCode는 `OPENCODE_CONFIG`,
`OPENCODE_CONFIG_DIR`과 관계없이 `${XDG_CONFIG_HOME:-~/.config}/opencode`
아래에 설치합니다. VS Code는 stable 버전의 기본 프로필을 대상으로 합니다.

## 업데이트

```sh
portone setup update
portone setup update --agent codex --scope user
portone setup update --dry-run
```

`--dry-run`은 업데이트 내용을 미리 보여주며 Node.js나 `npx`가 필요하지 않습니다.

프로젝트 설치 기록은 `.portone/setup.json`, 사용자 설치 기록은 PortOne 설정
디렉터리의 `setup.json`에 저장합니다. 사용자 설정 디렉터리는 `PORTONE_CONFIG_DIR`로
바꿀 수 있습니다. 필터 없이 업데이트하면 user와 project 범위의 설치 기록을 모두 확인합니다.

스킬과 MCP 파일은 GitHub의 공식 최신 릴리스에 해당하는 커밋 하나에서 가져옵니다.
최신 릴리스가 없을 때만 기본 브랜치를 사용하며, 릴리스에 스킬 원본이 없으면 실패합니다.
GitHub 요청에는 `GH_TOKEN`, `GITHUB_TOKEN` 순서로 토큰을 사용할 수 있습니다.

## 설정 후

에이전트를 다시 시작하고 MCP 서버 목록을 확인한 뒤, PortOne 문서를 가져오도록
요청하세요. 에이전트가 작업 공간 신뢰나 MCP 승인을 요청하면 안내를 따르세요.
setup은 파일을 설정하며, 콘솔 기능은 사용 시 로그인을 요청할 수 있습니다.

[Codex](../plugins/portone-codex/README.ko.md)와
[Claude Code](../plugins/portone-integration/README.ko.md)용 네이티브 플러그인도
제공합니다. CLI 설정으로 전환할 때 스킬이나 MCP 서버가 중복되면 기존 플러그인을
비활성화하세요.
