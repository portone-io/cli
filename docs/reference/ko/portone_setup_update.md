# portone setup update

[English](../portone_setup_update.md) | 한국어

기록된 PortOne 스킬과 MCP 설정 업데이트

```
portone setup update [OPTIONS]
```

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `--dry-run` | 파일을 변경하지 않고 업데이트 내용 확인 |
| `--agent <AGENT>` | 설정할 에이전트 (쉼표로 구분하거나 여러 번 지정) [가능한 값: claude-code, codex, cursor, gemini-cli, github-copilot, vscode-copilot, opencode] |
| `--scope <SCOPE>` | 설치 범위 (project \| user) [가능한 값: project, user] |

## 참고

- [portone setup](portone_setup.md)
