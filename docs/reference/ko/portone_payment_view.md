# portone payment view

[English](../portone_payment_view.md) | 한국어

결제 상세 조회

```
portone payment view [OPTIONS] --payment-id <PAYMENT_ID>
```

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `--payment-id <PAYMENT_ID>` | 고객사가 지정한 결제 ID |
| `--json [<FIELDS>]` | JSON 출력, 쉼표로 구분한 필드 선택 가능 |
| `-q, --jq <EXPR>` | jq 표현식으로 JSON 출력 필터링 (--json 필요) |
| `--profile <NAME>` | 사용할 설정 프로필 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--store <STORE_ID>` | 상점 ID (기본값: PORTONE_STORE_ID 또는 프로필 store_id) |

## 참고

- [portone payment](portone_payment.md)
