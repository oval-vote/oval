# Oval Vote

An open text format for voting guides.

- File extension: `.oval`
- Media type: `text/vnd.oval-vote`
- Every file starts with the line `oval 1`

This repository holds the specification, the conformance suite, and the Rust
reference implementation (parser, formatter, linter, language server, CLI).

| Path                  | Contents                                                     |
|-----------------------|--------------------------------------------------------------|
| `spec/1/`             | The version 1 specification and formal grammar               |
| `examples/`           | `sample.oval`, every construct once; `2026-general.oval`, a real guide |
| `tests/cases/`        | Conformance suite: `.oval` inputs, expected `.diagnostics`, model `.snap` |
| `crates/oval-syntax`  | Lossless parser, typed tree accessors, inline markup         |
| `crates/oval-model`   | Data model, lowering, semantic rules                         |
| `crates/oval-fmt`     | Canonical formatter                                          |
| `crates/oval-lint`    | Style rules, configured by `oval.toml`                       |
| `crates/oval-cli`     | The `oval` command                                           |

```
cargo run -p oval-cli -- check examples/*.oval          # spec diagnostics, exit 1 on errors
cargo run -p oval-cli -- lint examples/*.oval           # spec diagnostics plus lint rules
cargo run -p oval-cli -- lint --list                    # the rules and their default levels
cargo run -p oval-cli -- fmt examples/*.oval            # rewrite in canonical form (--check to verify)
cargo run -p oval-cli -- export examples/sample.oval    # data model as JSON
cargo run -p oval-cli -- tree examples/sample.oval      # syntax tree dump
cargo test                                              # unit tests, conformance suite, formatter checks
```

Developing: `mise run fmt` formats the Rust code and the example guides, `mise run lint`
runs clippy with warnings denied plus the guide linter, `mise run check` runs everything
CI runs, and `mise run hooks` installs a pre-commit hook that refuses unformatted or
warning-producing Rust. Lint levels are set once in the workspace `Cargo.toml`
(clippy pedantic, with a short allow list) and `rustfmt.toml` holds the style.

Status: pre-release. The parser and model exist and pass the conformance suite;
the grammar is not yet frozen. See `PLAN.md` in the parent directory.
