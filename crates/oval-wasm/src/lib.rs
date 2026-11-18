//! WebAssembly bindings for the Oval voting guide format.
//!
//! This crate is the source of the `@oval-vote/oval` npm package, built with `wasm-pack`. Every
//! function is synchronous. A panic, which is always a bug, prints its message to the console
//! before the call fails.

use wasm_bindgen::prelude::wasm_bindgen;

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
