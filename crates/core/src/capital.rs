//! Capital (step 6): reserves and one-off capex.

use crate::deal::Capital;
use crate::money::cents;
use crate::trace::{Ledger, Line, Source, SourceKind};

/// Adds capital reserves (flat per SF or unit) and capex items. Capex beyond
/// the exit month is rejected by validation; reserves stop at the exit month.
pub fn entries(ledger: &mut Ledger, c: &Capital, basis: f64, exit_month: usize) {
    let (rate, what) = match (c.reserves_per_sf_yr, c.reserves_per_unit_yr) {
        (Some(r), _) => (r, "SF"),
        (_, Some(r)) => (r, "unit"),
        _ => (0.0, "SF"),
    };
    let monthly = rate * basis * 100.0 / 12.0;
    let v: Vec<f64> = (1..=ledger.months)
        .map(|m| if m <= exit_month { monthly } else { 0.0 })
        .collect();
    let id = if what == "SF" {
        "capital.reserves_per_sf_yr"
    } else {
        "capital.reserves_per_unit_yr"
    };
    let detail = format!("${rate}/{what}/yr x {basis} {what}");
    ledger.push(
        Line::Reserves,
        Source::new(SourceKind::Assumption, id),
        detail,
        &v,
    );
    for x in &c.capex {
        let mut v = vec![0; ledger.months];
        v[x.month as usize - 1] = cents(x.amount);
        let detail = format!("{} in month {}", x.name, x.month);
        ledger.push_cents(
            Line::Capex,
            Source::new(SourceKind::Capex, &x.id),
            detail,
            v,
        );
    }
}
