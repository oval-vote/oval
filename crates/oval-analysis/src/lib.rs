//! Symbols, outlines, and other editor queries over Oval voting guides.
//!
//! An [`Analysis`] checks a document once and then answers the questions an editor asks about
//! it, such as which identifier sits under the cursor and where it's declared. Every answer is a
//! byte range into the document, so a language server, a WebAssembly binding, or a test can turn
//! it into its own positions. A document with errors still has answers, since an editor needs them
//! most while the author is mid-edit.
//!
//! ```
//! use oval_analysis::Analysis;
//!
//! let source = "oval 1\nparty @dem \"Democratic\"\ncandidate @n \"Nadia\"\n  party @dem\n";
//! let analysis = Analysis::new(source)?;
//! let reference = source.rfind("@dem").expect("the candidate names the party");
//! let definition = analysis.definition(reference.try_into()?).expect("the party is declared");
//! assert_eq!(definition.identifier.name().map(|name| name.to_string()).as_deref(), Some("dem"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod completion;
mod hover;
mod outline;
mod symbols;

pub use completion::{Completion, CompletionKind};
pub use hover::Hover;
pub use outline::{OutlineItem, OutlineKind};
pub use oval_core::InputRejection;
pub use symbols::{Occurrence, Symbol};

use oval_core::{Report, Symbols};

/// A checked document, ready to answer editor queries.
#[derive(Debug)]
pub struct Analysis {
    report: Report,
    symbols: Symbols,
}

impl Analysis {
    /// Checks `source` and collects its symbols.
    ///
    /// # Errors
    ///
    /// Returns an [`InputRejection`] when the input can't be an Oval document at all, because it
    /// contains U+0000 or is too large.
    pub fn new(source: &str) -> Result<Self, InputRejection> {
        let report = oval_core::check(source)?;
        let symbols = Symbols::new(&report.parse().document());
        Ok(Self { report, symbols })
    }

    /// Returns the report of the document's diagnostics, with its syntax tree.
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Returns the document's declarations and references.
    pub fn symbols(&self) -> &Symbols {
        &self.symbols
    }
}
