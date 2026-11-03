//! WebAssembly bindings for the Oval voting guide format.

use oval_core::Report;
use wasm_bindgen::prelude::*;

/// Parses and validates `source`, returning every diagnostic in source order.
#[wasm_bindgen]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    diagnostics(&oval_core::check(source))
}

/// Compiles `source` to its canonical form, or fails with the diagnostics.
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<JsValue, JsValue> {
    match oval_core::compile(source) {
        Ok(compiled) => serde_wasm_bindgen::to_value(&compiled).map_err(into_js),
        Err(report) => Err(diagnostics(&report)?),
    }
}

/// Formats `source`, or fails with the diagnostics.
#[wasm_bindgen]
pub fn format(source: &str) -> Result<String, JsValue> {
    match oval_format::format(source) {
        Ok(formatted) => Ok(formatted),
        Err(_) => Err(diagnostics(&oval_core::check(source))?),
    }
}

fn diagnostics(report: &Report) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(report.diagnostics()).map_err(into_js)
}

fn into_js(error: serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from(error)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_npm_package_has_the_workspace_version() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../../packages/oval/package.json"))
                .expect("package.json is JSON");
        assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    }
}
