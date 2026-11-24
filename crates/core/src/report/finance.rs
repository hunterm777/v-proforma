//! The audit trail below cash flow before debt: debt service, purchase, loan
//! funding, sale, payoff and the two cash flow series. Each row names the deal
//! input it comes from, so every number in the annual table can be traced.

use crate::engine::Outcome;
use crate::money::Cents;
use crate::trace::{Source, SourceKind};
use serde::Serialize;

/// The lines `trace` handles here, in report order.
pub const LINES: [&str; 8] = [
    "debt_service",
    "cash_flow_after_debt",
    "purchase",
    "loan_net",
    "net_sale",
    "loan_payoff",
    "unlevered_cash_flow",
    "levered_cash_flow",
];

#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub source: Source,
    pub line: String,
    pub step: &'static str,
    pub detail: String,
    /// Signed contribution to the traced line, in cents.
    pub cents: Cents,
}

/// Same shape as `trace::Trace`; month 0 is the close.
#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    pub line: String,
    pub from_month: usize,
    pub to_month: usize,
    pub total: Cents,
    pub rows: Vec<Row>,
}

fn row(id: &str, line: &str, step: &'static str, detail: String, cents: Cents) -> Row {
    Row {
        source: Source::new(SourceKind::Assumption, id),
        line: line.into(),
        step,
        detail,
        cents,
    }
}

const DEBT: &str = "7 Debt";
const EXIT: &str = "8 Exit";
const CLOSE: &str = "Capital at close";

fn debt_service(o: &Outcome, from: usize, to: usize) -> Vec<Row> {
    let Some(d) = &o.debt else { return Vec::new() };
    let (a, b) = (from.max(1), to.min(o.model.exit_month));
    if a > b {
        return Vec::new();
    }
    let p = &d.schedule[a - 1..b];
    let interest: Cents = p.iter().map(|x| x.interest).sum();
    let principal: Cents = p.iter().map(|x| x.principal).sum();
    vec![
        row(
            "debt",
            "interest",
            DEBT,
            "interest on the loan balance (30/360)".into(),
            -interest,
        ),
        row(
            "debt",
            "principal",
            DEBT,
            "scheduled amortization".into(),
            -principal,
        ),
    ]
}

fn close(o: &Outcome, from: usize, levered: bool) -> Vec<Row> {
    if from != 0 {
        return Vec::new();
    }
    let s = &o.sources_uses;
    let mut v = vec![
        row(
            "capital.price",
            "purchase",
            CLOSE,
            "purchase price".into(),
            -s.price,
        ),
        row(
            "capital.closing_costs",
            "purchase",
            CLOSE,
            "closing costs".into(),
            -s.closing_costs,
        ),
    ];
    if levered {
        v.extend(loan_net(o, from));
    }
    v
}

fn loan_net(o: &Outcome, from: usize) -> Vec<Row> {
    let Some(d) = &o.debt else { return Vec::new() };
    if from != 0 {
        return Vec::new();
    }
    let how = d.sizing.as_ref().map_or("loan amount".to_string(), |z| {
        format!("loan sized by {} (the lowest constraint)", z.binding)
    });
    vec![
        row("debt", "loan_net", DEBT, how, d.loan),
        row(
            "debt.origination_fee_pct",
            "loan_net",
            DEBT,
            "origination fee".into(),
            -d.fee,
        ),
    ]
}

fn sale(o: &Outcome, to: usize) -> Vec<Row> {
    let e = &o.exit;
    if !(e.month <= to) {
        return Vec::new();
    }
    vec![
        row(
            "exit.cap_rate",
            "net_sale",
            EXIT,
            format!(
                "forward 12-month NOI {} / cap rate {}",
                crate::report::csv::usd(e.forward_noi),
                e.cap_rate
            ),
            e.gross_sale,
        ),
        row(
            "exit.selling_costs_pct",
            "net_sale",
            EXIT,
            "selling costs".into(),
            -e.selling_costs,
        ),
    ]
}

fn payoff(o: &Outcome, to: usize) -> Vec<Row> {
    let Some(d) = &o.debt else { return Vec::new() };
    if o.model.exit_month > to {
        return Vec::new();
    }
    vec![
        row(
            "debt",
            "loan_payoff",
            DEBT,
            "loan balance repaid at sale".into(),
            -d.payoff,
        ),
        row(
            "debt.prepayment_penalty_pct",
            "loan_payoff",
            DEBT,
            format!("prepayment penalty at {}", d.prepayment_pct),
            -d.prepayment,
        ),
    ]
}

fn operating(o: &Outcome, from: usize, to: usize) -> Result<Vec<Row>, String> {
    let (a, b) = (from.max(1), to.min(o.model.exit_month));
    if a > b {
        return Ok(Vec::new());
    }
    let t = crate::trace::trace(&o.ledger, "cash_flow_before_debt", a, b)?;
    Ok(t.rows
        .into_iter()
        .map(|r| Row {
            source: r.source,
            line: r.line.key().into(),
            step: r.step,
            detail: r.detail,
            cents: r.cents,
        })
        .collect())
}

/// Traces one of `LINES` over months `[from, to]` (0 = the close).
pub fn trace(o: &Outcome, name: &str, from: usize, to: usize) -> Result<Trace, String> {
    let x = o.model.exit_month;
    if from > to || to > x {
        return Err(format!(
            "months {from} to {to} are outside the hold (0 to {x})"
        ));
    }
    let mut rows = match name {
        "debt_service" => debt_service(o, from, to),
        "cash_flow_after_debt" => [operating(o, from, to)?, debt_service(o, from, to)].concat(),
        "purchase" => close(o, from, false),
        "loan_net" => loan_net(o, from),
        "net_sale" => sale(o, to),
        "loan_payoff" => payoff(o, to),
        "unlevered_cash_flow" => {
            [close(o, from, false), operating(o, from, to)?, sale(o, to)].concat()
        }
        "levered_cash_flow" => [
            close(o, from, true),
            operating(o, from, to)?,
            debt_service(o, from, to),
            sale(o, to),
            payoff(o, to),
        ]
        .concat(),
        _ => return Err(format!("unknown line \"{name}\"")),
    };
    rows.retain(|r| r.cents != 0);
    rows.sort_by(|a, b| {
        b.cents
            .abs()
            .cmp(&a.cents.abs())
            .then(a.source.cmp(&b.source))
    });
    let total = rows.iter().map(|r| r.cents).sum();
    Ok(Trace {
        line: name.into(),
        from_month: from,
        to_month: to,
        total,
        rows,
    })
}
