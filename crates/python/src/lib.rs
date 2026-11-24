//! Python bindings: strings in and out of the core, nothing else.
//! Errors are raised as `ValueError`.

#![forbid(unsafe_code)]

use proforma_core::api;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn py(r: Result<String, String>) -> PyResult<String> {
    r.map_err(PyValueError::new_err)
}

/// Runs a deal (YAML or JSON) and returns the full result (JSON).
#[pyfunction]
fn run(deal: &str) -> PyResult<String> {
    py(api::run(deal))
}

/// Which leases and assumptions produced a line in a year or month (JSON).
#[pyfunction]
#[pyo3(signature = (deal, line, year=None, month=None))]
fn trace(deal: &str, line: &str, year: Option<u32>, month: Option<u32>) -> PyResult<String> {
    py(api::trace(deal, line, year, month))
}

/// A two-way sensitivity grid; `spec` is `{x, y, x_values, y_values}` JSON.
#[pyfunction]
fn sensitivity(deal: &str, spec: &str) -> PyResult<String> {
    py(api::sensitivity(deal, spec))
}

/// One report as CSV.
#[pyfunction]
fn csv(deal: &str, report: &str) -> PyResult<String> {
    py(api::csv(deal, report))
}

/// The deal re-written as YAML.
#[pyfunction]
fn to_yaml(deal: &str) -> PyResult<String> {
    py(api::to_yaml(deal))
}

/// The deal re-written as JSON.
#[pyfunction]
fn to_json(deal: &str) -> PyResult<String> {
    py(api::to_json(deal))
}

/// "ok", or raises with every validation error.
#[pyfunction]
fn validate(deal: &str) -> PyResult<String> {
    py(api::validate(deal))
}

/// The deal file JSON Schema.
#[pyfunction]
fn schema() -> &'static str {
    api::schema()
}

/// The compiled core.
#[pymodule]
fn _proforma(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(trace, m)?)?;
    m.add_function(wrap_pyfunction!(sensitivity, m)?)?;
    m.add_function(wrap_pyfunction!(csv, m)?)?;
    m.add_function(wrap_pyfunction!(to_yaml, m)?)?;
    m.add_function(wrap_pyfunction!(to_json, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(schema, m)?)?;
    m.add("__version__", api::version())?;
    Ok(())
}
