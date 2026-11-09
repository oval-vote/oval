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

`check` prints diagnostics as `file:line:column: severity[code]: message`, where the code is the
identifier of the error's entry in the specification, such as `illegal-tab` or `invalid-date`, and
exits 1 if any are errors, or if any are warnings and `--strict` is given. `compile` prints JSON
that validates against the specification's `oval.schema.json`, or prints diagnostics to stderr
and exits 1 if the document has errors.

Install with Homebrew on macOS or Linux, or download a binary from the
[releases](https://github.com/oval-vote/oval/releases), or build from a checkout:

```sh
brew install oval-vote/tap/oval
cargo install --path crates/oval-cli
```

## Crates

| Crate           | Purpose                                                                          |
| --------------- | -------------------------------------------------------------------------------- |
| `oval-syntax`   | Lexer, parser, lossless syntax tree, and typed AST                               |
| `oval-core`     | Identifier resolution, the validation rules, and the compiled model              |
| `oval-analysis` | Symbols, outlines, and the other queries an editor makes                         |
| `oval-format`   | Formatter                                                                        |
| `oval-cli`      | The `oval` binary                                                                |
| `oval-wasm`     | WebAssembly bindings, published to npm as `@oval-vote/oval` from `packages/oval` |

## Developing

Versions are pinned in `rust-toolchain.toml` (Rust, with its components and the WebAssembly
target) and `.node-version` (Node). With [mise](https://mise.jdx.dev) installed, `mise install` sets
up both; without it, rustup reads the toolchain file on its own and Node must be installed
separately. JavaScript tooling is a dev dependency of `packages/oval`.

```sh
mise run fmt            # rustfmt and Prettier
mise run ci             # formatting, clippy, documentation, and version checks, as CI runs them
cargo test --workspace  # the tests, which CI runs on Linux, macOS, and Windows
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

### Versioning

This workspace and the npm package share one version number, which is ordinary semver for the
implementation and says nothing about the language. The language has its own major version, which
documents declare with `oval 1`; the one this implementation reads is
`oval_syntax::SUPPORTED_LANGUAGE_MAJOR`, which the npm package exposes as `languageMajor()`. The specification is published in dated editions at [spec.oval.vote](https://spec.oval.vote).

The implementation version is set in `[workspace.package]` in `Cargo.toml`, which every crate
inherits, and mirrored in `Cargo.lock` and `packages/oval/package.json`. `mise run set-version
<version>` writes all of them, `mise run version:check` fails when any disagrees, and CI runs the
check.

### Releasing

The version in `Cargo.toml` and `packages/oval/package.json` is the version the next release will
have. Three workflows under Actions manage it, each run from `main` as the organization's release
app:

- **Set Version** takes a version and commits `Set version to <version>`, through
  `mise run set-version`. It makes no release.
- **Release** takes an optional version. It refuses unless the version is greater than the latest
  release, is not tagged or on npm, and every check on the latest commit of `main` is green. When a
  version was given it sets it, runs `mise run ci`, and commits `Release <version>`. Then it creates
  the GitHub release, tagged `v<version>`, with an empty body. The tag runs the same workflow's
  `publish` job, which builds the package, runs its tests, checks the versions against the tag, and
  publishes to npm with provenance through
  [trusted publishing](https://docs.npmjs.com/trusted-publishers), so no token is stored anywhere.
  The tag also builds the `oval` binary for macOS and Linux on Apple silicon, Intel, and arm64,
  attaches the archives and their checksums to the release, and pushes the Homebrew formula to
  [homebrew-tap](https://github.com/oval-vote/homebrew-tap).
- **Unrelease** takes the latest version, reverts its release commit if there was one, deletes
  the GitHub release and tag, and withdraws the formula from the tap. A version that reached npm
  cannot be removed; the workflow says so, and the next release takes a new number.

The app's ID and private key are the `OVAL_RELEASE_APP_ID` and `OVAL_RELEASE_APP_PRIVATE_KEY`
organization secrets. Commits and tags made with its token trigger the CI and publish workflows,
which commits made with the default `GITHUB_TOKEN` would not.

npm only lets a package trust a workflow once the package exists, so the first version of
`@oval-vote/oval` was published by hand with `npm publish` from `packages/oval`. After that, the
trusted publisher on npmjs.com is the `oval-vote/oval` repository and the `release.yml` workflow.
