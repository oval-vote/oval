//! Editor queries over Oval voting guides.
//!
//! This crate answers the questions an editor asks about a document once it is parsed. Which
//! identifiers are declared and where each one is used ([`Symbols`]), what the document's
//! structure is ([`outline`]), which identifier sits under the cursor ([`Symbols::at`]), what the
//! grammar accepts at a position ([`completions`]), what to say about the token there ([`hover`]),
//! and how to repair the diagnostics that have a mechanical fix ([`fixes`]). Every answer is positions and names, so a language server, a WebAssembly
//! binding, or a test can turn it into its own representation.
//!
//! The functions take a parsed [`Document`](oval_syntax::ast::Document) and never fail. A document with errors still has a
//! tree, and an editor needs symbols and an outline most while the author is mid-edit, so every
//! part the parser could not find is left out rather than reported.
//!
//! # Examples
//!
//! ```
//! use oval_analysis::{Symbols, outline};
//! use oval_syntax::ast::{AstNode, Document};
//! use oval_syntax::parser::parse;
//!
//! let source = r#"oval 1
//! guide "G"
//! election "E"
//! date 2026-11-03
//! candidate @delgado "Rosa Delgado"
//! ballot @city "City"
//!   contest @attorney "City Attorney" single
//!     campaign @delgado
//!     recommend
//!       support @delgado
//! "#;
//! let document = Document::cast(parse(source).unwrap().syntax()).unwrap();
//!
//! let symbols = Symbols::new(&document);
//! let reference = source.rfind("@delgado").unwrap();
//! let declaration = symbols.definition(reference.try_into().unwrap()).unwrap();
//! assert_eq!(usize::from(declaration.range.start()), source.find("@delgado").unwrap());
//! assert_eq!(symbols.occurrences(reference.try_into().unwrap()).len(), 3);
//!
//! let items = outline(&document);
//! assert_eq!(items[0].title.as_deref(), Some("City"));
//! assert_eq!(items[0].children[0].title.as_deref(), Some("City Attorney"));
//! ```

mod completion;
mod fix;
mod hover;
mod names;
mod outline;
mod symbols;

pub use completion::{Completion, CompletionKind, completions};
pub use fix::{Edit, Fix, fixes};
pub use hover::{Hover, hover};
pub use outline::{OutlineItem, OutlineKind, outline};
pub use symbols::{Declaration, Reference, Symbol, SymbolKind, Symbols};
