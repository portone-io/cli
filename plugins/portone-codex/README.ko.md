# PortOne Codex 플러그인

[English](README.md) | [한국어](README.ko.md)

공식 문서, MCP 도구, PortOne CLI를 사용해 Codex에서 포트원 결제 연동을
구현하고 검토합니다.

## 설치

Codex 플러그인 관리자에서 이 저장소의
[`portone` 마켓플레이스](../../.agents/plugins/marketplace.json)를 통해
`portone-codex`를 설치합니다. 포함된 MCP 서버를 실행하려면 Node.js와 `npx`가 필요합니다.

`portone-cli`, `portone-guide`, `payment-code-generator`,
`integration-validator` 스킬 4개가 포함되어 있습니다.

## 사용법

```text
포트원 V2 일회성 결제를 연동해줘.
이 프로젝트의 포트원 연동을 검토해줘.
PortOne CLI로 실패한 테스트 결제를 조회해줘.
```

## CLI로 설정

[PortOne CLI](../../README.ko.md#설치)로 같은 스킬과 MCP 설정을 설치할 수도 있습니다.

```sh
portone setup --agent codex --scope user
portone setup update --agent codex --scope user
```

설정 후 Codex를 다시 시작하세요. 플러그인에서 CLI 설정으로 전환했다면
스킬과 MCP 서버가 중복되지 않도록 플러그인을 비활성화하세요.
설치 범위와 경로는 [설정 안내](../../docs/setup.ko.md)를 참고하세요.

## 기여

저장소의 `skills/` 아래 원본을 수정한 뒤
[플러그인 사본을 동기화](../../docs/development.ko.md)하세요.

## 라이선스

MIT
