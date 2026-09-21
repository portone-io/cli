# PortOne Integration 플러그인

[English](README.md) | [한국어](README.ko.md)

Claude Code에서 포트원 V1·V2 연동을 구현하고 검토합니다.
공식 문서와 MCP 도구를 사용해 일회성 결제, 빌링키 결제, 수기 결제, 본인인증을 지원합니다.

## 설치

Claude Code 플러그인 관리자에서 이 저장소의
[`portone` 마켓플레이스](../../.claude-plugin/marketplace.json)를 통해
`portone-integration`을 설치합니다. 포함된 MCP 서버를 실행하려면 Node.js와 `npx`가 필요합니다.

`/portone-integration:start` 명령, 코드 생성·검토 에이전트,
`portone-cli`·`portone-guide` 스킬이 포함되어 있습니다.

## 사용법

대화형으로 시작하거나 버전과 결제 유형을 지정합니다.

```text
/portone-integration:start
/portone-integration:start v2 checkout
```

버전은 `v1`, `v2`, 유형은 `checkout`, `billing`, `keyin`, `identity`를 지원합니다.
Claude에 직접 요청할 수도 있습니다.

```text
src/payment/의 포트원 연동을 검토해줘.
PortOne CLI로 실패한 테스트 결제를 조회해줘.
```

## CLI로 설정

[PortOne CLI](../../README.ko.md#설치)를 설치한 뒤 스킬과 MCP를 설정합니다.

```sh
portone setup --agent claude-code --scope user
portone setup update --agent claude-code --scope user
```

CLI 설정에는 자연어 요청을 처리하는 스킬 4개가 포함됩니다.
`/start` 명령과 전문 에이전트는 네이티브 플러그인을 설치해야 사용할 수 있습니다.
설정 후 Claude Code를 다시 시작하고 `/mcp`로 서버를 확인하세요.
플러그인에서 CLI 설정으로 전환했다면 중복되지 않도록 플러그인을 비활성화하세요.
설치 범위와 경로는 [설정 안내](../../docs/setup.ko.md)를 참고하세요.

## 기여

저장소의 `skills/` 아래 `portone-cli`와 `portone-guide` 원본을 수정한 뒤
[플러그인 사본을 동기화](../../docs/development.ko.md)하세요.
플러그인의 `commands/`와 `agents/`는 별도로 관리합니다.

## 라이선스

MIT
