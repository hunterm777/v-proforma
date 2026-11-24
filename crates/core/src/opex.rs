//! Operating expenses and other income: line amounts and the recoverable pool.

use crate::calendar::Calendar;
use crate::deal::{ExpenseLine, IncomeLine};
use crate::money::grow;
use crate::trace::{Ledger, Line, Source, SourceKind};

/// Monthly amount in cents of a line with annual `amount_yr` dollars and
/// `growth`, in any model month (also before the start or past the horizon).
pub fn monthly(amount_yr: f64, growth: f64, m: i64) -> f64 {
    amount_yr * 100.0 * grow(growth, Calendar::year_of(m)) / 12.0
}

/// The recoverable expense pool, in unrounded cents.
#[derive(Clone, Debug)]
pub struct Pool<'a> {
    lines: Vec<&'a ExpenseLine>,
}

impl<'a> Pool<'a> {
    pub fn new(lines: &'a [ExpenseLine]) -> Pool<'a> {
        Pool {
            lines: lines.iter().filter(|l| l.recoverable).collect(),
        }
    }

    /// (controllable, non-controllable) pool in month `m`, limited to `only`
    /// line ids when given.
    pub fn split(&self, m: i64, only: Option<&[String]>) -> (f64, f64) {
        let mut c = 0.0;
        let mut n = 0.0;
        for l in &self.lines {
            if only.is_some_and(|ids| !ids.contains(&l.id)) {
                continue;
            }
            let v = monthly(l.amount_yr, l.growth, m);
            if l.controllable {
                c += v;
            } else {
                n += v;
            }
        }
        (c, n)
    }

    /// The whole uncapped pool in month `m`.
    pub fn total(&self, m: i64, only: Option<&[String]>) -> f64 {
        let (c, n) = self.split(m, only);
        c + n
    }
}

/// Adds one entry per expense line (step 5).
pub fn expenses(ledger: &mut Ledger, lines: &[ExpenseLine]) {
    for l in lines {
        let v: Vec<f64> = (1..=ledger.months as i64)
            .map(|m| monthly(l.amount_yr, l.growth, m))
            .collect();
        let detail = format!("{} ${}/yr, growth {}", l.name, l.amount_yr, l.growth);
        ledger.push(
            Line::Opex,
            Source::new(SourceKind::Expense, &l.id),
            detail,
            &v,
        );
    }
}

/// Adds one entry per other-income line (step 3).
pub fn income(ledger: &mut Ledger, lines: &[IncomeLine]) {
    for l in lines {
        let v: Vec<f64> = (1..=ledger.months as i64)
            .map(|m| monthly(l.amount_yr, l.growth, m))
            .collect();
        let detail = format!("{} ${}/yr, growth {}", l.name, l.amount_yr, l.growth);
        ledger.push(
            Line::OtherIncome,
            Source::new(SourceKind::Income, &l.id),
            detail,
            &v,
        );
    }
}
