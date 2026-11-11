//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

mod lexer;
mod token;

pub use lexer::{InputRejection, LexResult, LexicalError, LexicalErrorKind, lex, lex_bytes};
pub use token::{Token, TokenKind};
