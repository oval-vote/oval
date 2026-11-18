//! WebAssembly bindings for the Oval voting guide format.
//!
//! This crate is the source of the `@oval-vote/oval` npm package, built with `wasm-pack`. Every
//! function is synchronous. A panic, which is always a bug, prints its message to the console
//! before the call fails.
//!
//! A function throws an `OvalError` when the document has errors that stop it, with those errors
//! in its `diagnostics`, and a plain `Error` when the source can't be an Oval document at all.
//!
//! Positions in a document are JavaScript string indices, which count UTF-16 code units, so
//! `source.slice(range.start, range.end)` is the text a range spans. Results are plain objects and
//! arrays, whose TypeScript types are declared alongside the functions.

mod analysis;
mod utf16;

pub use analysis::Analysis;

use oval_core::{DiagnosticCode, InputRejection, Report, Severity};
use oval_format::FormatError;
use serde::Serialize;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

use crate::utf16::{Range, Utf16Index};

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = include_str!("types.d.ts");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}

#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|info| console_error(&info.to_string())));
}

/// Returns the major version of the language this package reads.
///
/// Exported to JavaScript as `languageMajor()`.
#[wasm_bindgen(js_name = languageMajor)]
pub fn language_major() -> u32 {
    oval_core::MAJOR_VERSION
}

/// Returns the version of this package.
///
/// It's the version of this crate, which the npm package shares.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Checks `source` and returns every problem found in it, ordered by where each one begins.
///
/// Exported to JavaScript as `check(source)`, which returns `Diagnostic[]`.
#[wasm_bindgen(unchecked_return_type = "Diagnostic[]")]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    let index = Utf16Index::new(source);
    let report = oval_core::check(source).map_err(|rejection| rejected(rejection, &index))?;
    to_js(&Diagnostic::all(&report, &index))
}

/// Compiles `source` into the object the specification's schema describes.
///
/// Exported to JavaScript as `compile(source)`, which returns a `Guide`. Warnings don't stop it,
/// and errors make it throw an `OvalError` with every error in the document.
#[wasm_bindgen(unchecked_return_type = "Guide")]
pub fn compile(source: &str) -> Result<JsValue, JsValue> {
    let index = Utf16Index::new(source);
    let report = oval_core::check(source).map_err(|rejection| rejected(rejection, &index))?;
    match report.compile() {
        Some(compiled) => to_js(&compiled),
        None => Err(invalid(&report, &index, |code| {
            code.severity() == Severity::Error
        })),
    }
}

/// Formats `source` in the canonical style.
///
/// Exported to JavaScript as `format(source)`. Validation errors don't stop it, so a guide that
/// isn't finished can be formatted. Lexical and syntax errors make it throw an `OvalError` with
/// those errors.
#[wasm_bindgen]
pub fn format(source: &str) -> Result<String, JsValue> {
    let index = Utf16Index::new(source);
    match oval_format::format(source) {
        Ok(formatted) => Ok(formatted),
        Err(FormatError::Rejected(rejection)) => Err(rejected(rejection, &index)),
        Err(FormatError::Malformed(_)) => {
            let report =
                oval_core::check(source).map_err(|rejection| rejected(rejection, &index))?;
            Err(invalid(&report, &index, |code| {
                !matches!(code, DiagnosticCode::Validation(_))
            }))
        }
    }
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(JsValue::from)
}

fn rejected(rejection: InputRejection, index: &Utf16Index) -> JsValue {
    js_sys::Error::new(&rejection_message(rejection, index)).into()
}

fn rejection_message(rejection: InputRejection, index: &Utf16Index) -> String {
    match rejection {
        InputRejection::NulCharacter { offset } => format!(
            "the document contains U+0000 at index {}",
            index.index(offset.into())
        ),
        InputRejection::TooLarge | InputRejection::InvalidUtf8 { .. } => rejection.to_string(),
    }
}

