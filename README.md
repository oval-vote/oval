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

## Developing

The toolchain is pinned in `rust-toolchain.toml`; rustup picks it up automatically.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

`crates/oval-syntax/tests/fixtures/franklin.oval` is a fictional guide at the scale of a real
metropolitan ballot and is used as the end-to-end fixture by every crate.
