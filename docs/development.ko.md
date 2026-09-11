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
cargo test --workspace
```

패키징 스크립트는 README에 명시된 Node.js와 `package.json`에 지정된
pnpm 버전을 설치한 뒤 검사합니다.

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm typecheck
pnpm test:scripts
```

## 문서와 스킬

CLI 소스의 명령어 도움말을 수정한 뒤 두 언어의 참조 문서를 생성합니다.
스킬은 `skills/` 아래 원본을 수정한 뒤 플러그인 사본을 동기화합니다.

```sh
cargo xtask gen-docs
cargo xtask gen-docs --check
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

생성 파일을 원본 변경 사항과 함께 커밋하세요. CI에서 두 항목을 검사합니다.
영어·한국어 안내는 함께 수정하세요. 루트 README는 npm에도 게시되는
`packages/@portone/cli/README*.md`의 심볼릭 링크입니다.
