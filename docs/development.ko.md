# 개발 안내

[English](development.md) | 한국어

## 소스 빌드

Rust stable을 설치한 뒤 저장소 루트에서 실행합니다.

```sh
cargo build --locked --release
./target/release/portone --help
```

## 검사

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --locked --workspace
```

패키징 스크립트는 README에 명시된 Node.js와 `package.json`에 지정된
pnpm 버전을 설치해 주세요.

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm typecheck
```

## 문서 및 스킬 생성

```sh
cargo xtask gen-docs
cargo xtask gen-docs --check
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

## npm 패키지 배포

릴리스 워크플로는 `@portone/cli`와 플랫폼 패키지 8개를 모두
`npm stage publish --tag latest`로 스테이징합니다. `NPM_TOKEN` 없이 npm trusted
publisher(OIDC)로 인증합니다. 각 패키지의 trusted publisher는 저장소
`portone-io/cli`, 워크플로 `release.yml`, 환경 `npm-publish`에서의
staged publishing을 허용해야 합니다.

스테이징된 버전은 관리자가 검토하고 2FA로 승인한 뒤 npm에 공개됩니다.
npm 11.15.0 이상에서 각 스테이지를 조회하고 승인하거나, npmjs.com의
Staged Packages 탭을 사용하세요.

```sh
npm stage list
npm stage view <stage-id>
npm stage approve <stage-id>
```

설치 시 optional dependency를 내려받을 수 있도록 플랫폼 패키지 8개를 모두
승인한 다음 `@portone/cli`를 승인합니다. 워크플로는 스테이징 후 GitHub 릴리스를
만들고 바이너리를 업로드하며, npm 승인은 별도로 진행합니다.

자세한 내용은 [npm staged publishing 문서](https://docs.npmjs.com/staged-publishing/)를
참고하세요.
