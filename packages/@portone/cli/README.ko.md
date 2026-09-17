# PortOne CLI

[English](README.md) | [한국어](README.ko.md)

[![npm 버전](https://img.shields.io/npm/v/%40portone%2Fcli)](https://www.npmjs.com/package/@portone/cli)
[![라이선스](https://img.shields.io/github/license/portone-io/portone-cli)](https://github.com/portone-io/portone-cli/blob/main/LICENSE)

`portone`은 터미널에서 PortOne을 사용하는 도구입니다.
결제를 검색·취소하고, 웹훅을 관리하고, PortOne V2 API를 호출할 수 있습니다.

macOS, Windows, Linux에서 사용할 수 있으며 영어와 한국어를 지원합니다.

## 설치

```sh
npm install --global @portone/cli
```

Node.js 22.x의 22.18.0 이상 버전 또는 Node.js 24 이상이 필요합니다.

## 빠른 시작

PortOne 콘솔에서 로그인한 뒤 결제를 조회합니다.

```sh
portone auth login
portone payment list --test --status failed
portone payment view --payment-id payment-xxx
```

`payment-xxx`를 결제 시 설정한 paymentId 로 변경해주세요.
명령어별 옵션과 예제는 `portone <command> --help`로 확인할 수 있습니다.

## 문서

- [명령어 참조](https://github.com/portone-io/portone-cli/blob/main/docs/reference/ko/index.md)
- [사용 예제](https://github.com/portone-io/portone-cli/blob/main/docs/usage.ko.md): 결제, API 요청, 셸 자동완성
- [설정](https://github.com/portone-io/portone-cli/blob/main/docs/configuration.ko.md): 인증, 프로필, 상점, 언어

## 에이전트 스킬

코딩 에이전트에 PortOne 스킬과 MCP 설정을 설치합니다.

```sh
portone setup

# 에이전트와 설치 범위 지정
portone setup --agent codex --scope user

# 설치한 스킬과 MCP 설정 업데이트
portone setup update
```

Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot CLI,
VS Code Copilot, OpenCode를 지원합니다.
옵션과 설치 경로는 [설정 안내](https://github.com/portone-io/portone-cli/blob/main/docs/setup.ko.md)를 참고하세요.

## 기여

소스 빌드와 문서 수정은 [개발 안내](https://github.com/portone-io/portone-cli/blob/main/docs/development.ko.md)를 참고하세요.
버그 제보와 기능 제안은 [GitHub Issues](https://github.com/portone-io/portone-cli/issues)에 남겨주세요.
