//! Lexer, parser, and lossless syntax tree for the Oval voting guide format.

mod input_rejection;
mod lex_result;
mod lexer;
mod lexical_error;
mod token;

pub use input_rejection::InputRejection;
pub use lex_result::LexResult;
pub use lexer::{lex, lex_bytes};
pub use lexical_error::{LexicalError, LexicalErrorKind};
pub use token::{Token, TokenKind};
