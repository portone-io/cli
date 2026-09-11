# portone auth login

[English](../portone_auth_login.md) | 한국어

PortOne 콘솔 로그인

```
portone auth login [OPTIONS]
```

## 옵션

| 옵션 | 설명 |
| --- | --- |
| `--profile <NAME>` | 인증 정보를 저장할 설정 프로필 |
| `--base-url <URL>` | API 요청의 기본 URL (기본값: https://api.portone.io) |
| `--scopes <SCOPES>` | 요청할 콘솔 권한 범위 (쉼표로 구분, 기본값: HOME_AND_REPORT,TX_READ,CHANNEL_READ,STORE_READ,MERCHANT_READ) |
| `--insecure-storage` | OS 키링 대신 설정 파일에 토큰 저장 |
| `--no-browser` | 브라우저를 열지 않고 로그인 URL 출력 |

## 참고

- [portone auth](portone_auth.md)