fn invalid(report: &Report, index: &Utf16Index, stops: impl Fn(DiagnosticCode) -> bool) -> JsValue {
    let errors: Vec<Diagnostic> = report
        .diagnostics()
        .iter()
        .filter(|diagnostic| stops(diagnostic.code))
        .map(|diagnostic| Diagnostic::new(diagnostic, index))
        .collect();
    let error = js_sys::Error::new(&invalid_message(errors.len()));
    error.set_name("OvalError");
    let diagnostics = match to_js(&errors) {
        Ok(diagnostics) => diagnostics,
        Err(conversion) => return conversion,
    };
    match js_sys::Reflect::set(&error, &JsValue::from_str("diagnostics"), &diagnostics) {
        Ok(_) => error.into(),
        Err(thrown) => thrown,
    }
}

fn invalid_message(errors: usize) -> String {
    match errors {
        1 => "the document has an error".to_owned(),
        errors => format!("the document has {errors} errors"),
    }
}

#[derive(Debug, PartialEq, Serialize)]
struct Diagnostic {
    code: &'static str,
    severity: &'static str,
    range: Range,
    message: String,
}

impl Diagnostic {
    fn all(report: &Report, index: &Utf16Index) -> Vec<Self> {
        report
            .diagnostics()
            .iter()
            .map(|diagnostic| Self::new(diagnostic, index))
            .collect()
    }

    fn new(diagnostic: &oval_core::Diagnostic, index: &Utf16Index) -> Self {
        Self {
            code: diagnostic.code.spec_id(),
            severity: match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            range: index.range(diagnostic.range),
            message: diagnostic.message.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use oval_core::DiagnosticCode;

    use oval_syntax::InputRejection;

    use super::{Diagnostic, invalid_message, rejection_message};
    use crate::utf16::{Range, Utf16Index};

    #[test]
    fn the_typescript_code_union_lists_every_code_in_order() {
        let union = include_str!("types.d.ts")
            .split_once("export type DiagnosticCode =")
            .and_then(|(_, rest)| rest.split_once(';'))
            .map(|(union, _)| union)
            .unwrap();
        let declared: Vec<&str> = union
            .split('|')
            .map(|member| member.trim().trim_matches('"'))
            .filter(|member| !member.is_empty())
            .collect();
        let codes: Vec<&str> = DiagnosticCode::all().map(DiagnosticCode::spec_id).collect();
        assert_eq!(declared, codes);
    }

    #[test]
    fn a_diagnostic_spans_string_indices() {
        let source = "oval 1\nguide \"🗳 Guía\"\nelection \"E\"\ndate 2026-11-03\nparty @p \"Pé\"\n";
        let report = oval_core::check(source).unwrap();
        let found = Diagnostic::all(&report, &Utf16Index::new(source));
        let declaration = "party @p \"Pé\"";
        let start = source.find(declaration).unwrap();
        let units = |text: &str| u32::try_from(text.encode_utf16().count()).unwrap();
        assert_eq!(
            found[0],
            Diagnostic {
                code: "unused-declaration",
                severity: "warning",
                range: Range {
                    start: units(&source[..start]),
                    end: units(&source[..start + declaration.len()]),
                },
                message: found[0].message.clone(),
            }
        );
    }

    #[test]
    fn a_rejection_gives_the_index_of_the_nul_character() {
        let source = "é😀\0";
        let rejection = oval_core::check(source).unwrap_err();
        assert_eq!(
            rejection_message(rejection, &Utf16Index::new(source)),
            "the document contains U+0000 at index 3"
        );
        assert_eq!(
            rejection_message(InputRejection::TooLarge, &Utf16Index::new(source)),
            "the document is too large"
        );
    }

    #[test]
    fn an_invalid_document_says_how_many_errors_it_has() {
        assert_eq!(invalid_message(1), "the document has an error");
        assert_eq!(invalid_message(3), "the document has 3 errors");
    }
}
