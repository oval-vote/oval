# Oval

Reference implementation of [Oval](https://oval.vote), a plain-text format for voting guides. A
guide is a `.oval` file: the election, the ballot organized by electorate, every contest and
measure, and the author's recommendation with the reasoning behind it. The language is specified
in [oval-spec](https://github.com/oval-vote/oval-spec).

## The `oval` command

```sh
oval check guide.oval            # parse and validate; one line per diagnostic
oval compile --pretty guide.oval # the canonical JSON form, for renderers and other tools
```

`check` prints diagnostics as `file:line:column: severity[code]: message`, where the code is
`syntax` or a validation rule such as `V7`, and exits 1 if any are errors. `compile` prints JSON
that validates against the specification's `guide.schema.json`, or prints diagnostics to stderr
and exits 1 if the document has errors.

Install from a checkout:

```sh
cargo install --path crates/oval-cli
```

## Crates

| Crate | Purpose |
|---|---|
| `oval-syntax` | Lexer, parser, lossless syntax tree, and typed AST |
| `oval-core` | Identifier resolution, the validation rules, and the compiled model |
| `oval-format` | Formatter |
| `oval-cli` | The `oval` binary |
| `oval-wasm` | WebAssembly bindings, published to npm as `@oval-vote/oval` from `packages/oval` |

## Developing

Versions are pinned in `rust-toolchain.toml` (Rust, with its components and the WebAssembly
target) and `.node-version` (Node). With [mise](https://mise.jdx.dev) installed, `mise install` sets
up both; without it, rustup reads the toolchain file on its own and Node must be installed
separately. JavaScript tooling is a dev dependency of `packages/oval`.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

`crates/oval-syntax/tests/fixtures/franklin.oval` is a fictional guide at the scale of a real
metropolitan ballot and is used as the end-to-end fixture by every crate.

## The npm package

`@oval-vote/oval` exposes `check`, `compile`, and `format` to JavaScript through WebAssembly.
`wasm-pack` is a dev dependency of the package; the toolchain file adds the
`wasm32-unknown-unknown` target.

```sh
cd packages/oval
npm ci
npm run build
npm test
```

### Releasing

A release is a tag. Set `version` in `packages/oval/package.json`, commit, tag the commit
`v<version>`, and push the tag. The release workflow builds the package, runs its tests, checks
that the tag matches the version, and publishes to npm with provenance through
[trusted publishing](https://docs.npmjs.com/trusted-publishers), so no token is stored anywhere.

npm only lets a package trust a workflow once the package exists, so the first version of
`@oval-vote/oval` was published by hand with `npm publish` from `packages/oval`. After that, the
trusted publisher on npmjs.com is the `oval-vote/oval` repository and the `release.yml` workflow.
