//! Reports: the full result a run returns, as one serializable value.
//! Money is integer cents; formatting to dollars happens in CSV and the page.

use crate::calendar::Calendar;
use crate::debt::DebtResult;
use crate::engine::{Outcome, SourcesUses};
use crate::exit::ExitResult;
use crate::money::Cents;
use crate::returns::Series;
use crate::trace::{step_name, sum, Line, Source};
use crate::waterfall::WaterfallResult;
use labels::label;
use serde::Serialize;

mod checks;
pub mod csv;
pub mod finance;
mod labels;
mod rentroll;
pub use checks::Check;
pub use rentroll::{RentRoll, TenantRow};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize)]
pub struct DealBar {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: crate::deal::PropertyType,
    pub price: Cents,
    /// Year-1 NOI / price.
    pub going_in_cap: f64,
    pub levered_irr: Option<f64>,
    pub equity_multiple: Option<f64>,
    pub unlevered_irr: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub key: String,
    pub label: String,
    /// Signed cents (deductions negative).
    pub values: Vec<Cents>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EntryOut {
    pub line: Line,
    pub source: Source,
    pub step: &'static str,
    pub detail: String,
    /// Sum by analysis year over the hold.
    pub years: Vec<Cents>,
    /// Model month of `values[0]`; leading and trailing zero months are cut.
    pub from_month: usize,
    pub values: Vec<Cents>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DebtYear {
    pub year: usize,
    pub debt_service: Cents,
    pub dscr: Option<f64>,
    pub debt_yield: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DebtReport {
    #[serde(flatten)]
    pub result: DebtResult,
    pub by_year: Vec<DebtYear>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Returns {
    pub unlevered: Series,
    pub levered: Series,
    pub sources_uses: SourcesUses,
}

/// The whole result. The deal file plus `engine` reproduce it exactly.
#[derive(Clone, Debug, Serialize)]
pub struct Output {
    pub engine: &'static str,
    pub deal_bar: DealBar,
    pub start_date: String,
    pub exit_month: usize,
    pub model_months: usize,
    pub annual: Table,
    pub monthly: Table,
    pub entries: Vec<EntryOut>,
    pub rent_roll: RentRoll,
    pub tenants: Vec<TenantRow>,
    pub debt: Option<DebtReport>,
    pub exit: ExitResult,
    pub returns: Returns,
    pub waterfall: Option<WaterfallResult>,
    pub checks: Vec<Check>,
}

/// Report line keys in order: primitives with derived subtotals between them.
pub const ORDER: [&str; 16] = [
    "base_rent",
    "recoveries",
    "other_income",
    "pgr",
    "general_vacancy",
    "credit_loss",
    "free_rent",
    "egi",
    "opex",
    "management_fee",
    "noi",
    "ti",
    "lc",
    "reserves",
    "capex",
    "cash_flow_before_debt",
];

/// A report line in signed cents: deductions negative (see `trace::components`).
fn signed(o: &Outcome, key: &str) -> Vec<Cents> {
    o.ledger.line(key).expect("known line")
}

fn by_year(v: &[Cents], x: usize) -> Vec<Cents> {
    (0..Calendar::years(x))
        .map(|y| sum(v, y * 12 + 1, ((y + 1) * 12).min(x)))
        .collect()
}

fn tables(o: &Outcome) -> (Table, Table) {
    let x = o.model.exit_month;
    let years = Calendar::years(x);
    let mut monthly = Vec::new();
    let mut annual = Vec::new();
    let ds: Vec<Cents> = (0..x)
        .map(|i| -o.debt.as_ref().map_or(0, |d| d.schedule[i].payment))
        .collect();
    let mut op: Vec<(String, Vec<Cents>)> = ORDER
        .iter()
        .map(|k| (k.to_string(), signed(o, k)[..x].to_vec()))
        .collect();
    let cfad: Vec<Cents> = o.cfbd[..x].iter().zip(&ds).map(|(a, b)| a + b).collect();
    op.push(("debt_service".into(), ds));
    op.push(("cash_flow_after_debt".into(), cfad));
    for (k, v) in &op {
        let mut a = vec![0];
        a.extend(by_year(v, x));
        annual.push(Row {
            key: k.clone(),
            label: label(k).into(),
            values: a,
        });
        monthly.push(Row {
            key: k.clone(),
            label: label(k).into(),
            values: v.clone(),
        });
    }
    let mut cap = |key: &str, at0: Cents, at_exit: Cents| {
        let mut v = vec![0; years + 1];
        v[0] = at0;
        v[years] += at_exit;
        annual.push(Row {
            key: key.into(),
            label: label(key).into(),
            values: v,
        });
    };
    let su = &o.sources_uses;
    cap("purchase", -(su.price + su.closing_costs), 0);
    cap("loan_net", su.loan - su.loan_fees, 0);
    cap("net_sale", 0, o.exit.net_sale);
    cap(
        "loan_payoff",
        0,
        -o.debt.as_ref().map_or(0, |d| d.payoff + d.prepayment),
    );
    for (key, s) in [
        ("unlevered_cash_flow", &o.unlevered),
        ("levered_cash_flow", &o.levered),
    ] {
        let cf = &s.cash_flows;
        let mut v = vec![cf[0]];
        v.extend(by_year(&cf[1..], x));
        annual.push(Row {
            key: key.into(),
            label: label(key).into(),
            values: v,
        });
        monthly.push(Row {
            key: key.into(),
            label: label(key).into(),
            values: cf[1..].to_vec(),
        });
    }
    let mut cols = vec!["Year 0".to_string()];
    cols.extend((1..=years).map(|y| format!("Year {y}")));
    let mcols = (1..=x as i64)
        .map(|m| o.model.cal.month_start(m).to_string()[..7].to_string())
        .collect();
    (
        Table {
            columns: cols,
            rows: annual,
        },
        Table {
            columns: mcols,
            rows: monthly,
        },
    )
}

fn entries(o: &Outcome) -> Vec<EntryOut> {
    let x = o.model.exit_month;
    o.ledger
        .entries
        .iter()
        .map(|e| {
            let first = e.cents.iter().position(|c| *c != 0).unwrap_or(0);
            let last = e.cents.iter().rposition(|c| *c != 0).unwrap_or(0);
            EntryOut {
                line: e.line,
                source: e.source.clone(),
                step: step_name(e.line.step()),
                detail: e.detail.clone(),
                years: by_year(&e.cents, x),
                from_month: first + 1,
                values: e.cents[first..=last].to_vec(),
            }
        })
        .collect()
}

fn debt_report(o: &Outcome) -> Option<DebtReport> {
    let d = o.debt.clone()?;
    let x = o.model.exit_month;
    let by_year = (0..Calendar::years(x))
        .map(|y| {
            let (a, b) = (y * 12 + 1, ((y + 1) * 12).min(x));
            let ds: Cents = d.schedule[a - 1..b].iter().map(|p| p.payment).sum();
            let noi = sum(&o.noi, a, b);
            let open = d.schedule[a - 1].opening;
            DebtYear {
                year: y + 1,
                debt_service: ds,
                dscr: (ds > 0).then(|| noi as f64 / ds as f64),
                debt_yield: (open > 0).then(|| noi as f64 / open as f64),
            }
        })
        .collect();
    Some(DebtReport { result: d, by_year })
}

/// Builds the full output of a run.
pub fn output(o: &Outcome) -> Output {
    let (annual, monthly) = tables(o);
    let d = &o.model.deal;
    let noi1 = sum(&o.noi, 1, 12);
    Output {
        engine: VERSION,
        deal_bar: DealBar {
            name: d.property.name.clone(),
            kind: d.property.kind,
            price: o.model.price,
            going_in_cap: noi1 as f64 / o.model.price as f64,
            levered_irr: o.levered.irr,
            equity_multiple: o.levered.equity_multiple,
            unlevered_irr: o.unlevered.irr,
        },
        start_date: o.model.cal.start.to_string(),
        exit_month: o.model.exit_month,
        model_months: o.model.cal.months,
        annual,
        monthly,
        entries: entries(o),
        rent_roll: rentroll::summary(o),
        tenants: rentroll::tenants(o),
        debt: debt_report(o),
        exit: o.exit.clone(),
        returns: Returns {
            unlevered: o.unlevered.clone(),
            levered: o.levered.clone(),
            sources_uses: o.sources_uses.clone(),
        },
        waterfall: o.waterfall.clone(),
        checks: checks::all(o),
    }
}
