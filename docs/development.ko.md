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
pnpm 버전을 설치해 주세요.

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm typecheck
pnpm test:scripts
```

## 문서 및 스킬 생성

```sh
cargo xtask gen-docs
cargo xtask gen-docs --check
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```
