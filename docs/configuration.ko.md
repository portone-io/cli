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

로그인 URL만 출력하려면 `--no-browser`를, 특정 브라우저를 지정하려면 `PORTONE_BROWSER`
또는 `BROWSER` 환경변수를 사용하세요.

토큰은 OS 키링에 저장됩니다. OS 키링을 사용할 수 없으면 경고 후 설정 파일에 저장하며,
`--insecure-storage`로 파일 저장을 직접 선택할 수 있습니다.

## 프로필과 상점

고객사나 환경별로 프로필을 사용합니다.

```sh
portone auth login --profile staging
portone payment list --profile staging
portone store set-default --profile staging
portone store set-default --profile staging --view
portone store set-default --profile staging --unset
```

설정 파일은 유닉스 계열 시스템의 `~/.config/portone/config.toml`,
Windows의 `%APPDATA%\portone\config.toml`에 저장됩니다.
`PORTONE_CONFIG_DIR`로 디렉터리를 바꿀 수 있습니다.

```toml
language = "auto"
default_profile = "default"

[profiles.default]
base_url = "https://api.portone.io"
store_id = "store-xxx"
```

각 설정은 아래 우선순위를 따릅니다.

| 설정 | 우선순위 |
| --- | --- |
| 인증 정보 | `PORTONE_ACCESS_TOKEN`, 프로필 |
| 프로필 | `--profile`, `default_profile`, `default` |
| API 기본 URL | `--base-url`, `PORTONE_API_BASE`, 프로필의 `base_url`, `https://api.portone.io` |
| 상점 | `--store`, `PORTONE_STORE_ID`, 프로필의 `store_id`, API 기본값 |

`PORTONE_ACCESS_TOKEN` 환경변수가 지정되면 로그인 정보가 무시합니다.
`auth login`이나 `auth logout`을 실행하려면 이 환경 변수 설정을 해제하세요.
`payment list --all-stores`는 기본 상점을 무시하며 `--store`와 함께 사용할 수 없습니다.

## 표시 언어

영어와 한국어를 지원합니다. `PORTONE_LANG` 환경변수를 사용하면 표시 언어를 수동으로 지정할 수 있습니다.

```sh
PORTONE_LANG=en portone auth status
PORTONE_LANG=ko portone --help
```

표시 언어는 아래 우선순위를 따릅니다.

| 설정 | 우선순위 |
| --- | --- |
| 표시 언어 | `PORTONE_LANG`, 설정 파일의 `language`, 운영체제 언어 감지, 영어 |
| Linux 언어 감지 | `LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG` |

`auto`는 운영체제의 언어를 감지합니다. `PORTONE_LANG=auto`도 설정 파일의
`language`보다 우선합니다. macOS와 Windows에서는 UI 언어 설정을 사용합니다.
`ko-KR` 같은 지역별 locale도 지원하며, 지원하는 언어가 없으면 영어를 사용합니다.

전체 옵션은 [명령어 참조](reference/ko/index.md)를 참고하세요.
