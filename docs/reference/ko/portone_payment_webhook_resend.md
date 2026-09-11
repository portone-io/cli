# portone payment webhook resend

[English](../portone_payment_webhook_resend.md) | 한국어

결제 웹훅 재발송

```
portone payment webhook resend [OPTIONS] <PAYMENT_ID>
```

## 인자

| 인자 | 설명 |
| --- | --- |
| `<PAYMENT_ID>` | 고객사가 지정한 결제 ID |

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `--webhook-id <WEBHOOK_ID>` | 재발송할 웹훅 (기본값: 가장 최근 웹훅) |
| `--json [<FIELDS>]` | JSON 출력, 쉼표로 구분한 필드 선택 가능 |
| `-q, --jq <EXPR>` | jq 표현식으로 JSON 출력 필터링 (--json 필요) |
| `--profile <NAME>` | 사용할 설정 프로필 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--store <STORE_ID>` | 상점 ID (기본값: PORTONE_STORE_ID 또는 프로필 store_id) |

## 참고

- [portone payment webhook](portone_payment_webhook.md)
