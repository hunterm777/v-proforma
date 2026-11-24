//! The audit trail: every monthly cell is an entry that knows its source.

use crate::money::{round, Cents};
use serde::Serialize;

/// A primitive cash-flow line. Amounts are positive; derived lines apply signs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Line {
    BaseRent,
    Recoveries,
    OtherIncome,
    GeneralVacancy,
    CreditLoss,
    FreeRent,
    Opex,
    ManagementFee,
    Ti,
    Lc,
    Reserves,
    Capex,
}

impl Line {
    pub const ALL: [Line; 12] = [
        Line::BaseRent,
        Line::Recoveries,
        Line::OtherIncome,
        Line::GeneralVacancy,
        Line::CreditLoss,
        Line::FreeRent,
        Line::Opex,
        Line::ManagementFee,
        Line::Ti,
        Line::Lc,
        Line::Reserves,
        Line::Capex,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Line::BaseRent => "base_rent",
            Line::Recoveries => "recoveries",
            Line::OtherIncome => "other_income",
            Line::GeneralVacancy => "general_vacancy",
            Line::CreditLoss => "credit_loss",
            Line::FreeRent => "free_rent",
            Line::Opex => "opex",
            Line::ManagementFee => "management_fee",
            Line::Ti => "ti",
            Line::Lc => "lc",
            Line::Reserves => "reserves",
            Line::Capex => "capex",
        }
    }

    /// The PRD calculation step that produces this line.
    pub fn step(self) -> u8 {
        match self {
            Line::BaseRent | Line::FreeRent | Line::Ti | Line::Lc => 1,
            Line::Recoveries => 2,
            Line::OtherIncome => 3,
            Line::GeneralVacancy | Line::CreditLoss => 4,
            Line::Opex | Line::ManagementFee => 5,
            Line::Reserves | Line::Capex => 6,
        }
    }
}

/// PRD step names, by number.
pub fn step_name(step: u8) -> &'static str {
    match step {
        1 => "1 Lease cash flows",
        2 => "2 Recoveries",
        3 => "3 Potential gross revenue",
        4 => "4 Effective gross income",
        5 => "5 NOI",
        6 => "6 Cash flow before debt",
        _ => "",
    }
}

/// The primitive lines in a line, with their signs (deductions negative).
pub fn components(name: &str) -> Option<Vec<(Line, i64)>> {
    use Line::*;
    let pgr = vec![(BaseRent, 1), (Recoveries, 1), (OtherIncome, 1)];
    let mut egi = pgr.clone();
    egi.extend([(GeneralVacancy, -1), (CreditLoss, -1), (FreeRent, -1)]);
    let mut noi = egi.clone();
    noi.extend([(Opex, -1), (ManagementFee, -1)]);
    let mut cfbd = noi.clone();
    cfbd.extend([(Ti, -1), (Lc, -1), (Reserves, -1), (Capex, -1)]);
    Some(match name {
        "pgr" => pgr,
        "egi" => egi,
        "noi" => noi,
        "cash_flow_before_debt" => cfbd,
        // A primitive line carries the sign it has in cash flow before debt,
        // so deductions trace as negative numbers, as in the reports.
        _ => vec![*cfbd.iter().find(|(l, _)| l.key() == name)?],
    })
}

/// The derived lines, in report order.
pub const DERIVED: [&str; 4] = ["pgr", "egi", "noi", "cash_flow_before_debt"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Lease,
    UnitType,
    Expense,
    Income,
    Capex,
    Assumption,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Source {
    pub kind: SourceKind,
    pub id: String,
}

impl Source {
    pub fn new(kind: SourceKind, id: impl Into<String>) -> Source {
        Source {
            kind,
            id: id.into(),
        }
    }
}

/// One source's contribution to one line, every model month.
#[derive(Clone, Debug)]
pub struct Entry {
    pub line: Line,
    pub source: Source,
    pub detail: String,
    /// Cents by model month; index 0 is month 1.
    pub cents: Vec<Cents>,
}

/// All entries of a run.
#[derive(Clone, Debug, Default)]
pub struct Ledger {
    pub months: usize,
    pub entries: Vec<Entry>,
}

impl Ledger {
    pub fn new(months: usize) -> Ledger {
        Ledger {
            months,
            entries: Vec::new(),
        }
    }

    /// Adds an entry from unrounded cents, rounding each month once.
    /// All-zero entries are dropped.
    pub fn push(&mut self, line: Line, source: Source, detail: impl Into<String>, values: &[f64]) {
        let cents: Vec<Cents> = values.iter().map(|v| round(*v)).collect();
        self.push_cents(line, source, detail, cents);
    }

    pub fn push_cents(
        &mut self,
        line: Line,
        source: Source,
        detail: impl Into<String>,
        cents: Vec<Cents>,
    ) {
        debug_assert_eq!(cents.len(), self.months);
        if cents.iter().any(|c| *c != 0) {
            self.entries.push(Entry {
                line,
                source,
                detail: detail.into(),
                cents,
            });
        }
    }

    /// The property-level total of a primitive line, by month.
    pub fn total(&self, line: Line) -> Vec<Cents> {
        let mut t = vec![0; self.months];
        for e in self.entries.iter().filter(|e| e.line == line) {
            for (a, b) in t.iter_mut().zip(&e.cents) {
                *a += b;
            }
        }
        t
    }

    /// A primitive or derived line by key, by month.
    pub fn line(&self, name: &str) -> Option<Vec<Cents>> {
        let mut t = vec![0; self.months];
        for (line, sign) in components(name)? {
            for (a, b) in t.iter_mut().zip(self.total(line)) {
                *a += sign * b;
            }
        }
        Some(t)
    }
}

/// Sum of `v` over model months `[from, to]` (1-based, inclusive).
pub fn sum(v: &[Cents], from: usize, to: usize) -> Cents {
    v[from - 1..to].iter().sum()
}

/// One row of a trace: one entry's contribution to a line over a period.
#[derive(Clone, Debug, Serialize)]
pub struct TraceRow {
    pub source: Source,
    pub line: Line,
    pub step: &'static str,
    pub detail: String,
    /// Signed contribution to the traced line, in cents.
    pub cents: Cents,
}

#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    pub line: String,
    pub from_month: usize,
    pub to_month: usize,
    pub total: Cents,
    pub rows: Vec<TraceRow>,
}

/// Which entries produced `name` over months `[from, to]`, largest first.
pub fn trace(ledger: &Ledger, name: &str, from: usize, to: usize) -> Result<Trace, String> {
    let comps = components(name).ok_or_else(|| format!("unknown line \"{name}\""))?;
    if from < 1 || to > ledger.months || from > to {
        return Err(format!(
            "months {from} to {to} are outside the model (1 to {})",
            ledger.months
        ));
    }
    let mut rows: Vec<TraceRow> = Vec::new();
    for e in &ledger.entries {
        if let Some((_, sign)) = comps.iter().find(|(l, _)| *l == e.line) {
            let c = sign * sum(&e.cents, from, to);
            if c != 0 {
                rows.push(TraceRow {
                    source: e.source.clone(),
                    line: e.line,
                    step: step_name(e.line.step()),
                    detail: e.detail.clone(),
                    cents: c,
                });
            }
        }
    }
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
