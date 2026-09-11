# portone api

[English](../portone_api.md) | 한국어

인증 정보를 사용하여 PortOne V2 API에 HTTP 요청을 보내고 응답을 출력합니다.

`<ENDPOINT>` 인자에는 `/payments/{paymentId}` 같은 REST 경로
(자리표시자를 실제 값으로 바꾸세요), 전체 URL 또는 GraphQL API를 위한
`graphql`을 사용할 수 있습니다. 경로는 `--base-url`에 추가되며, 기본값은
`https://api.portone.io`입니다. 전체 URL은 그대로 사용합니다. 전체 URL의
출처(origin)가 기본 URL과 다르면 Authorization 헤더를 보내지 않습니다.

기본 HTTP 메서드는 `GET`이며, 필드나 `--input`으로 요청 본문을 제공하면
`POST`를 사용합니다. `--method`로 메서드를 지정할 수 있습니다. PortOne V2
목록 엔드포인트는 GET 요청 본문으로 필터를 받으므로, 필드를 보내려면
`-X GET`을 함께 지정하세요.

`-f/--raw-field`에 `key=value` 형식의 값을 전달하면 문자열 필드를 추가합니다.
`-F/--field`는 값에 따라 자료형을 변환합니다.

- `true`, `false`, `null` 및 정수는 JSON 자료형으로 변환합니다.
- `@`로 시작하는 값은 나머지 경로의 파일에서 읽고, `@-`는 표준 입력에서
  읽습니다.

중첩 값에는 `key[subkey]=value`를, 배열에는 반복되는 `key[]=value` 필드를,
빈 배열에는 값 없이 `key[]`를 사용합니다.

GraphQL 요청에서는 `query`와 `operationName`을 제외한 모든 필드를
GraphQL 변수로 보냅니다. 응답에 `errors` 배열이 있으면 HTTP 상태가
200이어도 종료 코드 1을 반환합니다.

미리 작성한 본문은 `--input <FILE>`로 전달하거나, `-`로 표준 입력에서
읽을 수 있습니다. `--input`은 필드 옵션 또는 `--paginate`와 함께 사용할 수 없습니다.

`--paginate`를 사용하면 더 이상 페이지가 없을 때까지 요청을 계속합니다.
REST 페이지 이동은 오프셋 방식의 `page.totalCount` 또는 커서 방식의
`items[].cursor`를 사용합니다. `page` 필드가 없으면 오프셋 페이지 이동은
`number=0, size=100`에서 시작합니다. GraphQL 페이지 이동에는
`$endCursor: String` 변수와 `pageInfo { hasNextPage endCursor }` 선택이
필요합니다. 각 페이지를 별도의 JSON 값으로 출력하며, `--slurp`를 사용하면
모든 페이지를 하나의 배열로 묶습니다.

`-q/--jq`는 내장된 jaq 엔진을 사용하며 `--slurp`와 함께 사용할 수 없습니다.
`--jq`, `--silent`, `--verbose` 중 하나만 사용할 수 있습니다.

환경 변수:

- `PORTONE_ACCESS_TOKEN`: 콘솔 액세스 토큰 (프로필보다 우선하며 갱신하지 않음)
- `PORTONE_API_BASE`: API 기본 URL (`--base-url` > 환경 변수 > 프로필 > 기본값)
- `PORTONE_CONFIG_DIR`: 설정 디렉터리
- `PORTONE_CACHE_DIR`: `--cache`에서 사용하는 응답 캐시 디렉터리
- `PORTONE_PAGER`, `PAGER`: TTY 출력용 페이저 (`cat` 또는 빈 값으로 비활성화)
- `NO_COLOR`, `CLICOLOR_FORCE`: 색상 출력 제어

```
portone api [OPTIONS] <ENDPOINT>
```

## 인자

| 인자 | 설명 |
| --- | --- |
| `<ENDPOINT>` | 엔드포인트 경로, 전체 URL 또는 GraphQL API를 위한 graphql |

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `-X, --method <METHOD>` | 요청의 HTTP 메서드 (기본값: GET, 필드가 있으면 POST) |
| `-F, --field <key=value>` | key=value 형식으로 자료형을 변환하는 요청 필드 추가 (@path, @-, 정수, true, false, null 지원) |
| `-f, --raw-field <key=value>` | key=value 형식으로 문자열 요청 필드 추가 |
| `-H, --header <key:value>` | key:value 형식으로 HTTP 요청 헤더 추가 |
| `--input <FILE>` | 요청 본문으로 사용할 파일 (표준 입력은 "-" 사용) |
| `-i, --include` | 출력에 응답 상태 줄과 헤더 포함 |
| `--paginate` | 결과의 모든 페이지를 가져오도록 추가 요청 전송 |
| `--slurp` | 모든 페이지의 JSON 값을 하나의 배열로 출력 (--paginate 필요) |
| `-q, --jq <EXPR>` | jq 문법으로 응답 조회 |
| `--cache <TTL>` | 3600s, 60m, 1h 등 지정한 시간 동안 응답 캐시 |
| `--silent` | 응답 본문 출력 생략 |
| `--verbose` | 출력에 전체 HTTP 요청과 응답 포함 |
| `--allow-escape-sequences` | 터미널 이스케이프 시퀀스 출력 허용 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--profile <NAME>` | 사용할 설정 프로필 |

## 예제

```sh
# 결제 조회 (자리표시자를 실제 ID로 바꾸세요)
$ portone api /payments/{paymentId}

# 경로에 쿼리 매개변수 직접 추가
$ portone api '/payments/{paymentId}?storeId=store-xxx'

# GET 요청 본문에 필터를 넣어 결제 목록 조회
$ portone api /payments -X GET -F 'page[size]=10' -F 'filter[isTest]=true'

# 배열 필드 전달
$ portone api /payments -X GET \
  -F 'filter[methods][]=CARD' -F 'filter[methods][]=EASY_PAY'

# 결제 취소 (필드가 있으면 요청이 POST로 전환됨)
$ portone api /payments/{paymentId}/cancel -f reason='Customer request'

# 파일 또는 표준 입력에서 JSON 요청 본문 읽기
$ portone api /payments/{paymentId}/cancel --input cancel.json
$ echo '{"reason":"Customer request"}' | portone api /payments/{paymentId}/cancel --input -

# 사용자 지정 헤더 추가
$ portone api /payments/{paymentId}/cancel -f reason=duplicate-payment \
  -H 'Idempotency-Key: abc123'

# 모든 페이지를 가져와 결제 ID 출력
$ portone api /payments -X GET --paginate -q '.items[].id'

# 커서 방식 엔드포인트의 모든 페이지를 하나의 JSON 배열로 출력
$ portone api /payments-by-cursor -X GET --paginate --slurp

# 응답을 한 시간 동안 캐시
$ portone api /payments/{paymentId} --cache 1h

# GraphQL 쿼리 전송
$ portone api graphql \
  -f query='query { merchant { ... on Merchant { id plainId } } }'

# GraphQL 변수 전달 (query를 제외한 모든 필드가 변수로 전달됨)
$ portone api graphql -f id='<merchant-global-id>' -f query='
  query($id: ID!) { node(id: $id) { ... on Merchant { plainId } } }
'

# GraphQL 페이지 이동 및 중첩 필드로 객체 변수 구성
$ portone api graphql --paginate --slurp \
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

## 참고

- [portone](portone.md)
