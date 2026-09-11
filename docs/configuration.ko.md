# 설정

[English](configuration.md) | 한국어

## 인증

PortOne 콘솔에서 로그인합니다.

```sh
portone auth login
portone auth status
portone auth token   # 액세스 토큰 출력, 필요한 경우 갱신
portone auth logout  # 로컬 인증 정보 삭제
```

로그인은 브라우저를 열고 `127.0.0.1:1271`에서 최대 5분간 콜백을 기다립니다.
URL만 출력하려면 `--no-browser`를, 브라우저를 지정하려면 `PORTONE_BROWSER`
또는 `BROWSER`를 사용하세요.

토큰은 OS 키링에 저장합니다. 키링을 사용할 수 없으면 경고 후 설정 파일에 저장하며,
`--insecure-storage`로 파일 저장을 직접 선택할 수 있습니다.
액세스 토큰은 자동 갱신합니다. 리프레시 토큰은 사용할 때마다 교체되며,
24시간 동안 사용하지 않으면 만료됩니다. 기기마다 별도로 로그인하고,
세션이 만료되면 다시 로그인하세요. 로그아웃은 서버의 토큰을 취소하지 않고
로컬 인증 정보를 삭제합니다.

## 프로필과 상점

고객사나 환경별로 프로필을 사용합니다.

```sh
portone auth login --profile staging
portone payment list --profile staging
portone store set-default --profile staging
portone store set-default --profile staging --view
portone store set-default --profile staging --unset
```

로그인하면 대표 상점을 기본값으로 선택하며, 재로그인 시 접근 가능한 기존 선택을
유지합니다. 대표 상점이 없으면 접근 가능한 유일한 상점을 선택하거나,
여러 상점 중 하나를 선택하도록 안내합니다. 상점 선택은 CLI 프로필에만 적용됩니다.

설정 파일은 유닉스 계열 시스템의 `~/.config/portone/config.toml`,
Windows의 `%APPDATA%\portone\config.toml`에 저장합니다.
`PORTONE_CONFIG_DIR`로 디렉터리를 바꿀 수 있습니다.
인증 항목은 로그인 시 관리하며, 다른 설정은 다음과 같이 지정합니다.

```toml
language = "auto"
default_profile = "default"

[profiles.default]
base_url = "https://api.portone.io"
store_id = "store-xxx"
```

각 설정은 다음 순서에서 먼저 지정된 값을 사용합니다.

| 설정 | 우선순위 |
| --- | --- |
| 인증 정보 | `PORTONE_ACCESS_TOKEN`, 선택한 OAuth 프로필 |
| 프로필 | `--profile`, `default_profile`, `default` |
| API 기본 URL | `--base-url`, `PORTONE_API_BASE`, 프로필의 `base_url`, `https://api.portone.io` |
| 상점 | `--store`, `PORTONE_STORE_ID`, 프로필의 `store_id`, API 기본값 |

`PORTONE_ACCESS_TOKEN`은 지정한 값을 그대로 사용하며 갱신하지 않습니다.
`auth login`이나 `auth logout`을 실행하려면 이 환경 변수 설정을 해제하세요.
`payment list --all-stores`는 기본 상점을 무시하며 `--store`와 함께 사용할 수 없습니다.

로그인 환경을 바꾸려면 `PORTONE_CONSOLE_URL`, `PORTONE_MERCHANT_SERVICE_URL`,
`PORTONE_OAUTH_CLIENT_ID`, `PORTONE_OAUTH_REDIRECT_URI`를 사용하세요.
이 환경 변수는 로그인에만 적용됩니다.

## 표시 언어

영어와 한국어를 지원합니다. 한 번의 실행에는 `PORTONE_LANG`을 사용하고,
설정을 저장하려면 설정 파일에 `language = "en"`, `"ko"`, `"auto"`를 지정하세요.

```sh
PORTONE_LANG=en portone auth status
PORTONE_LANG=ko portone --help
```

`PORTONE_LANG`은 저장된 설정보다 우선하며, `auto`는 운영체제의 언어를 감지합니다.
macOS와 Windows에서는 UI 언어 설정을, Linux에서는 `LANGUAGE`, `LC_ALL`,
`LC_MESSAGES`, `LANG` 순서로 확인합니다. `ko-KR` 같은 지역별 로캘도 지원하며,
지원하는 언어가 없으면 영어를 사용합니다.

명령어 이름, 옵션, API 응답은 원래 값을 유지합니다. 인자 파싱 오류와 생성된
자동완성 스크립트는 영어를 사용합니다. CI나 에이전트에서 일정한 언어로 진단하려면
`PORTONE_LANG=en`을 설정하세요. PowerShell에서는 실행 전에
`$env:PORTONE_LANG = 'en'`을 사용합니다.

전체 옵션은 [명령어 참조](reference/ko/index.md)를 참고하세요.
