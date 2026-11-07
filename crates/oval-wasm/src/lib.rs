//! WebAssembly bindings for the Oval voting guide format.
//!
//! This crate is the source of the `@oval-vote/oval` npm package. It exposes [`check`],
//! [`compile`], and [`format()`] from [`oval_core`] and [`oval_format`], and [`symbols`],
//! [`outline`], and [`completions`] from [`oval_analysis`], to JavaScript through `wasm-bindgen`,
//! with [`catalog`],
//! [`language_major`], and [`version`] beside them. Every function is synchronous and takes the
//! document's source text as a string.
//!
//! Results cross into JavaScript as plain values. Strings and numbers arrive as themselves;
//! diagnostics, catalog entries, symbols, outlines, and the compiled model arrive as ordinary
//! objects and arrays
//! produced by `serde-wasm-bindgen`, not as class instances. Failures are thrown. A function that
//! cannot read its input at all, because it contains U+0000, throws a string that says so; one
//! that fails because the document has errors throws the array of diagnostics that [`check`]
//! would return. Should a value fail to convert to JavaScript, the function throws that
//! conversion error.
//!
//! [`TYPES`] declares the TypeScript types of these values and is appended to the package's
//! generated typings.

use oval_analysis::Symbols;
use oval_core::{CompileError, DiagnosticCode, InputRejection, Report, Severity};
use oval_format::FormatError;
use oval_syntax::ast::{AstNode, Document};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// The TypeScript declarations appended to the package's typings.
///
/// They name the values the functions return.
///
/// - `DiagnosticIdentifier` is the union of every identifier [`DiagnosticCode::all`] lists, in
///   the specification's order.
/// - `Severity` is `"error"` or `"warning"`.
/// - `Diagnostic` is the shape of each element [`check`] returns.
/// - `CatalogEntry` is the shape of each element [`catalog`] returns.
/// - `Compiled` is the canonical model, typed as a record because its shape is fixed by the
///   specification's `guide.schema.json` rather than by this package.
/// - `Range` is a span of byte offsets into the UTF-8 source.
/// - `Symbols`, `Declaration`, and `Reference` are what [`symbols`] returns.
/// - `OutlineItem` is the shape of each element [`outline`] returns, nested through `children`.
/// - `Completion` is the shape of each element [`completions`] returns.
pub const TYPES: &str = r#"
export type DiagnosticIdentifier =
  | "illegal-tab"
  | "unexpected-character"
  | "unterminated-string"
  | "invalid-escape"
  | "malformed-number"
  | "malformed-url"
  | "inconsistent-dedent"
  | "unexpected-token"
  | "missing-identifier"
  | "missing-name"
  | "missing-contest-kind"
  | "missing-value"
  | "missing-body"
  | "missing-block"
  | "unexpected-block"
  | "unexpected-entry"
  | "repeated-field"
  | "repeated-flag"
  | "misplaced-entry"
  | "empty-recommendation"
  | "unsupported-version"
  | "missing-guide-field"
  | "empty-document"
  | "invalid-date"
  | "undeclared-reference"
  | "duplicate-declaration"
  | "mismatched-reference"
  | "block-on-reference"
  | "misplaced-candidate-field"
  | "empty-ballot"
  | "empty-section"
  | "contest-without-campaigns"
  | "campaign-without-members"
  | "duplicate-member"
  | "candidate-in-two-campaigns"
  | "invalid-multiple-count"
  | "invalid-ranked-max"
  | "retention-campaign-count"
  | "missing-recommendation"
  | "repeated-recommendation"
  | "campaign-not-in-contest"
  | "repeated-oppose"
  | "repeated-stance"
  | "tiers-in-single"
  | "support-over-max"
  | "retention-stance"
  | "markdown-profile"
  | "unused-declaration"
  | "duplicate-party-name"
  | "duplicate-ballot-title"
  | "support-over-count"
  | "non-head-reference"
  | "unpaired-outcome"
  | "insecure-url";

export type Severity = "error" | "warning";

export interface Diagnostic {
  code: DiagnosticIdentifier;
  severity: Severity;
  range: { start: number; end: number };
  message: string;
}

export interface CatalogEntry {
  identifier: DiagnosticIdentifier;
  severity: Severity;
  title: string;
  url: string;
}

export type Compiled = Record<string, unknown>;

export interface Range {
  start: number;
  end: number;
}

export type SymbolKind = "party" | "candidate" | "ballot" | "contest" | "measure";

export interface Declaration {
  name: string;
  kind: SymbolKind;
  range: Range;
  nameRange: Range;
}

export interface Reference {
  name: string;
  range: Range;
  nameRange: Range;
  declaration?: number;
}

export interface Symbols {
  declarations: Declaration[];
  references: Reference[];
}

export type OutlineKind = "ballot" | "section" | "contest" | "measure";

export interface OutlineItem {
  kind: OutlineKind;
  title?: string;
  name?: string;
  range: Range;
  selection: Range;
  children: OutlineItem[];
}

export type CompletionKind = "keyword" | SymbolKind;

