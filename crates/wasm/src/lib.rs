//! WebAssembly bindings: strings in and out of the core, nothing else.
//! Errors come back as thrown strings.

#![forbid(unsafe_code)]

use proforma_core::api;
use wasm_bindgen::prelude::*;

fn js(r: Result<String, String>) -> Result<String, JsValue> {
    r.map_err(|e| JsValue::from_str(&e))
}

/// Runs a deal (YAML or JSON) and returns the full result (JSON).
#[wasm_bindgen]
pub fn run(deal: &str) -> Result<String, JsValue> {
    js(api::run(deal))
}

/// Which leases and assumptions produced a line in a year or month (JSON).
#[wasm_bindgen]
pub fn trace(
    deal: &str,
    line: &str,
    year: Option<u32>,
    month: Option<u32>,
) -> Result<String, JsValue> {
    js(api::trace(deal, line, year, month))
}

/// A two-way sensitivity grid; `spec` is `{x, y, x_values, y_values}` JSON.
#[wasm_bindgen]
pub fn sensitivity(deal: &str, spec: &str) -> Result<String, JsValue> {
    js(api::sensitivity(deal, spec))
}

/// One report as CSV.
#[wasm_bindgen]
pub fn csv(deal: &str, report: &str) -> Result<String, JsValue> {
    js(api::csv(deal, report))
}

/// The deal re-written as YAML.
#[wasm_bindgen]
pub fn to_yaml(deal: &str) -> Result<String, JsValue> {
    js(api::to_yaml(deal))
}

/// The deal re-written as JSON.
#[wasm_bindgen]
pub fn to_json(deal: &str) -> Result<String, JsValue> {
    js(api::to_json(deal))
}

/// "ok", or every validation error.
#[wasm_bindgen]
pub fn validate(deal: &str) -> Result<String, JsValue> {
    js(api::validate(deal))
}

/// The deal file JSON Schema.
#[wasm_bindgen]
pub fn schema() -> String {
    api::schema().to_string()
}

/// Engine version.
#[wasm_bindgen]
pub fn version() -> String {
    api::version().to_string()
}
