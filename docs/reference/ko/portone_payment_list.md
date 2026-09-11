# portone payment list

[English](../portone_payment_list.md) | 한국어

최근 결제 목록 조회

```
portone payment list [OPTIONS]
```

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `-L, --limit <LIMIT>` | 조회할 최대 결제 건수 (1-60000) [기본값: 30] |
| `--status <STATUS>` | 결제 상태로 필터링 (반복 또는 쉼표로 구분) [가능한 값: ready, pending, virtual-account-issued, paid, failed, partial-cancelled, cancelled] |
| `--method <METHOD>` | 결제 수단으로 필터링 (반복 또는 쉼표로 구분) [가능한 값: card, transfer, virtual-account, gift-certificate, mobile, easy-pay, convenience-store, crypto] |
| `--pg <PG>` | PG사로 필터링 (반복 또는 쉼표로 구분) |
| `--currency <CURRENCY>` | KRW, USD 등의 통화 코드로 필터링 |
| `--test` | 테스트 결제만 조회 |
| `--live` | 실결제만 조회 |
| `--version <VERSION>` | PortOne 결제 버전으로 필터링 [기본값: v2] [가능한 값: v1, v2, all] |
| `--from <RFC3339>` | 조회 기간 시작 (기본값: --until 기준 90일 전) |
| `--until <RFC3339>` | 조회 기간 종료 (기본값: 현재 시각) |
| `--time-field <TIME_FIELD>` | --from 및 --until에 사용할 시각 필드 [기본값: status-changed-at] [가능한 값: created-at, status-changed-at] |
| `--sort <SORT>` | 결제 정렬에 사용할 필드 [기본값: status-changed-at] [가능한 값: requested-at, status-changed-at] |
| `--order <ORDER>` | 정렬 순서 [기본값: desc] [가능한 값: desc, asc] |
| `--search <TEXT>` | 결제 텍스트 검색 |
| `--search-field <SEARCH_FIELD>` | 검색할 결제 필드 [기본값: all] |
| `--all-stores` | 기본 상점 설정을 무시하고 접근 가능한 모든 상점 조회 |
| `--json [<FIELDS>]` | JSON 출력, 쉼표로 구분한 필드 선택 가능 |
| `-q, --jq <EXPR>` | jq 표현식으로 JSON 출력 필터링 (--json 필요) |
| `--profile <NAME>` | 사용할 설정 프로필 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--store <STORE_ID>` | 상점 ID (기본값: PORTONE_STORE_ID 또는 프로필 store_id) |

## 참고

- [portone payment](portone_payment.md)
