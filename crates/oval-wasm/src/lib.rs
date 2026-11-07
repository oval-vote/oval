//! WebAssembly bindings for the Oval voting guide format.

use oval_core::{CompileError, DiagnosticCode, InputRejection, Report, Severity};
use oval_format::FormatError;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// The TypeScript declarations appended to the package's typings.
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

/// The major version of the Oval language this package reads.
#[wasm_bindgen(js_name = languageMajor)]
pub fn language_major() -> u32 {
    oval_core::SUPPORTED_LANGUAGE_MAJOR
}

/// The version of this package.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Every diagnostic the implementation can report, in the specification's order, with the
/// address of its entry on the specification's site.
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
#[wasm_bindgen(unchecked_return_type = "Diagnostic[]")]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    diagnostics(&oval_core::check(source).map_err(rejected)?)
}

/// Compiles `source` to its canonical form, or fails with the diagnostics.
#[wasm_bindgen(unchecked_return_type = "Compiled")]
pub fn compile(source: &str) -> Result<JsValue, JsValue> {
    match oval_core::compile(source) {
        Ok(compiled) => serde_wasm_bindgen::to_value(&compiled).map_err(into_js),
        Err(CompileError::Rejected(rejection)) => Err(rejected(rejection)),
        Err(CompileError::Invalid(report)) => Err(diagnostics(&report)?),
    }
}

/// Formats `source`, or fails with the diagnostics.
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
