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
cargo build --locked --workspace
```

For packaging scripts, install the Node.js version required by the README
and the pnpm version specified in `package.json`.

```sh
pnpm install --frozen-lockfile
pnpm check
pnpm typecheck
```

## Generate documentation and skills

```sh
cargo xtask gen-docs
cargo xtask gen-docs --check
cargo xtask sync-plugin-skills
cargo xtask sync-plugin-skills --check
```

## Release npm packages

The release workflow stages `@portone/cli` and all eight platform packages with
`npm stage publish --tag latest`. It authenticates through npm trusted publishers
(OIDC), without `NPM_TOKEN`. Each package's trusted publisher must allow staged
publishing from `portone-io/cli`, workflow `release.yml`, environment
`npm-publish`.

Staged versions become available on npm only after a maintainer reviews and
approves them with 2FA. Use npm 11.15.0 or later to review and approve each stage,
or use the Staged Packages tab on npmjs.com:

```sh
npm stage list
npm stage view <stage-id>
npm stage approve <stage-id>
```

Approve all eight platform packages before approving `@portone/cli`, so its
optional dependencies are available when users install it. The workflow creates
the GitHub release and uploads binaries after staging; npm approval is separate.

See the [npm staged publishing documentation](https://docs.npmjs.com/staged-publishing/).
