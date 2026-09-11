# PortOne Integration 플러그인

[English](README.md) | [한국어](README.ko.md)

포트원 결제 연동을 구현하고 검토하는 Claude Code 플러그인입니다.

## 기능

- 지원하는 프런트엔드 및 백엔드 프레임워크에 맞춰 포트원 V1 및 V2 연동 코드를
  생성합니다.
- 일회성 결제, 빌링키 결제, 수기 결제, 본인인증을 지원합니다.
- 기존 연동의 보안, 정확성, 포트원 권장 방식 준수 여부를 검토합니다.
- 포함된 `portone-cli` 스킬을 통해 PortOne CLI로 인증, 결제 조회, API 요청을
  수행합니다.

## 설치

### 스킬 및 MCP 설정

[PortOne CLI](../../README.ko.md#설치), Node.js, `npx`를 설치한 다음,
사용자 계정에 Claude Code용 설정을 적용합니다.

```bash
portone setup --agent claude-code --scope user
```

설정 명령은 포트원 공식 스킬 4개(`portone-cli`, `portone-guide`,
`payment-code-generator`, `integration-validator`)를 복사하고 PortOne MCP
서버를 직접 설정합니다. Claude Code 네이티브 플러그인이나 플러그인의
`/portone-integration:start` 명령, 전문 에이전트를 설치하지는 않습니다. 설정
명령을 실행하는 데 Git이나 Claude Code CLI는 필요하지 않으며, Claude Code
자체를 설치·업데이트하지도 않습니다.

설치한 스킬과 MCP 설정은 다음 명령으로 명시적으로 업데이트합니다.

```bash
portone setup update --agent claude-code --scope user
```

설정된 MCP 서버는 `npx -y @portone/mcp-server@latest`를 실행합니다. 새 Claude
Code 세션을 시작하고 `/mcp`로 PortOne 서버를 확인한 다음, Claude에 포트원 문서를
가져오도록 요청합니다. Claude Code가 작업 공간 신뢰나 MCP 승인을 요청하면 해당
안내를 따르세요. 콘솔 기능을 사용할 때 로그인을 요청할 수 있으며, 설정 명령
자체는 로그인하거나 토큰을 저장하지 않습니다. 프로젝트 범위 설정, 설치 경로,
업데이트 동작은 [설정 안내](../../README.ko.md#portone-setup)를 참고하세요.

### 네이티브 플러그인 설치

`/portone-integration:start`와 전문 에이전트를 사용하려면 Claude Code 플러그인
관리자를 통해 이 저장소의
[`portone` 마켓플레이스](../../.claude-plugin/marketplace.json)에서
`portone-integration`을 별도로 설치합니다. 네이티브 플러그인에는 자체 MCP 설정,
명령, 에이전트와 `portone-cli`, `portone-guide` 스킬이 포함되어 있습니다. 직접
설정 방식으로 전환할 때 기존 플러그인의 스킬이나 서버가 중복되면 해당 플러그인을
비활성화하세요.

## 사용법

### `/start`

네이티브 플러그인을 설치한 상태에서 대화형으로 결제 연동 코드를 생성합니다.

```text
/portone-integration:start
/portone-integration:start v2
/portone-integration:start v2 checkout
/portone-integration:start v1 billing
```

결제 유형:

- `checkout`: PG사 결제창을 통한 일회성 결제.
- `billing`: 빌링키를 사용한 정기 결제 또는 필요할 때 청구하는 결제.
- `keyin`: 카드 정보를 직접 입력하는 수기 결제.
- `identity`: 본인인증.

### 연동 검토

Claude에 기존 포트원 연동의 검토를 요청합니다. 네이티브 플러그인의
`integration-validator` 에이전트가 다음과 같은 요청을 처리합니다.

```text
src/payment/의 포트원 연동에 보안 문제가 있는지 검토해줘.
src/api/pay.ts의 포트원 API 호출을 검증해줘.
```

자연어 요청은 스킬을 직접 설정한 경우와 네이티브 플러그인을 설치한 경우 모두
사용할 수 있습니다. 네이티브 플러그인은 전문 에이전트에 작업을 위임할 수도
있습니다.

```text
포트원 결제 기능을 구현해줘.
정기 결제 연동을 추가해줘.
이 포트원 연동에 보안 문제가 있는지 검토해줘.
PortOne CLI로 실패한 테스트 결제를 조회해줘.
```

## 포함된 스킬 유지보수

이 플러그인의 `skills/portone-cli/`와 `skills/portone-guide/`는 저장소 루트의
`skills/` 아래 원본 디렉터리에서 생성한 복사본입니다. 루트의 원본을 수정한 다음
플러그인 복사본을 동기화합니다.

```bash
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

원본 변경 사항과 생성된 플러그인 복사본을 모두 함께 커밋하세요. 동기화 명령은
두 플러그인의 관리 대상 스킬 전체를 업데이트합니다. 검사 명령은 누락되거나
내용이 달라진 파일, 더 이상 필요하지 않은 생성 파일을 보고합니다. 이 플러그인의
`commands/`와 `agents/`는 별도로 관리하며 이 명령으로 생성하지 않습니다.

## 지원 프레임워크

프런트엔드 예제는 React, 순수 HTML/JavaScript와 Vue에 맞춘 적용 예제를
포함합니다. 백엔드 예제는 Express, FastAPI, Flask, Kotlin을 사용하는 Spring을
포함합니다.

## 연동 방식 선택

- PG사 결제창에서 개별 구매를 완료하려면 일회성 결제를 사용하세요.
- 구독, 멤버십, 서버에서 시작하는 청구에는 빌링키 결제를 사용하세요.
- 회원 가입, 연령 확인 등의 흐름에는 본인인증을 사용하세요.
- 새 프로젝트에는 V2를 권장합니다. 기존 V1 연동을 유지보수하거나 필요한 PG사
  기능이 V1에서만 제공되는 경우에는 V1을 사용하세요.

## 보안

- 클라이언트 코드에 API Secret을 노출하지 마세요.
- 완료된 결제는 서버에서 검증하세요.
- 인증 정보는 환경 변수에 보관하고 `.env` 파일을 버전 관리에서 제외하세요.

## 라이선스

MIT 라이선스