export interface Completion {
  label: string;
  kind: CompletionKind;
  detail?: string;
  replace: Range;
}
"#;

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT: &str = TYPES;

/// One entry of the specification's catalog of diagnostics.
#[derive(Serialize)]
struct CatalogEntry {
    identifier: &'static str,
    severity: Severity,
    title: &'static str,
    url: String,
}

/// The major version of the Oval language this package reads, `1`.
///
/// Exported to JavaScript as `languageMajor()`, returning a number. A document whose version
/// marker names another major version is reported as `unsupported-version` rather than parsed.
#[wasm_bindgen(js_name = languageMajor)]
pub fn language_major() -> u32 {
    oval_core::SUPPORTED_LANGUAGE_MAJOR
}

/// The version of this package, as a semantic version string.
///
/// It is the Cargo version of this crate, which the npm package mirrors.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Every diagnostic this package can report, in the specification's order.
///
/// Returns an array of `CatalogEntry` objects, one per [`DiagnosticCode`], each with the code's
/// `identifier`, `severity`, `title`, and `url`, the address of its entry on the specification's
/// site. A tool uses it to list what it reports or to link a diagnostic to its documentation.
///
/// ```js
/// const { catalog } = require("@oval-vote/oval");
/// const entry = catalog().find((e) => e.identifier === "invalid-date");
/// console.log(entry.title, entry.url);
/// ```
///
/// # Errors
///
/// Throws only when the entries cannot be converted to a JavaScript value.
#[wasm_bindgen(unchecked_return_type = "CatalogEntry[]")]
pub fn catalog() -> Result<JsValue, JsValue> {
    let entries: Vec<CatalogEntry> = DiagnosticCode::all()
        .into_iter()
        .map(|code| CatalogEntry {
            identifier: code.identifier(),
            severity: code.severity(),
            title: code.title(),
            url: code.url(),
        })
        .collect();
    serde_wasm_bindgen::to_value(&entries).map_err(into_js)
}

/// Parses and validates `source`, returning every diagnostic in source order.
///
/// Returns an array of `Diagnostic` objects, errors and warnings alike, each with its `code` (the
/// identifier), `severity`, `range`, and `message`. The range is `{ start, end }` in byte offsets
/// into the UTF-8 source, not in JavaScript string indices. The array is empty when the document
/// is valid and has no warnings. When the version marker is missing or names another major
/// version, the array contains only `unsupported-version`.
///
/// ```js
/// const { check } = require("@oval-vote/oval");
/// const diagnostics = check('oval 1\nguide "Franklin Voter Guide"\n');
/// const errors = diagnostics.filter((d) => d.severity === "error");
/// console.log(errors.map((d) => `${d.code}: ${d.message}`));
/// ```
///
/// # Errors
///
/// Throws a string when `source` contains U+0000, the one input that is not an Oval document at
/// all.
#[wasm_bindgen(unchecked_return_type = "Diagnostic[]")]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    diagnostics(&oval_core::check(source).map_err(rejected)?)
}

/// Compiles `source` to the canonical model, or throws its diagnostics.
///
/// Returns the object the specification's `guide.schema.json` describes, Section 5, which is the
/// guide with every reference resolved and the derived fields filled in, as a plain JavaScript
/// object. It is the same value `oval compile` prints as JSON.
///
/// ```js
/// const { compile } = require("@oval-vote/oval");
/// let compiled;
/// try {
///   compiled = compile(source);
/// } catch (thrown) {
///   if (Array.isArray(thrown)) console.error(thrown);
///   throw thrown;
/// }
/// console.log(compiled.guide.name, compiled.counts.contests);
/// ```
///
/// # Errors
///
/// Throws a string when `source` contains U+0000. Throws the array of `Diagnostic` objects that
/// [`check`] would return when the document has any error. Warnings alone do not prevent
/// compilation and are not reported here, so a tool that wants them calls [`check`] as well.
#[wasm_bindgen(unchecked_return_type = "Compiled")]
pub fn compile(source: &str) -> Result<JsValue, JsValue> {
    match oval_core::compile(source) {
        Ok(compiled) => serde_wasm_bindgen::to_value(&compiled).map_err(into_js),
        Err(CompileError::Rejected(rejection)) => Err(rejected(rejection)),
        Err(CompileError::Invalid(report)) => Err(diagnostics(&report)?),
    }
}

/// Formats `source` in the canonical style of Section 6, or throws its diagnostics.
///
/// Returns the formatted document as a string. It uses `\n` as its line terminator, ends with
/// exactly one, and formatting it again returns it unchanged. A document with validation errors
/// formats normally; only lexical and syntax errors prevent formatting.
///
/// ```js
/// const { format } = require("@oval-vote/oval");
/// format('oval 01\nguide   "G"\n'); // 'oval 1\nguide "G"\n'
/// ```
///
/// # Errors
///
/// Throws a string when `source` contains U+0000. Throws the array of `Diagnostic` objects that
/// [`check`] would return when the document has lexical or syntax errors, so a caller sees every
/// problem and not only the ones that stopped the formatter.
#[wasm_bindgen]
pub fn format(source: &str) -> Result<String, JsValue> {
    match oval_format::format(source) {
        Ok(formatted) => Ok(formatted),
        Err(FormatError::Rejected(rejection)) => Err(rejected(rejection)),
        Err(FormatError::SyntaxErrors(_)) => {
            Err(diagnostics(&oval_core::check(source).map_err(rejected)?)?)
        }
    }
}

