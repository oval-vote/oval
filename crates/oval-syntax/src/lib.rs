//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

mod lexer;
mod token;

pub use lexer::lex;
pub use token::{Token, TokenKind};
