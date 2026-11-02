# oval

Reference implementation of [Oval](https://oval.vote), a plain-text file format for voting
guides. The language is specified in [oval-spec](https://github.com/oval-vote/oval-spec).

This repository holds the core library (lexer, parser, validator, formatter) and the tools built
on it (command line, language server, WebAssembly bindings). Nothing is usable yet.

## Developing

The toolchain is pinned in `rust-toolchain.toml`; rustup picks it up automatically.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```
