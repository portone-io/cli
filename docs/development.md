# Development

English | [한국어](development.ko.md)

## Build from source

Install stable Rust, then run from the repository root:

```sh
cargo build --locked --release
./target/release/portone --help
```

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For packaging scripts, install the Node.js version required by the README
and the pnpm version specified in `package.json`.

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm typecheck
pnpm test:scripts
```

## Generate documentation and skills

```sh
cargo xtask gen-docs
cargo xtask gen-docs --check
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```