/// The declared identifiers of `source` and every use of one.
///
/// Returns a `Symbols` object with `declarations` and `references`, each in document order. A
/// declaration has its `name` without the `@`, its `kind`, its `range`, and its `nameRange`, the
/// span of the name alone, which a rename replaces. A reference has the same spans and
/// `declaration`, the index in `declarations` of the one it names, absent when nothing declares
/// the name. Ranges are byte offsets into the UTF-8 source. A document with errors still has
/// symbols; the parts the parser could not read are left out.
///
/// ```js
/// const { symbols } = require("@oval-vote/oval");
/// const { declarations, references } = symbols(source);
/// const uses = references.filter((r) => r.name === "delgado").map((r) => r.range);
/// ```
///
/// # Errors
///
/// Throws a string when `source` contains U+0000.
#[wasm_bindgen(unchecked_return_type = "Symbols")]
pub fn symbols(source: &str) -> Result<JsValue, JsValue> {
    let document = document(source)?;
    serde_wasm_bindgen::to_value(&Symbols::new(&document)).map_err(into_js)
}

/// The structure of `source` as nested ballots, sections, contests, and measures.
///
/// Returns an array of `OutlineItem` objects for the root ballots, in document order, each with
/// its `kind`, `title`, `name` (the identifier without `@`, absent for a section), `range` (the
/// whole construct), `selection` (its header line), and `children`. Parties and candidates are
/// not in the outline; [`symbols`] lists them.
///
/// ```js
/// const { outline } = require("@oval-vote/oval");
/// for (const ballot of outline(source)) console.log(ballot.title, ballot.children.length);
/// ```
///
/// # Errors
///
/// Throws a string when `source` contains U+0000.
#[wasm_bindgen(unchecked_return_type = "OutlineItem[]")]
pub fn outline(source: &str) -> Result<JsValue, JsValue> {
    let document = document(source)?;
    serde_wasm_bindgen::to_value(&oval_analysis::outline(&document)).map_err(into_js)
}

/// Everything the grammar accepts at `offset` in `source`, for completion.
///
/// Returns an array of `Completion` objects. Each has a `label` to insert, which is a keyword or
/// an identifier with its `@`; its `kind`; `detail`, the name an identifier declares, such as a
/// candidate's name; and `replace`, the range of the partial word before `offset` that the label
/// replaces. `offset` is a byte offset into the UTF-8 source. Keywords are the ones the parser
/// accepts at that point and identifiers are the declarations of the kind the point names, so
/// `support` offers candidates and `party` offers parties. The array is empty inside a string, a
/// comment, or a Markdown body, and where the point names a new identifier.
///
/// ```js
/// const { completions } = require("@oval-vote/oval");
/// const found = completions(source, byteOffsetOfCursor);
/// console.log(found.map((c) => c.label));
/// ```
///
/// # Errors
///
/// Throws a string when `source` contains U+0000.
#[wasm_bindgen(unchecked_return_type = "Completion[]")]
pub fn completions(source: &str, offset: u32) -> Result<JsValue, JsValue> {
    if source.contains('\0') {
        return Err(rejected(InputRejection {
            offset: oval_syntax::TextSize::from(0),
        }));
    }
    let found = oval_analysis::completions(source, oval_syntax::TextSize::from(offset));
    serde_wasm_bindgen::to_value(&found).map_err(into_js)
}

fn document(source: &str) -> Result<Document, JsValue> {
    let parsed = oval_syntax::parser::parse(source).map_err(rejected)?;
    Document::cast(parsed.syntax()).ok_or_else(|| JsValue::from("the root is not a document"))
}

fn diagnostics(report: &Report) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(report.diagnostics()).map_err(into_js)
}

fn into_js(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from(error)
}

fn rejected(rejection: InputRejection) -> JsValue {
    JsValue::from(rejection.to_string())
}

#[cfg(test)]
mod tests {
    use oval_core::DiagnosticCode;

    use super::TYPES;

    #[test]
    fn the_typescript_union_lists_every_identifier_in_order() {
        let union = TYPES
            .split("export type DiagnosticIdentifier =")
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .expect("union in the custom section");
        let listed: Vec<&str> = union
            .split('|')
            .map(|item| item.trim().trim_matches('"'))
            .filter(|item| !item.is_empty())
            .collect();
        let all: Vec<&str> = DiagnosticCode::all()
            .into_iter()
            .map(DiagnosticCode::identifier)
            .collect();
        assert_eq!(listed, all);
    }

    #[test]
    fn the_npm_package_has_the_workspace_version() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../../packages/oval/package.json"))
                .expect("package.json is JSON");
        assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    }
}
