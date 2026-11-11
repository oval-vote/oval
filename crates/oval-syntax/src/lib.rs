//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

mod input_rejection;
mod lexer;
mod token;

pub use input_rejection::InputRejection;
pub use lexer::{lex, lex_bytes};
pub use token::{Token, TokenKind};
