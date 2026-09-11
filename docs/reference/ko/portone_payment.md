# portone payment

[English](../portone_payment.md) | 한국어

결제 조회 및 관리

```
portone payment [OPTIONS] <COMMAND>
```

## 명령어

| 명령어 | 설명 |
| --- | --- |
| [portone payment list](portone_payment_list.md) | 최근 결제 목록 조회 |
| [portone payment view](portone_payment_view.md) | 결제 상세 조회 |
| [portone payment transactions](portone_payment_transactions.md) | 결제 시도 내역 조회 (불안정 API) |
| [portone payment cancel](portone_payment_cancel.md) | 결제 취소 |
| [portone payment webhook](portone_payment_webhook.md) | 결제 웹훅 조회 및 재발송 |

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `--profile <NAME>` | 사용할 설정 프로필 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--store <STORE_ID>` | 상점 ID (기본값: PORTONE_STORE_ID 또는 프로필 store_id) |

## 참고

- [portone](portone.md)
