# 사용 예제

[English](usage.md) | 한국어

먼저 `portone auth login`으로 로그인하세요. 프로필과 인증 정보는
[설정](configuration.ko.md), 전체 옵션은 [명령어 참조](reference/ko/index.md)를
참고하세요.

## 결제

```sh
portone payment list --test --status failed --limit 20
portone payment view payment-xxx
portone payment transactions payment-xxx
portone payment webhook list payment-xxx
```

`payment-xxx`를 연동 시 지정한 결제 ID로 바꾸세요. PortOne 또는 PG사 거래 ID로
찾으려면 `list --search TEXT`를 사용합니다. `transactions`는 불안정 API를 사용합니다.

`list`는 기본적으로 90일 이내 변경된 V2 결제 중 최신 30건을 조회하며,
테스트 결제와 실결제를 모두 포함합니다. `--limit`(1~60,000)까지 필요한 페이지를
자동으로 가져옵니다. 필터로 결과를 좁힐 수 있습니다.

```sh
portone payment list --live --status paid,partial-cancelled --currency KRW
portone payment list --method card --pg tosspayments --version all
portone payment list --from 2026-09-01T00:00:00+09:00 --until 2026-09-08T00:00:00+09:00
```

스크립트에서는 JSON 출력과 내장된 jq 호환 필터를 사용하세요.

```sh
portone payment view payment-xxx --json
portone payment list --json id,status
portone payment list --json --jq '.[] | .id'
```

목록은 배열, 다른 결과는 객체로 출력하며 API의 필드 이름과 값을 유지합니다.
빈 목록은 `[]`를 출력하고 성공으로 종료합니다. `--json`이 없으면 터미널에서는
표로, 파이프로 전달할 때는 머리글 없는 TSV로 출력합니다. 금액은 최소 통화 단위의 정수입니다.

## 취소와 웹훅

```sh
portone payment cancel payment-xxx --reason 'Customer request'
portone payment cancel payment-xxx --amount 1000 --reason 'Partial refund' --yes
portone payment cancel payment-xxx --input cancel.json
portone payment webhook resend payment-xxx --webhook-id webhook-xxx
```

취소에는 사유가 필요하며 실행 전 확인을 요청합니다. `--yes`는 확인을 생략하며
TTY가 없으면 필수입니다. `--amount`를 생략하면 남은 금액 전체를 취소합니다.
모든 금액 필드는 최소 통화 단위의 정수를 사용합니다.

환불 계좌 등이 필요하면 `--input FILE` 또는 `--input -`로 사유를 포함한 완전한
JSON 본문을 전달하세요. 개별 취소 필드 옵션과 함께 사용할 수 없습니다.
본문의 `storeId`는 기본 상점보다 우선하지만, 명시한 `--store`와는 일치해야 합니다.

`REQUESTED`는 취소 접수, `SUCCEEDED`는 취소 완료이며 둘 다 종료 코드 0을 반환합니다.
`FAILED`는 종료 코드 1을 반환합니다. 취소는 자동으로 재시도하지 않습니다.

웹훅 재발송은 추가 확인 없이 실행하며, `--webhook-id`를 생략하면 최신 웹훅을 선택합니다.
요청 성공이 전송 성공을 보장하지는 않으며, 전송 실패가 보고되면 종료 코드 1을 반환합니다.
요청·응답 세부 내용은 `payment webhook list --json`으로 확인하세요.

## API 요청

REST와 GraphQL 요청에는 `portone api`를 사용합니다.

```sh
portone api /payments/payment-xxx
portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'
portone api /payments -X GET --paginate -q '.items[].id'
portone api graphql -f query='query { merchant { ... on Merchant { id plainId } } }'
```

기본 메서드는 GET이며, 필드나 `--input`을 추가하면 POST로 바뀝니다.
V2 목록 엔드포인트에 필터를 전달할 때는 `-X GET`을 지정하세요.
`-f`는 문자열을 전송하며, `-F`는 정수·불리언·null을 변환하고
`@path`로 파일, `@-`로 표준 입력을 읽습니다.

`--paginate`는 모든 페이지를 가져옵니다. `--slurp`로 배열에 모으거나
`--jq`로 필터링할 수 있습니다. 내장 jaq 엔진이 대부분의 jq 구문을 지원하므로
jq를 별도로 설치하지 않아도 됩니다.

`--cache 1h`는 HTTP 403·5xx를 제외한 캐시 가능한 응답을 한 시간 동안 저장합니다.
기본 경로는 `~/.cache/portone`이며 `PORTONE_CACHE_DIR`로 바꿀 수 있습니다.
직접 지정한 `Authorization` 헤더는 저장된 인증 정보보다 우선합니다.
출처(origin)가 다른 전체 URL에는 인증 정보를 자동으로 보내지 않습니다.

중첩 필드, 요청 본문, GraphQL 변수, 페이지네이션 예제는
[`portone api`](reference/ko/portone_api.md)를 참고하세요.

| 종료 코드 | 의미 |
| --- | --- |
| 0 | 성공. 출력 파이프가 일찍 닫힌 경우도 포함 |
| 1 | HTTP 4xx/5xx, GraphQL 오류, 잘못된 옵션 조합 또는 런타임 오류 |
| 2 | 명령줄 인자 파싱 오류 |

## 셸 자동완성

Bash, Zsh, Fish, PowerShell, Elvish용 스크립트를 생성합니다.

```sh
portone completion zsh > "${fpath[1]}/_portone"
portone completion bash > "$(brew --prefix)/etc/bash_completion.d/portone"
portone completion fish > ~/.config/fish/completions/portone.fish
```

사용하는 셸에 맞는 명령을 실행하세요. Bash 예제는 Homebrew의 자동완성 디렉터리를
사용합니다. 새 셸을 열면 `portone <TAB>` 자동완성을 사용할 수 있습니다.
