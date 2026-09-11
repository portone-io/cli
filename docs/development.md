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
and the pnpm version in `package.json`, then run:

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm typecheck
pnpm test:scripts
```

## Documentation and skills

Edit command help in the CLI source and generate both reference languages.
Edit canonical skills under `skills/` and synchronize their plugin copies:

```sh
cargo xtask gen-docs
cargo xtask gen-docs --check
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

Commit generated files with their source changes; CI checks both.
Maintain English and Korean guides together. The root READMEs are symlinks
to `packages/@portone/cli/README*.md`, which are also published on npm.
