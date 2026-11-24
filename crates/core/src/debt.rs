//! Debt (step 7): sizing at close, the monthly schedule, payoff at exit.

use crate::deal::{Debt, Floating, PctSchedule, Rate};
use crate::money::{pow, round, Cents};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    /// `ltv`, `dscr` or `debt_yield`.
    pub constraint: &'static str,
    pub amount: Cents,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sized {
    pub noi_year1: Cents,
    /// Annual rate used for DSCR sizing (month 1).
    pub rate: f64,
    pub candidates: Vec<Candidate>,
    pub binding: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Payment {
    pub month: usize,
    pub rate: f64,
    pub opening: Cents,
    pub interest: Cents,
    pub principal: Cents,
    pub payment: Cents,
    pub closing: Cents,
}

#[derive(Clone, Debug, Serialize)]
pub struct DebtResult {
    pub loan: Cents,
    pub fee: Cents,
    pub sizing: Option<Sized>,
    pub schedule: Vec<Payment>,
    pub payoff: Cents,
    pub prepayment_pct: f64,
    pub prepayment: Cents,
}

/// Validation of the debt block.
pub fn check(d: &Debt, e: &mut Vec<String>) {
    let mut c = |ok: bool, m: &str| {
        if !ok {
            e.push(format!("debt: {m}"));
        }
    };
    c(
        d.amount.is_some() != d.sizing.is_some(),
        "give either amount or sizing",
    );
    if let Some(a) = d.amount {
        c(a.is_finite() && a >= 0.0, "amount must be >= 0");
    }
    if let Some(s) = &d.sizing {
        c(
            s.max_ltv.is_some() || s.min_dscr.is_some() || s.min_debt_yield.is_some(),
            "sizing needs a constraint",
        );
        c(
            s.max_ltv.is_none_or(|x| x > 0.0 && x <= 1.0),
            "max_ltv must be in (0, 1]",
        );
        c(
            s.min_dscr.is_none_or(|x| x.is_finite() && x > 0.0),
            "min_dscr must be > 0",
        );
        c(
            s.min_debt_yield.is_none_or(|x| x.is_finite() && x > 0.0),
            "min_debt_yield must be > 0",
        );
    }
    c(
        d.rate.fixed.is_some() != d.rate.floating.is_some(),
        "rate needs exactly one of fixed or floating",
    );
    match (&d.rate.fixed, &d.rate.floating) {
        (Some(r), _) => c(r.is_finite() && *r >= 0.0, "rate.fixed must be >= 0"),
        (None, Some(Floating { spread, curve })) => {
            c(spread.is_finite(), "rate.floating.spread must be a number");
            c(
                curve.first().is_some_and(|p| p.month == 1),
                "rate.floating.curve must start at month 1",
            );
            c(
                curve.windows(2).all(|w| w[1].month > w[0].month),
                "rate.floating.curve months must increase",
            );
            c(
                curve
                    .iter()
                    .all(|p| p.index.is_finite() && p.index + spread >= 0.0),
                "floating rate must be >= 0",
            );
        }
        (None, None) => {}
    }
    c(
        d.amortization_years.is_none_or(|y| (1..=50).contains(&y)),
        "amortization_years must be 1 to 50",
    );
    c(
        d.origination_fee_pct.is_finite() && (0.0..=1.0).contains(&d.origination_fee_pct),
        "origination_fee_pct must be in [0, 1]",
    );
    let pp_ok = match &d.prepayment_penalty_pct {
        None => true,
        Some(PctSchedule::Flat(x)) => x.is_finite() && *x >= 0.0,
        Some(PctSchedule::ByLoanYear(v)) => {
            !v.is_empty() && v.iter().all(|x| x.is_finite() && *x >= 0.0)
        }
    };
    c(
        pp_ok,
        "prepayment_penalty_pct must be >= 0 (a number or a non-empty list)",
    );
}

/// Annual rate in model month `m`.
pub fn rate_at(r: &Rate, m: usize) -> f64 {
    match (&r.fixed, &r.floating) {
        (Some(x), _) => *x,
        (None, Some(Floating { spread, curve })) => {
            let p = curve
                .iter()
                .rev()
                .find(|p| p.month as usize <= m)
                .unwrap_or(&curve[0]);
            p.index + spread
        }
        (None, None) => 0.0,
    }
}

/// Level payment per dollar for `n` months at monthly rate `r`.
pub fn constant(r: f64, n: u32) -> f64 {
    if r == 0.0 {
        1.0 / f64::from(n)
    } else {
        r / (1.0 - pow(1.0 + r, -f64::from(n)))
    }
}

fn size(d: &Debt, price: Cents, noi1: Cents) -> (Cents, Option<Sized>) {
    let Some(s) = &d.sizing else {
        return (crate::money::cents(d.amount.unwrap_or(0.0)), None);
    };
    let rate = rate_at(&d.rate, 1);
    let mut cands = Vec::new();
    if let Some(x) = s.max_ltv {
        cands.push(Candidate {
            constraint: "ltv",
            amount: (x * price as f64).floor() as Cents,
        });
    }
    if let Some(x) = s.min_dscr {
        let k = match d.amortization_years {
            Some(y) => constant(rate / 12.0, y * 12),
            None => rate / 12.0,
        };
        if k > 0.0 {
            let amt = (noi1 as f64 / (x * 12.0 * k)).floor().max(0.0);
            cands.push(Candidate {
                constraint: "dscr",
                amount: amt as Cents,
            });
        }
    }
    if let Some(x) = s.min_debt_yield {
        cands.push(Candidate {
            constraint: "debt_yield",
            amount: (noi1 as f64 / x).floor().max(0.0) as Cents,
        });
    }
    let best = cands.iter().min_by_key(|c| c.amount).cloned();
    let (loan, binding) = best.map_or((0, "none"), |c| (c.amount, c.constraint));
    (
        loan,
        Some(Sized {
            noi_year1: noi1,
            rate,
            candidates: cands,
            binding,
        }),
    )
}

/// Sizes the loan and runs the schedule through `exit_month`.
pub fn run(d: &Debt, price: Cents, noi1: Cents, exit_month: usize) -> DebtResult {
    let (loan, sizing) = size(d, price, noi1);
    let fee = round(d.origination_fee_pct * loan as f64);
    let amort = d.amortization_years.map(|y| y * 12);
    let fixed = d.rate.fixed.is_some();
    let mut bal = loan;
    let mut level: Option<Cents> = None;
    let mut schedule = Vec::with_capacity(exit_month);
    for m in 1..=exit_month {
        let rate = rate_at(&d.rate, m);
        let r = rate / 12.0;
        let interest = round(bal as f64 * r);
        let j = m as i64 - i64::from(d.io_months);
        let principal = match amort {
            Some(n) if j >= 1 && j <= i64::from(n) && bal > 0 => {
                let left = n - (j as u32 - 1);
                let pay = if fixed {
                    *level.get_or_insert_with(|| round(bal as f64 * constant(r, left)))
                } else {
                    round(bal as f64 * constant(r, left))
                };
                if left == 1 {
                    bal
                } else {
                    (pay - interest).clamp(0, bal)
                }
            }
            _ => 0,
        };
        let closing = bal - principal;
        schedule.push(Payment {
            month: m,
            rate,
            opening: bal,
            interest,
            principal,
            payment: interest + principal,
            closing,
        });
        bal = closing;
    }
    let year = exit_month.div_ceil(12);
    let prepayment_pct = match &d.prepayment_penalty_pct {
        None => 0.0,
        Some(PctSchedule::Flat(x)) => *x,
        Some(PctSchedule::ByLoanYear(v)) => v[(year - 1).min(v.len() - 1)],
    };
    let prepayment = round(prepayment_pct * bal as f64);
    DebtResult {
        loan,
        fee,
        sizing,
        schedule,
        payoff: bal,
        prepayment_pct,
        prepayment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debt(rate: f64, io: u32, amort: Option<u32>) -> Debt {
        Debt {
            amount: Some(1_000_000.0),
            sizing: None,
            rate: Rate {
                fixed: Some(rate),
                floating: None,
            },
            io_months: io,
            amortization_years: amort,
            origination_fee_pct: 0.01,
            prepayment_penalty_pct: None,
        }
    }

    #[test]
    fn thirty_year_six_percent_matches_the_textbook_payment() {
        // $1,000,000 at 6% over 30 years: $5,995.51 a month.
        let r = run(&debt(0.06, 0, Some(30)), 0, 0, 360);
        assert_eq!(r.schedule[0].payment, 599_551);
        assert_eq!(r.schedule[0].interest, 500_000);
        assert_eq!(r.schedule[359].closing, 0);
        assert_eq!(r.fee, 1_000_000);
    }

    #[test]
    fn interest_only_then_amortizing() {
        let r = run(&debt(0.06, 24, Some(30)), 0, 0, 36);
        assert!(r.schedule[..24]
            .iter()
            .all(|p| p.principal == 0 && p.interest == 500_000));
        assert_eq!(r.schedule[24].payment, 599_551);
        assert_eq!(r.payoff, r.schedule[35].closing);
    }
}
