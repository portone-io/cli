# portone payment cancel

[English](../portone_payment_cancel.md) | 한국어

결제 취소

```
portone payment cancel [OPTIONS] --payment-id <PAYMENT_ID>
```

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `--payment-id <PAYMENT_ID>` | 고객사가 지정한 결제 ID |
| `--reason <REASON>` | 결제 취소 사유 |
| `--amount <INTEGER>` | 최소 통화 단위의 취소 금액 (기본값: 남은 금액 전체) |
| `--tax-free-amount <INTEGER>` | 최소 통화 단위의 면세 취소 금액 |
| `--vat-amount <INTEGER>` | 최소 통화 단위의 부가세 취소 금액 |
| `--current-cancellable-amount <INTEGER>` | 최소 통화 단위의 예상 취소 가능 잔액 |
| `--input <FILE>` | 파일에서 취소 JSON 본문 읽기 (표준 입력은 -) |
| `-y, --yes` | 확인 생략 (비대화형 실행 시 필수) |
| `--json [<FIELDS>]` | JSON 출력, 쉼표로 구분한 필드 선택 가능 |
| `-q, --jq <EXPR>` | jq 표현식으로 JSON 출력 필터링 (--json 필요) |
| `--profile <NAME>` | 사용할 설정 프로필 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--store <STORE_ID>` | 상점 ID (기본값: PORTONE_STORE_ID 또는 프로필 store_id) |

## 참고

- [portone payment](portone_payment.md)
