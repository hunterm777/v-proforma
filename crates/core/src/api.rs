//! The string API the WASM and Python bindings wrap: deal text (YAML or
//! JSON) in, JSON or CSV out. Bindings hold no math.

use crate::calendar::Calendar;
use crate::deal;
use crate::report;
use crate::sensitivity;

fn json<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|e| e.to_string())
}

/// Runs a deal and returns the full result as JSON.
pub fn run(deal_text: &str) -> Result<String, String> {
    let d = deal::parse(deal_text)?;
    let o = crate::engine::run(&d)?;
    json(&report::output(&o))
}

/// Which entries produced `line` in analysis `year` (1-based) or model
/// `month` (1-based); neither means the whole hold. Lines below cash flow
/// before debt (debt service, purchase, loan, sale, payoff and the cash flow
/// series) also take year 0 or month 0, the close.
pub fn trace(
    deal_text: &str,
    line: &str,
    year: Option<u32>,
    month: Option<u32>,
) -> Result<String, String> {
    let d = deal::parse(deal_text)?;
    let o = crate::engine::run(&d)?;
    let x = o.model.exit_month;
    let finance = report::finance::LINES.contains(&line);
    let first = usize::from(!finance);
    let (from, to) = match (year, month) {
        (Some(_), Some(_)) => return Err("give a year or a month, not both".into()),
        (Some(y), None) => {
            let y = y as usize;
            if y < first || y > Calendar::years(x) {
                return Err(format!("year must be {first} to {}", Calendar::years(x)));
            }
            if y == 0 {
                (0, 0)
            } else {
                ((y - 1) * 12 + 1, (y * 12).min(x))
            }
        }
        (None, Some(m)) => (m as usize, m as usize),
        (None, None) => (first, x),
    };
    if finance {
        return json(&report::finance::trace(&o, line, from, to)?);
    }
    json(&crate::trace::trace(&o.ledger, line, from, to)?)
}

/// A two-way sensitivity grid; `spec` is JSON `{x, y, x_values, y_values}`.
pub fn sensitivity(deal_text: &str, spec: &str) -> Result<String, String> {
    let d = deal::parse(deal_text)?;
    let s: sensitivity::Spec =
        serde_json::from_str(spec).map_err(|e| format!("sensitivity spec: {e}"))?;
    json(&sensitivity::run(&d, &s)?)
}

/// One report as CSV (see `report::csv::REPORTS`).
pub fn csv(deal_text: &str, report: &str) -> Result<String, String> {
    let d = deal::parse(deal_text)?;
    let o = crate::engine::run(&d)?;
    report::csv::csv(&report::output(&o), report)
}

/// The deal re-written as YAML (validated first).
pub fn to_yaml(deal_text: &str) -> Result<String, String> {
    let d = deal::parse(deal_text)?;
    deal::validate(&d)?;
    deal::to_yaml(&d)
}

/// The deal re-written as pretty JSON (validated first).
pub fn to_json(deal_text: &str) -> Result<String, String> {
    let d = deal::parse(deal_text)?;
    deal::validate(&d)?;
    serde_json::to_string_pretty(&d).map_err(|e| e.to_string())
}

/// Validates a deal; returns "ok" or every error.
pub fn validate(deal_text: &str) -> Result<String, String> {
    let d = deal::parse(deal_text)?;
    deal::validate(&d).map(|_| "ok".into())
}

/// The deal file JSON Schema.
pub fn schema() -> &'static str {
    include_str!("../../../schema/deal.schema.json")
}

/// Engine version.
pub fn version() -> &'static str {
    report::VERSION
}
