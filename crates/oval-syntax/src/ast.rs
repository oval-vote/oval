//! Typed views of a syntax tree.
//!
//! A view wraps a [`SyntaxNode`](crate::SyntaxNode) of one kind, such as the [`Document`] at the
//! root of every tree, and reads the parts of that construct from the tree. Every view implements
//! [`AstNode`], whose [`cast`](AstNode::cast) turns a node into a view and whose
//! [`syntax`](AstNode::syntax) returns the node again. A view holds nothing but its node, so it
//! clones cheaply and reads the tree each time it is asked.
//!
//! A document with errors still has a complete tree, and views read it like any other. A part the
//! parser did not find is absent rather than an error. Text that was discarded or set aside lives
//! in an [`ErrorNode`](crate::SyntaxKind::ErrorNode) and is not part of any view.
//!
//! ```
//! use oval_syntax::SyntaxKind;
//! use oval_syntax::ast::AstNode;
//!
//! let result = oval_syntax::parse("guide \"Franklin Voter Guide\"\n")?;
//! let document = result.document();
//! assert_eq!(document.syntax().kind(), SyntaxKind::Document);
//! # Ok::<(), oval_syntax::InputRejection>(())
//! ```

mod ast_node;
mod ast_token;
mod ballot;
mod candidate;
mod contest;
mod contest_kind;
mod document;
mod extensibility;
mod fields;
mod identifier;
mod literals;
mod measure;
mod party;
mod party_ref;
mod requirement;
mod section;
mod support;
mod version_marker;

pub use ast_node::AstNode;
use ast_node::ast_node;
pub use ast_token::AstToken;
use ast_token::ast_token;
pub use ballot::{Ballot, BallotItem};
pub use candidate::Candidate;
pub use contest::Contest;
pub use contest_kind::{ContestKind, Method};
pub use document::Document;
pub use extensibility::Extensibility;
pub use fields::{DateField, MarkdownField, StringField, StringListField, UrlField};
pub use identifier::Identifier;
pub use literals::{DateLiteral, IntegerLiteral, MarkdownBody, StringLiteral, UrlLiteral};
pub use measure::Measure;
pub use party::Party;
pub use party_ref::PartyRef;
pub use requirement::Requirement;
pub use section::{Section, SectionItem};
pub use version_marker::VersionMarker;
