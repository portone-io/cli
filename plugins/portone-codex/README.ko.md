# PortOne Codex 플러그인

[English](README.md) | [한국어](README.ko.md)

포트원 결제 연동을 구현하고 검토하는 Codex 플러그인입니다.

## 기능

- 포트원 V1 및 V2 결제 연동 코드를 생성합니다.
- 기존 연동을 검증하고 구체적인 문제를 진단합니다.
- 포트원 공식 문서와 MCP 예제를 기준으로 작업합니다.
- PortOne CLI로 인증, 결제 조회, API 요청을 수행합니다.

## 설치

### 스킬 및 MCP 설정

[PortOne CLI](../../README.ko.md#설치), Node.js, `npx`를 설치한 다음,
사용자 계정에 Codex용 설정을 적용합니다.

```bash
portone setup --agent codex --scope user
```

설정 명령은 포트원 공식 스킬 4개를 복사하고 PortOne MCP 서버를 직접 설정합니다.
Codex 네이티브 플러그인을 설치하거나 Codex 자체를 설치·업데이트하지는 않습니다.
설정 명령을 실행하는 데 Git이나 Codex CLI는 필요하지 않습니다.

설치한 스킬과 MCP 설정은 다음 명령으로 명시적으로 업데이트합니다.

```bash
portone setup update --agent codex --scope user
```

설정 후 새 Codex 세션을 시작하고 MCP 서버 목록을 확인한 다음, Codex에 포트원
문서를 가져오도록 요청합니다. Codex가 작업 공간 신뢰나 MCP 승인을 요청하면
해당 안내를 따르세요. 콘솔 기능을 사용할 때 로그인을 요청할 수 있으며, 설정
명령 자체는 로그인하거나 토큰을 저장하지 않습니다. 프로젝트 범위 설정, 설치
경로, 업데이트 동작은 [설정 안내](../../README.ko.md#portone-setup)를 참고하세요.

### 네이티브 플러그인 설치

네이티브 플러그인을 사용하려면 Codex 플러그인 관리자를 통해 이 저장소의
[`portone` 마켓플레이스](../../.agents/plugins/marketplace.json)에서
`portone-codex`를 설치합니다. 플러그인에는 동일한 스킬 4개와 자체 MCP 설정이
포함되어 있습니다. 직접 설정 방식으로 전환할 때 기존 플러그인의 스킬이나 서버가
중복되면 해당 플러그인을 비활성화하세요.

네이티브 플러그인에 포함된 `.mcp.json`은 다음 설정을 사용합니다.

```json
{
  "mcpServers": {
    "portone": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@portone/mcp-server@latest"]
    }
  }
}
```

## 사용법

Codex에 필요한 연동 작업을 요청합니다.

```text
포트원 V2 일회성 결제를 연동해줘.
이 프로젝트의 포트원 연동을 검토해줘.
포트원 빌링키 결제 흐름을 추가해줘.
PortOne CLI로 실패한 테스트 결제를 조회해줘.
```

## 포함된 스킬

- `payment-code-generator`: 새로운 포트원 연동을 구현합니다.
- `integration-validator`: 기존 연동이나 새로 생성한 연동을 검증합니다.
- `portone-guide`: 포트원 개념을 설명하고 공식 가이드를 찾습니다.
- `portone-cli`: 인증하고 PortOne CLI의 결제 및 API 명령을 사용합니다.

## 포함된 스킬 유지보수

이 플러그인의 `skills/` 아래에 있는 디렉터리 4개는 모두 저장소 루트의 `skills/`
아래 원본 디렉터리에서 생성한 복사본입니다. 루트의 원본을 수정한 다음 플러그인
복사본을 동기화합니다.

```bash
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

원본 변경 사항과 생성된 플러그인 복사본을 모두 함께 커밋하세요. 동기화 명령은
두 플러그인의 관리 대상 스킬 전체를 업데이트합니다. 검사 명령은 누락되거나
내용이 달라진 파일, 더 이상 필요하지 않은 생성 파일을 보고합니다.

## 라이선스

MIT 라이선스
