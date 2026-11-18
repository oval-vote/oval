//! WebAssembly bindings for the Oval voting guide format.
//!
//! This crate is the source of the `@oval-vote/oval` npm package, built with `wasm-pack`. Every
//! function is synchronous. A panic, which is always a bug, prints its message to the console
//! before the call fails.
//!
//! Positions in a document are JavaScript string indices, which count UTF-16 code units, so
//! `source.slice(range.start, range.end)` is the text a range spans. Results are plain objects and
//! arrays, whose TypeScript types are declared alongside the functions.

mod utf16;

use oval_core::{Report, Severity};
use serde::Serialize;
use wasm_bindgen::prelude::{JsError, JsValue, wasm_bindgen};

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
/// Exported to JavaScript as `check(source)`, which returns `Diagnostic[]`. It throws an `Error`
/// when the source can't be an Oval document at all, because it contains U+0000 or is too large.
#[wasm_bindgen(unchecked_return_type = "Diagnostic[]")]
pub fn check(source: &str) -> Result<JsValue, JsError> {
    let report = oval_core::check(source)?;
    let diagnostics = diagnostics(&report, &Utf16Index::new(source));
    Ok(serde_wasm_bindgen::to_value(&diagnostics)?)
}

#[derive(Debug, PartialEq, Serialize)]
struct Diagnostic {
    code: &'static str,
    severity: &'static str,
    range: Range,
    message: String,
}

fn diagnostics(report: &Report, index: &Utf16Index) -> Vec<Diagnostic> {
    report
        .diagnostics()
        .iter()
        .map(|diagnostic| Diagnostic {
            code: diagnostic.code.spec_id(),
            severity: match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            range: index.range(diagnostic.range),
            message: diagnostic.message.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use oval_core::DiagnosticCode;

    use super::{Diagnostic, diagnostics};
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
        let found = diagnostics(&report, &Utf16Index::new(source));
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
}
