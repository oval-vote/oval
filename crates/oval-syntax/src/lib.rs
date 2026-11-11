//! Lexer for the Oval voting guide format.
//!
//! [`lex`] splits the text of an Oval document into [`Token`]s, as described in Sections 2 and 3.1
//! of the [Oval specification](https://spec.oval.vote). [`lex_bytes`] does the same for a document
//! that arrives as raw bytes.
//!
//! ```
//! use oval_syntax::{TokenKind, lex};
//!
//! let result = lex("guide \"Franklin County Voters' Guide\"\n")?;
//! let kinds: Vec<TokenKind> = result.tokens.iter().map(|token| token.kind).collect();
//! assert_eq!(
//!     kinds,
//!     [
//!         TokenKind::Word,
//!         TokenKind::Whitespace,
//!         TokenKind::String,
//!         TokenKind::Newline,
//!     ]
//! );
//! # Ok::<(), oval_syntax::InputRejection>(())
//! ```
//!
//! # Tokens
//!
//! A [`Token`] is a [`TokenKind`] and a length in bytes. Tokens hold no text. They are contiguous,
//! so each one's text is the slice of the document that begins where the token before it ends.
//! Every byte of a document belongs to exactly one token, so the tokens reproduce it exactly, which
//! is what lets a formatter rewrite a document without losing anything.
//!
//! Every kind of token is in one of four categories.
//!
//! - **Lexical** tokens carry a document's content. They are words, strings, integers, dates, URLs,
//!   `@`, `:`, `,`, and Markdown bodies.
//! - **Ignored** tokens are whitespace, comments, blank lines, and a leading byte order mark. The
//!   parser skips them.
//! - **Layout** tokens give a document its structure. A [`Newline`](TokenKind::Newline) ends each
//!   line with tokens, and [`Indent`](TokenKind::Indent) and [`Dedent`](TokenKind::Dedent) open and
//!   close blocks as indentation changes.
//! - **Error** tokens cover text that cannot be lexed.
//!
//! # Errors
//!
//! An [`InputRejection`] means the input cannot be an Oval document at all, because it contains
//! U+0000, is not UTF-8, or is too large. It is returned instead of any tokens.
//!
//! Any other document lexes completely. Its flaws are reported as [`LexicalError`]s in
//! [`LexResult::errors`], alongside the tokens, and lexing continues after each one.
//!
//! # Offsets
//!
//! Lengths and offsets are counted in bytes, not characters, and stored as `u32`. A document longer
//! than [`u32::MAX`] bytes is rejected, so every offset in an accepted document fits.

mod lexer;
mod token;

pub use lexer::{InputRejection, LexResult, LexicalError, LexicalErrorKind, lex, lex_bytes};
pub use token::{Token, TokenKind};
