//! Revenue (steps 3 to 5 assumptions): general vacancy, credit loss and the
//! management fee, each one entry sourced to its assumption.

use crate::deal::Revenue;
use crate::money::Cents;
use crate::trace::{Ledger, Line, Source, SourceKind};

/// Adds general vacancy and credit loss. `turnover` is modeled downtime
/// vacancy in whole cents (the sum of rounded path cells).
pub fn vacancy(ledger: &mut Ledger, r: &Revenue, turnover: &[f64]) {
    let pgr = ledger.line("pgr").expect("pgr");
    let v: Vec<f64> = pgr
        .iter()
        .zip(turnover)
        .map(|(p, t)| {
            let gross = r.general_vacancy_pct * *p as f64;
            if r.vacancy_on_top_of_downtime {
                gross
            } else {
                (gross - t).max(0.0)
            }
        })
        .collect();
    let detail = if r.vacancy_on_top_of_downtime {
        format!("{} of PGR", r.general_vacancy_pct)
    } else {
        format!(
            "{} of PGR, less modeled downtime vacancy",
            r.general_vacancy_pct
        )
    };
    let src = Source::new(SourceKind::Assumption, "revenue.general_vacancy_pct");
    ledger.push(Line::GeneralVacancy, src, detail, &v);
    let c: Vec<f64> = pgr.iter().map(|p| r.credit_loss_pct * *p as f64).collect();
    let src = Source::new(SourceKind::Assumption, "revenue.credit_loss_pct");
    ledger.push(
        Line::CreditLoss,
        src,
        format!("{} of PGR", r.credit_loss_pct),
        &c,
    );
}

/// Adds the management fee on EGI.
pub fn management_fee(ledger: &mut Ledger, pct: f64) {
    let egi = ledger.line("egi").expect("egi");
    let v: Vec<f64> = egi.iter().map(|e| pct * *e as f64).collect();
    let src = Source::new(SourceKind::Assumption, "opex.management_fee_pct");
    ledger.push(Line::ManagementFee, src, format!("{pct} of EGI"), &v);
}

/// Total turnover vacancy, rounded, for reports.
pub fn rounded(v: &[f64]) -> Vec<Cents> {
    v.iter().map(|x| crate::money::round(*x)).collect()
}
