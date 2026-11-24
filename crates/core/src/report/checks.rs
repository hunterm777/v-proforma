//! The PRD invariants, checked on every run and reported with the result.

use crate::engine::Outcome;
use crate::returns::npv;
use crate::trace::{Line, SourceKind};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

fn lease_sums(o: &Outcome) -> Check {
    let mut bad = 0;
    for line in [Line::BaseRent, Line::Recoveries, Line::FreeRent] {
        // Sum the lease and unit-type entries independently of `Ledger::total`.
        let mut s = vec![0i64; o.ledger.months];
        for e in o.ledger.entries.iter().filter(|e| {
            e.line == line && matches!(e.source.kind, SourceKind::Lease | SourceKind::UnitType)
        }) {
            for (a, c) in s.iter_mut().zip(&e.cents) {
                *a += c;
            }
        }
        bad += o
            .ledger
            .total(line)
            .iter()
            .zip(&s)
            .filter(|(t, s)| t != s)
            .count();
    }
    Check {
        name: "lease_rent_sums_to_property",
        passed: bad == 0,
        detail: format!("{bad} month-lines where lease-level rent, free rent or recoveries differ from the property line"),
    }
}

fn sources_uses(o: &Outcome) -> Check {
    let s = &o.sources_uses;
    let diff = s.equity + s.loan - (s.price + s.closing_costs + s.loan_fees);
    Check {
        name: "sources_equal_uses",
        passed: diff == 0,
        detail: format!("equity + debt - (price + costs + fees) = {diff} cents"),
    }
}

fn waterfall(o: &Outcome) -> Check {
    let Some(w) = &o.waterfall else {
        return Check {
            name: "waterfall_sums_to_levered",
            passed: true,
            detail: "no equity waterfall".into(),
        };
    };
    let mut bad = 0;
    for (t, cf) in o.levered.cash_flows.iter().enumerate() {
        let lp = w.lp.cash_flows[t];
        let gp = w.gp.cash_flows[t];
        bad += usize::from(lp + gp != *cf);
    }
    let mut split_bad = 0;
    for (i, tier) in w.tiers.iter().enumerate() {
        let s = &o.model.deal.equity.as_ref().expect("equity").tiers[i].split;
        let lp_share = match s {
            crate::deal::Split::Shares(x) => x.lp,
            _ => {
                o.model
                    .deal
                    .equity
                    .as_ref()
                    .expect("equity")
                    .contributions
                    .lp
            }
        };
        for t in 0..tier.lp.len() {
            let amt = tier.lp[t] + tier.gp[t];
            split_bad += usize::from(tier.lp[t] != crate::money::round(amt as f64 * lp_share));
        }
    }
    Check {
        name: "waterfall_sums_to_levered",
        passed: bad == 0 && split_bad == 0,
        detail: format!("{bad} months where LP + GP differ from levered cash flow; {split_bad} tier allocations off their split"),
    }
}

fn irr(o: &Outcome) -> Check {
    let mut worst: f64 = 0.0;
    let mut n = 0;
    let mut series: Vec<(Vec<i64>, Option<f64>)> = vec![
        (o.unlevered.cash_flows.clone(), o.unlevered.irr_monthly),
        (o.levered.cash_flows.clone(), o.levered.irr_monthly),
    ];
    if let Some(w) = &o.waterfall {
        for p in [&w.lp, &w.gp] {
            let f: Vec<f64> = p.cash_flows.iter().map(|c| *c as f64).collect();
            series.push((p.cash_flows.clone(), crate::returns::irr_monthly(&f)));
        }
    }
    for (cf, r) in series {
        if let Some(r) = r {
            let f: Vec<f64> = cf.iter().map(|c| *c as f64).collect();
            let gross: f64 = f.iter().map(|c| c.abs()).sum();
            worst = worst.max(npv(&f, r).abs() / gross);
            n += 1;
        }
    }
    Check {
        name: "irr_reprices_to_zero",
        passed: worst <= 1e-9,
        detail: format!("{n} IRRs; largest |NPV| / sum |cash flow| = {worst:e}"),
    }
}

pub fn all(o: &Outcome) -> Vec<Check> {
    vec![lease_sums(o), sources_uses(o), waterfall(o), irr(o)]
}
