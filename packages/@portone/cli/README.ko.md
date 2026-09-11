# portone-cli

[English](README.md) | [한국어](README.ko.md)

[![npm 버전](https://img.shields.io/npm/v/%40portone%2Fcli)](https://www.npmjs.com/package/@portone/cli)
[![라이선스](https://img.shields.io/github/license/portone-io/portone-cli)](https://github.com/portone-io/portone-cli/blob/main/LICENSE)

PortOne CLI는 PortOne 결제와 본인인증 연동을 돕는 도구입니다.
결제 검색·조회·취소와 웹훅 관리(`portone payment`), 인증 정보를 사용한
PortOne V2 API 요청(`portone api`), 인증 관리(`portone auth`),
지원하는 코딩 에이전트에 PortOne 스킬과 MCP 설정 설치(`portone setup`) 기능을 제공합니다.

- 저장소: <https://github.com/portone-io/portone-cli>
- 이슈: <https://github.com/portone-io/portone-cli/issues>

## 설치

```bash
npm install --global @portone/cli
```

Node.js 22.x의 22.18.0 이상 버전 또는 Node.js 24 이상이 필요합니다.

## 표시 언어

CLI는 영어와 한국어를 지원합니다. 언어를 자동으로 감지하며,
지원하는 언어를 찾지 못하면 영어를 사용합니다.
한 번의 실행에 적용하려면 `PORTONE_LANG`을 설정하고,
언어 설정을 저장하려면 `config.toml`의 최상위 `language` 항목을 사용하세요.

```bash
PORTONE_LANG=en portone auth status
PORTONE_LANG=ko portone --help
```

```toml
language = "ko" # en, ko 또는 auto (기본값)
```

`PORTONE_LANG`은 저장된 설정보다 우선합니다. `auto`로 설정하면 저장된 설정을
무시하고 언어를 자동으로 감지합니다. 빈 값은 설정하지 않은 것으로 취급하며,
지원하지 않는 값은 영어로 처리합니다. `ko-KR`, `ko_KR.UTF-8`과 같은 지역별
로캘도 인식합니다.

PortOne 언어를 명시적으로 설정하지 않으면 운영체제의 기본 언어를 자동으로
감지합니다. macOS와 Windows에서는 OS의 UI 언어 설정을 사용합니다.
Linux에서는 `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG` 순서로
언어 후보를 수집합니다. `LANGUAGE`에는 콜론으로 구분한 언어 목록을
선호하는 순서대로 지정할 수 있습니다. 지원하는 첫 번째 언어를 선택하며,
해당하는 언어가 없으면 영어를 사용합니다. 모든 플랫폼에서 영어를 사용하려면
`PORTONE_LANG=en`을 설정하세요.

도움말, 대화형 입력 안내, 인증 상태, CLI 진단 메시지에는 선택한 언어를 사용합니다.
명령어 이름, 옵션, API 응답, 토큰, 타임스탬프, 외부 오류의 세부 내용은 변경하지 않습니다.
clap의 인자 파싱 진단 메시지는 영어로 표시합니다. 생성되는 자동완성 스크립트는
선택한 언어와 관계없이 영어를 사용합니다. 명령어 참조 문서는 선택한 언어와
관계없이 영어와 한국어 두 버전으로 생성합니다.

에이전트나 CI에서 호출할 때는 `PORTONE_LANG=en`을 설정하면 사용자의 저장된
설정을 바꾸지 않고도 진단 메시지를 일관된 언어로 받을 수 있습니다. PowerShell에서는
CLI를 호출하기 전에 `$env:PORTONE_LANG = 'en'`으로 프로세스 환경 변수를 설정하세요.

## `portone setup`

공식 PortOne 스킬 4개를 복사하고, 하나 이상의 코딩 에이전트에 PortOne MCP
서버를 설정합니다.

```bash
portone setup --agent codex --scope project
portone setup --agent claude-code,cursor --scope user
portone setup --agent gemini-cli --agent opencode --scope project
```

`--agent`에는 쉼표로 구분한 값을 지정하거나 옵션을 반복해서 사용할 수 있습니다.
지원하는 ID는 `claude-code`, `codex`, `cursor`, `gemini-cli`,
`github-copilot`, `vscode-copilot`, `opencode`입니다.
`--scope`에는 `project` 또는 `user`를 지정합니다.
비대화형 실행에서는 두 옵션이 모두 필요합니다. 대화형 실행에서는 처음에 아무
에이전트도 선택되지 않은 다중 선택 목록을 표시한 뒤, 프로젝트 범위를 기본값으로 제안합니다.

이전 setup 명령어를 사용하는 스크립트를 위해 기존 옵션도 유지합니다.
이 옵션은 사용자 범위를 선택하며, 쉼표로 구분하거나 반복해서 지정할 수 있습니다.

```bash
portone setup --assistant claude
portone setup --assistant codex
portone setup --assistant both
```

setup은 `portone-cli`, `portone-guide`, `payment-code-generator`,
`integration-validator`를 복사합니다. 또한
`npx -y @portone/mcp-server@latest`를 실행하는 `portone` MCP 서버를 추가합니다.
변경 사항을 적용하기 전에 Node.js와 `npx`를 설치하세요.
Git과 선택한 에이전트의 CLI는 필요하지 않습니다. 미리보기(`--dry-run`)에는
Node.js나 `npx`가 필요하지 않습니다.

이 명령어는 GitHub의 공식 최신 릴리스에 해당하는 커밋 하나를 확정하고,
모든 스킬과 MCP 파일을 해당 커밋에서 가져옵니다. 최신 릴리스가 없을 때만 저장소의
기본 브랜치를 사용합니다. 최신 릴리스가 있지만 4개의 스킬 원본 디렉터리가
추가되기 전 릴리스라면 setup은 실패합니다. 릴리스 파일과 기본 브랜치 파일을
섞어서 사용하지 않습니다.

GitHub 요청에는 `GH_TOKEN`, `GITHUB_TOKEN` 순서로 토큰을 사용할 수 있습니다.
setup 파일을 다운로드할 때 PortOne API 인증 정보는 사용하지 않습니다.

### 설치 위치

프로젝트 범위로 설치하면 다음 위치에 스킬과 MCP 설정을 저장합니다.

| 에이전트 | 스킬 | MCP 설정 |
| --- | --- | --- |
| Claude Code | `.claude/skills` | `.mcp.json` |
| Codex | `.agents/skills` | `.codex/config.toml` |
| Cursor | `.agents/skills` | `.cursor/mcp.json` |
| Gemini CLI | `.agents/skills` | `.gemini/settings.json` |
| GitHub Copilot CLI | `.agents/skills` | `.mcp.json` |
| VS Code Copilot | `.agents/skills` | `.vscode/mcp.json` |
| OpenCode | `.agents/skills` | `opencode.jsonc`가 있으면 해당 파일, 없으면 `opencode.json` |

사용자 범위로 설치하면 각 에이전트의 표준 설정 디렉터리를 사용합니다.
Codex는 `~/.agents/skills`를 공유하며, Claude Code의 기본값은
`~/.claude/skills`, Cursor는 `~/.cursor/skills`, Gemini CLI는
`~/.gemini/skills`, Copilot 도구는 `~/.copilot/skills`입니다.
OpenCode는 항상 `${XDG_CONFIG_HOME:-~/.config}/opencode` 아래에 설치합니다.
`OPENCODE_CONFIG`와 `OPENCODE_CONFIG_DIR` 설정은 이 설치 루트를 바꾸지 않습니다.
해당하는 경우 `CODEX_HOME`, `CLAUDE_CONFIG_DIR`, `GEMINI_CLI_HOME`,
`COPILOT_HOME`, `XDG_CONFIG_HOME` 등의 환경 변수 설정을 따릅니다.
VS Code 설정은 stable 버전의 기본 프로필만 대상으로 합니다.

설치 위치를 공유하는 에이전트에는 사본 하나만 저장합니다. setup은 관련 없는 MCP
서버, 설정, 지원되는 JSONC/TOML 주석을 보존합니다. setup이나 update를 명시적으로
실행하면 관리 대상인 스킬 디렉터리 4개와 `portone` MCP 항목을 선택한 공식 리비전으로
교체합니다.

### 업데이트와 설치 기록

setup은 프로젝트 설치 내역을 `.portone/setup.json`에 기록합니다.
사용자 설치 내역은 운영체제별 PortOne 설정 디렉터리의 `setup.json`에 기록하며,
`PORTONE_CONFIG_DIR`로 이 디렉터리를 바꿀 수 있습니다.
설치 기록을 사용하므로 나중에 업데이트할 때 디스크의 파일에서 설치 위치를 추측하지
않고 같은 경로를 대상으로 삼을 수 있습니다.

```bash
portone setup update
portone setup update --agent codex,cursor
portone setup update --scope project
portone setup update --dry-run
```

필터 없이 `portone setup update`를 실행하면 현재 프로젝트와 사용자 범위의
설치 기록을 모두 확인합니다. `--agent`와 `--scope`로 기록된 대상을 좁힐 수 있습니다.
업데이트는 명시적으로 실행해야 하며, 설치된 CLI 버전과 독립적입니다.
백그라운드에서 자동으로 실행되지 않습니다.

setup 완료 메시지는 파일 설정이 끝났음을 의미합니다. 선택한 에이전트를 다시 시작하고,
MCP 서버 목록을 확인한 뒤 PortOne 문서를 가져오도록 요청하여 MCP 프로세스가 실제로
시작되는지 확인하세요. 호스트에서 작업 공간 신뢰 또는 MCP 승인을 요청하면 해당
안내를 따르세요. setup이 이러한 승인을 대신 부여하지는 않습니다.
기존 PortOne 플러그인은 자동으로 제거하지 않습니다. 설치한 스킬이나 서버와
중복된다면 호스트에서 해당 플러그인을 비활성화하세요.

### 번들 스킬 유지보수

소스 저장소에서는 `skills/` 아래의 원본 디렉터리를 수정하고,
생성된 플러그인 사본을 동기화합니다.

```bash
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

생성된 사본도 원본 변경 사항과 함께 커밋하세요. CI는 누락되거나 내용이 달라진
생성 파일과 더 이상 필요하지 않은 생성 파일을 검사합니다. setup에서 사용하는
릴리스에는 스킬 원본 디렉터리 4개가 모두 포함되어야 합니다.

## `portone auth`

인증 정보와 프로필을 관리합니다. 내장 인증은 PortOne 콘솔 OAuth를 사용하며,
REST와 GraphQL 요청 모두에 `Authorization: Bearer <token>`을 전송합니다.

```bash
portone auth login                # 브라우저에서 PortOne 콘솔을 통해 인증
portone auth login --profile staging --base-url <URL>
portone auth login --no-browser --scopes TX_READ,STORE_READ
portone auth status               # 인증 정보의 출처, 만료 시각, 권한 범위, 유효성 표시
portone auth status --show-secret # 액세스 토큰을 가리지 않고 표시
portone auth token                # 현재 액세스 토큰 출력, 필요한 경우 갱신
portone auth logout               # 서버의 토큰을 취소하지 않고 로컬 인증 정보 삭제
```

`login`은 발급된 토큰을 검증한 뒤 저장합니다. 처음 로그인할 때는 고객사의 대표 상점을
프로필의 기본 상점으로 선택하고 상점 이름과 ID를 출력합니다. 다시 인증할 때는 이전에
선택한 기본 상점에 계속 접근할 수 있으면 해당 설정을 유지합니다. 사용할 수 있는
대표 상점이 없고 접근 가능한 상점이 하나뿐이면 자동으로 선택합니다. 여러 상점이
있으면 대화형으로 선택하거나 선택을 건너뛸 수 있습니다.
상점 조회에 실패해도 로그인은 계속 진행합니다.

### 콘솔 로그인

`auth login`은 `127.0.0.1:1271`에서 콜백 서버를 시작하고 PortOne 콘솔
로그인 페이지를 엽니다. 브라우저를 선택하려면 `PORTONE_BROWSER` 또는
`BROWSER`를 설정하세요. `--no-browser`를 지정하면 브라우저를 열지 않고 URL을
출력합니다. 명령어는 콜백 코드를 토큰으로 교환하고, GraphQL을 통해 토큰을 검증한 뒤
인증 정보를 저장합니다. 5분 안에 콜백이 도착하지 않으면 중단합니다.

- `portone api`, `auth status`, `auth token`은 액세스 토큰 만료 60초 전부터
  토큰을 갱신합니다. 프로세스 간 잠금을 사용하여 같은 기기에서 갱신이 동시에
  실행되지 않도록 합니다.
- 리프레시 토큰은 사용할 때마다 교체되며, 24시간 동안 사용하지 않으면 만료됩니다.
  세션이 만료되면 `portone auth login`을 다시 실행하세요.
- 기기마다 별도로 로그인하세요. 프로필을 다른 기기로 복사하면 한 기기에서
  리프레시 토큰이 교체될 때 다른 기기의 세션이 무효화될 수 있습니다.
- 토큰은 OS 키링(macOS 키체인, Windows 자격 증명 관리자, Linux Secret Service)에
  `portone-cli/<credential_id>`로 저장합니다. 키링을 사용할 수 없으면 경고를 표시하고
  설정 파일에 저장합니다. 파일 저장을 명시적으로 선택하려면 `--insecure-storage`를
  사용하세요.
- 프로필은 토큰을 발급한 환경의 콘솔 URL, 토큰 엔드포인트, API 기본 URL을 보관하여
  이후 명령어에서도 같은 환경을 사용합니다.

`auth token`으로 PortOne MCP 서버 같은 다른 도구에 토큰을 전달할 수 있습니다.

```bash
PORTONE_ACCESS_TOKEN=$(portone auth token) npx @portone/mcp-server
```

### 인증 정보 우선순위

1. `PORTONE_ACCESS_TOKEN`: 지정한 값을 그대로 사용하며 CLI에서 갱신하지 않습니다.
2. 설정 파일의 OAuth 프로필: `--profile`, `default_profile`, `default` 순서로 선택합니다.

`PORTONE_ACCESS_TOKEN`이 설정되어 있으면 `auth login`과 `auth logout`은
실행되지 않습니다. 저장된 인증 정보를 변경하기 전에 환경 변수 설정을 해제하세요.

API 기본 URL은 인증 정보와 별도로 `--base-url`, `PORTONE_API_BASE`,
선택한 프로필의 `base_url`, `https://api.portone.io` 순서로 결정합니다.

### 설정

설정 파일은 유닉스 계열 시스템에서 `~/.config/portone/config.toml`,
Windows에서 `%APPDATA%\portone\config.toml`에 저장합니다.
다른 디렉터리를 사용하려면 `PORTONE_CONFIG_DIR`를 설정하세요.
파일에는 민감한 인증 메타데이터가 포함되며, 소유자만 접근할 수 있는 권한(`0600`)으로
저장합니다.

프로필을 사용하면 고객사나 환경별로 인증 정보를 분리할 수 있습니다.
`--profile <NAME>`으로 사용할 프로필을 선택하세요.

```toml
default_profile = "default"

[profiles.default]
base_url = "https://api.portone.io"
store_id = "store-xxx"

[profiles.default.oauth]
storage = "keyring"          # 토큰을 portone-cli/<credential_id>로 저장
credential_id = "..."
client_id = "CLI"
token_url = "https://merchant-service.prod.iamport.co/oauth/token"
console_url = "https://admin.portone.io"
```

로그인 환경은 `PORTONE_CONSOLE_URL`, `PORTONE_MERCHANT_SERVICE_URL`,
`PORTONE_OAUTH_CLIENT_ID`, `PORTONE_OAUTH_REDIRECT_URI`로 재정의할 수 있습니다.
이 환경 변수는 로그인에만 영향을 줍니다.

## `portone store`

프로필에 저장된 기본 상점을 관리합니다.

```bash
portone store set-default                    # 접근 가능한 상점 선택
portone store set-default store-xxx          # ID를 직접 저장
portone store set-default --profile staging --view
portone store set-default --unset
```

선택 목록에는 상점 이름과 ID가 표시되며 대표 상점이 가장 먼저 나옵니다.
이 명령어는 CLI 프로필만 변경하며, PortOne에 설정된 대표 상점은 바꾸지 않습니다.
`--view`는 환경 변수의 영향을 받지 않고 저장된 값을 표시합니다.

## `portone payment`

결제를 검색하고, 실패와 결제 시도 내역을 조회하고, 결제를 취소하고,
웹훅을 조회하거나 재발송합니다.

```bash
portone payment list --test --status failed --limit 20
portone payment view payment-xxx
portone payment transactions payment-xxx
portone payment webhook list payment-xxx
portone payment cancel payment-xxx --reason 'Customer request'
portone payment cancel payment-xxx --amount 1000 --reason 'Partial refund' --yes
portone payment webhook resend payment-xxx --webhook-id webhook-xxx
```

결제 ID는 연동 시 고객사가 지정한 ID입니다. PortOne 또는 PG사 거래 ID로 결제를
찾으려면 `list --search TEXT`를 사용하세요.

### 상점 및 검색 기본값

상점은 `--store`, `PORTONE_STORE_ID`, 프로필의 `store_id`, API 기본값
순서로 선택합니다. `payment list --all-stores`는 환경 변수와 프로필의 기본값을
무시하고 상점 필터를 생략합니다. `--store`와 함께 사용할 수 없으며,
조회할 수 있는 결과 범위는 토큰에 따라 달라집니다.

`payment list`는 기본적으로 지난 90일 이내에 변경된 V2 결제 중 최신 30건을
조회하며, 테스트 결제와 실결제를 모두 포함합니다. `--limit/-L`로 최종 조회 건수를
1~60,000건으로 지정할 수 있으며, 필요한 페이지는 자동으로 가져옵니다.

```bash
portone payment list --live --status paid,partial-cancelled --currency KRW
portone payment list --method card --pg tosspayments --version all
portone payment list --from 2026-09-01T00:00:00+09:00 --until 2026-09-08T00:00:00+09:00
portone payment list --search payment-xxx --search-field payment-id
```

시간 필터의 기준은 `--time-field created-at|status-changed-at`으로 선택하고,
정렬 방식은 `--sort requested-at|status-changed-at --order asc|desc`로 지정합니다.
`--status`, `--method`, `--pg`에는 쉼표로 구분한 값을 지정하거나 옵션을 반복해서
사용할 수 있습니다. `transactions`는 결제 시도 내역을 표시하며 불안정 API를 사용합니다.

### 구조화된 출력

```bash
portone payment view payment-xxx --json
portone payment list --json id,status
portone payment list --json --jq '.[] | .id'
```

`--json`은 전체 API 객체를 출력하고, `--json id,status`는 최상위 필드를
선택합니다. `--jq/-q`를 사용하려면 `--json`이 필요합니다. 목록은 배열로,
상세 조회·취소·웹훅 재발송 결과는 객체로 출력합니다. JSON은 API의 필드 이름,
상태 값, 정수 금액을 그대로 유지합니다. 조회 결과가 없으면 빈 배열(`[]`)을 출력하고
성공으로 종료합니다.

`--json`을 지정하지 않으면 터미널에 읽기 쉬운 표와 상세 화면을 표시합니다.
TTY가 아닌 환경에서 목록은 머리글 없는 TSV로 출력합니다.
금액은 최소 통화 단위의 정수입니다.

### 취소 및 웹훅 결과

`cancel`에는 취소 사유가 필요하며, 대화형으로 결제와 취소 요청 내용을 확인합니다.
확인을 생략하려면 `--yes`를 지정하세요. TTY가 없는 환경에서는 이 옵션이 필수입니다.
`--amount`를 생략하면 남은 금액 전체를 취소합니다.
`--tax-free-amount`, `--vat-amount`, `--current-cancellable-amount`에도
최소 통화 단위의 정수를 지정합니다.

환불 계좌 등 복잡한 필드는 `--input cancel.json` 또는 `--input -`로 완전한
JSON 본문을 전달하세요. 본문에는 취소 사유가 포함되어야 하며,
개별 취소 필드 옵션과 함께 사용할 수 없습니다. 본문의 `storeId`는 기본 상점보다
우선하지만, `--store`를 명시했다면 그 값과 일치해야 합니다.

취소 결과 `REQUESTED`는 요청이 접수되었음을, `SUCCEEDED`는 취소가
완료되었음을 의미합니다. 둘 다 종료 코드 0으로 종료합니다.
`FAILED`는 종료 코드 1로 종료합니다. 취소는 자동으로 재시도하지 않습니다.

웹훅 재발송은 추가 확인 없이 실행됩니다. `--webhook-id`를 생략하면 API가
가장 최근 웹훅을 선택합니다. 재발송 요청 성공과 실제 전송 성공은 구분되며,
전송 실패가 보고되면 종료 코드 1로 종료합니다. 웹훅 요청과 응답의 세부 내용은
`payment webhook list --json`으로 확인할 수 있습니다.

## `portone api`

인증 정보를 사용하여 PortOne V2 API 요청을 보냅니다.

```bash
portone api <endpoint> [flags]
```

`<endpoint>`에는 `/payments/{paymentId}`와 같은 경로(자리표시자는 실제 값으로
바꾸세요), 전체 URL 또는 `graphql`을 지정할 수 있습니다. 기본 메서드는 GET이며,
요청 필드나 `--input`으로 본문을 제공하면 POST로 바뀝니다. `-X`로 메서드를
지정할 수 있습니다. `--paginate`를 사용하면 REST 요청은 GET,
GraphQL 요청은 POST를 사용합니다.

모든 옵션은 [명령어 참조](https://github.com/portone-io/portone-cli/blob/main/docs/reference/ko/portone_api.md)에서
확인할 수 있습니다. `--jq`, `--silent`, `--verbose` 중 하나만 사용할 수 있습니다.

### 예제

결제 한 건 조회:

```bash
portone api /payments/{paymentId}
```

PortOne V2 목록 엔드포인트에서 사용하는 GET 요청 본문으로 필터 전달:

```bash
portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'
```

`key[sub]=value`는 중첩 객체를 만들고, `key[]=value` 필드를 반복하면
배열을 만듭니다. `-F`는 정수, `true`, `false`, `null`을 JSON 자료형으로
변환하며, `@file`이나 `@-`로 파일 또는 표준 입력의 값을 읽을 수 있습니다.
`-f`는 항상 문자열을 전송합니다.

모든 페이지를 가져와 결제 ID 출력:

```bash
portone api /payments -X GET --paginate -q '.items[].id'
```

`--slurp`는 모든 페이지를 하나의 JSON 배열로 묶습니다.
`--jq`와 함께 사용할 수 없습니다.

```bash
portone api /payments -X GET --paginate --slurp
```

파일 또는 표준 입력에서 요청 본문 읽기:

```bash
portone api /payments/{paymentId}/cancel --input cancel.json
echo '{"reason":"Customer request"}' | portone api /payments/{paymentId}/cancel --input -
```

캐시 가능한 GET, HEAD, GraphQL 응답을 지정한 유효 기간(TTL) 동안 캐시합니다.
상태 코드가 403 또는 5xx인 응답은 캐시하지 않습니다. 기본 캐시 디렉터리는
`~/.cache/portone`이며, `PORTONE_CACHE_DIR`로 바꿀 수 있습니다.

```bash
portone api /payments/{paymentId} --cache 1h
```

`-H`로 `Authorization`을 명시하면 저장된 인증 정보보다 우선합니다.
전체 엔드포인트 URL의 출처(origin)가 설정된 기본 URL과 다르면 CLI는
Authorization 헤더를 보내지 않습니다.

```bash
portone api /payments/{paymentId} -H 'Idempotency-Key: abc123'
portone api '/identity-verifications/{identityVerificationId}?storeId=store-xxx'
```

### GraphQL

엔드포인트로 `graphql`을 지정하면 `{base URL}/graphql`에 요청합니다.
`query`와 `operationName`을 제외한 모든 필드는 GraphQL 변수로 전송합니다.

```bash
portone api graphql -f query='query { merchant { ... on Merchant { id plainId } } }'

portone api graphql \
  -f query='query($id: ID!) { node(id: $id) { ... on Merchant { plainId } } }' \
  -f id='MDptZXJjaGFudC...'
```

응답에 `errors` 배열이 있으면 HTTP 상태가 200이어도 종료 코드 1로 종료합니다.
원본 JSON은 표준 출력(stdout)에, 오류 메시지는 표준 오류(stderr)에 출력합니다.

GraphQL 페이지네이션에는 `$endCursor: String` 변수와
`pageInfo { hasNextPage endCursor }` 선택 항목이 필요합니다.
객체 변수에는 중첩 필드 구문을 사용하세요.

```bash
portone api graphql --paginate --slurp \
  -f storeId='<store-global-id>' \
  -F 'filter[statuses][]=IN_PROGRESS' -F 'filter[cardCompanies][]' \
  -f query='
  query($storeId: ID!, $filter: PromotionFilterInput!, $endCursor: String) {
    node(id: $storeId) {
      ... on Store {
        promotions(filter: $filter, first: 50, after: $endCursor) {
          edges { node { id name status } }
          pageInfo { hasNextPage endCursor }
        }
      }
    }
  }'
```

### 내장 jq 지원

`-q`와 `--jq`는 내장된 [jaq](https://github.com/01mf02/jaq) 엔진을 사용하므로
jq를 별도로 설치할 필요가 없습니다. 대부분의 jq 구문과 내장 함수를 사용할 수 있지만,
일부 내장 함수는 지원하지 않거나 동작이 다를 수 있습니다. 복잡한 변환이 필요하면
출력을 파이프로 다른 도구에 전달할 수도 있습니다.

### 종료 코드

| 코드 | 의미 |
| --- | --- |
| 0 | 성공. 출력 파이프가 일찍 닫힌 경우도 포함 |
| 1 | HTTP 4xx/5xx, GraphQL 오류, 잘못된 옵션 조합 또는 기타 런타임 오류 |
| 2 | 명령줄 인자 파싱 오류 |

## `portone completion`

Bash, Zsh, Fish, PowerShell, Elvish용 자동완성 스크립트를 생성합니다.

```bash
# Zsh: $fpath에 포함된 디렉터리에 저장
portone completion zsh > "${fpath[1]}/_portone"

# Bash
portone completion bash > "$(brew --prefix)/etc/bash_completion.d/portone"

# Fish
portone completion fish > ~/.config/fish/completions/portone.fish
```

새 셸을 열면 `portone <TAB>` 자동완성을 사용할 수 있습니다.

## 명령어 참조

모든 명령어와 옵션의 자세한 설명은
[docs/reference/ko](https://github.com/portone-io/portone-cli/blob/main/docs/reference/ko/index.md)에서
확인할 수 있습니다. CLI 정의에서 자동으로 생성하며 CI에서 검증합니다.

소스 저장소에서 두 언어의 문서를 다시 생성하거나 최신 상태인지 확인하려면
다음 명령어를 실행하세요.

```bash
cargo xtask gen-docs
cargo xtask gen-docs --check
```

AI 에이전트가 `portone`을 호출하는 데 도움이 되는 사용 패턴은
[skills/portone-cli/SKILL.md](https://github.com/portone-io/portone-cli/blob/main/skills/portone-cli/SKILL.md)를
참고하세요.
