# Oval Vote

An open text format for voting guides.

- File extension: `.oval`
- Media type: `text/vnd.oval-vote`
- Every file starts with the line `oval 1`

This repository holds the specification, the conformance suite, and the Rust
reference implementation (parser, formatter, linter, language server, CLI).

| Path            | Contents                                                     |
|-----------------|--------------------------------------------------------------|
| `spec/1/`       | The version 1 specification and formal grammar               |
| `examples/`     | `sample.oval`, every construct once; `2026-general.oval`, a real guide |

Status: pre-release. The grammar is being tested against real guides before any
code is written; see `PLAN.md` in the parent directory.
