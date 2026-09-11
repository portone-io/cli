# portone payment webhook

[English](../portone_payment_webhook.md) | 한국어

결제 웹훅 조회 및 재발송

```
portone payment webhook [OPTIONS] <COMMAND>
```

## 명령어

| 명령어 | 설명 |
| --- | --- |
| [portone payment webhook list](portone_payment_webhook_list.md) | 결제 웹훅 목록 조회 |
| [portone payment webhook resend](portone_payment_webhook_resend.md) | 결제 웹훅 재발송 |

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `--profile <NAME>` | 사용할 설정 프로필 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--store <STORE_ID>` | 상점 ID (기본값: PORTONE_STORE_ID 또는 프로필 store_id) |

## 참고

- [portone payment](portone_payment.md)
