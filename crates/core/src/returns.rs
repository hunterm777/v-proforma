//! Returns (step 9): IRR, XIRR, multiple, cash-on-cash, peak equity, payback.

use crate::money::{pow, Cents};
use serde::Serialize;

/// NPV of monthly flows at monthly rate `r`, by repeated multiplication.
pub fn npv(cf: &[f64], r: f64) -> f64 {
    let v = 1.0 / (1.0 + r);
    let mut d = 1.0;
    let mut s = 0.0;
    for c in cf {
        s += c * d;
        d *= v;
    }
    s
}

fn root(f: impl Fn(f64) -> f64, grid: &[f64]) -> Option<f64> {
    let vals: Vec<f64> = grid.iter().map(|r| f(*r)).collect();
    let mut best: Option<(f64, f64)> = None;
    for i in 0..grid.len() - 1 {
        if vals[i] == 0.0 {
            return Some(grid[i]);
        }
        if vals[i].is_finite() && vals[i + 1].is_finite() && (vals[i] < 0.0) != (vals[i + 1] < 0.0)
        {
            let (lo, hi) = (grid[i], grid[i + 1]);
            let near = if lo <= 0.0 && hi >= 0.0 {
                0.0
            } else {
                lo.abs().min(hi.abs())
            };
            if best.is_none_or(|(b, _)| near < b) {
                best = Some((near, lo));
            }
        }
    }
    let (_, lo0) = best?;
    let i = grid.iter().position(|r| *r == lo0)?;
    let (mut lo, mut hi) = (grid[i], grid[i + 1]);
    let flo = f(lo) < 0.0;
    for _ in 0..400 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if (f(mid) < 0.0) == flo {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

const MONTHLY_GRID: [f64; 24] = [
    -0.99, -0.9, -0.5, -0.2, -0.1, -0.05, -0.02, -0.01, -0.005, -0.002, 0.0, 0.002, 0.004, 0.006,
    0.008, 0.01, 0.015, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 5.0,
];

/// Monthly IRR of monthly flows; when several roots exist, the one nearest 0.
pub fn irr_monthly(cf: &[f64]) -> Option<f64> {
    let pos = cf.iter().any(|c| *c > 0.0);
    let neg = cf.iter().any(|c| *c < 0.0);
    if !(pos && neg) {
        return None;
    }
    root(|r| npv(cf, r), &MONTHLY_GRID)
}

/// Annualised: (1 + r)^12 - 1.
pub fn annualise(r: f64) -> f64 {
    pow(1.0 + r, 12.0) - 1.0
}

/// XIRR on day offsets with an actual/365 year.
pub fn xirr(cf: &[f64], days: &[i64]) -> Option<f64> {
    if !(cf.iter().any(|c| *c > 0.0) && cf.iter().any(|c| *c < 0.0)) {
        return None;
    }
    let f = |r: f64| -> f64 {
        cf.iter()
            .zip(days)
            .map(|(c, d)| c / pow(1.0 + r, (d - days[0]) as f64 / 365.0))
            .sum()
    };
    let grid = [
        -0.99, -0.9, -0.5, -0.2, -0.1, -0.05, 0.0, 0.02, 0.05, 0.08, 0.1, 0.15, 0.2, 0.3, 0.5, 1.0,
        2.0, 5.0, 20.0, 100.0,
    ];
    root(f, &grid)
}

#[derive(Clone, Debug, Serialize)]
pub struct Series {
    /// Cents at month 0 (close) to the exit month.
    pub cash_flows: Vec<Cents>,
    pub irr: Option<f64>,
    pub irr_monthly: Option<f64>,
    pub xirr: Option<f64>,
    /// |NPV at the monthly IRR| / sum of |cash flows|.
    pub irr_npv_residual: Option<f64>,
    pub equity_multiple: Option<f64>,
    pub profit: Cents,
    /// Operating cash flow by analysis year over the year-0 investment.
    pub cash_on_cash: Vec<f64>,
    pub peak_equity: Cents,
    pub payback_month: Option<usize>,
}

/// Every return measure of a series. `operating` is the cash flow by month
/// without the sale (for cash-on-cash); `days` dates each flow.
pub fn series(cf: Vec<Cents>, operating: &[Cents], days: &[i64]) -> Series {
    let f: Vec<f64> = cf.iter().map(|c| *c as f64).collect();
    let m = irr_monthly(&f);
    let resid = m.map(|r| npv(&f, r).abs() / f.iter().map(|c| c.abs()).sum::<f64>());
    let inflow: Cents = cf.iter().filter(|c| **c > 0).sum();
    let outflow: Cents = -cf.iter().filter(|c| **c < 0).sum::<Cents>();
    let invest = -cf[0];
    let years = operating.len().div_ceil(12);
    let coc = (0..years)
        .map(|y| {
            let s: Cents = operating[y * 12..((y + 1) * 12).min(operating.len())]
                .iter()
                .sum();
            if invest > 0 {
                s as f64 / invest as f64
            } else {
                0.0
            }
        })
        .collect();
    let mut cum = 0;
    let mut low = 0;
    let mut payback = None;
    for (t, c) in cf.iter().enumerate() {
        cum += c;
        low = low.min(cum);
        if payback.is_none() && t > 0 && cum >= 0 && low < 0 {
            payback = Some(t);
        }
    }
    Series {
        irr: m.map(annualise),
        irr_monthly: m,
        xirr: xirr(&f, days),
        irr_npv_residual: resid,
        equity_multiple: (outflow > 0).then(|| inflow as f64 / outflow as f64),
        profit: cf.iter().sum(),
        cash_on_cash: coc,
        peak_equity: -low,
        payback_month: payback,
        cash_flows: cf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irr_of_a_known_series() {
        // -100 then 110 after one month: 10% a month.
        let r = irr_monthly(&[-100.0, 110.0]).unwrap();
        assert!((r - 0.1).abs() < 1e-15);
        // A level annuity: -1000, then 12 x 88.8488 ~ 1% per month.
        let pay = 1000.0 * 0.01 / (1.0 - libm::pow(1.01, -12.0));
        let mut cf = vec![-1000.0];
        cf.extend([pay; 12]);
        assert!((irr_monthly(&cf).unwrap() - 0.01).abs() < 1e-13);
        assert!(irr_monthly(&[1.0, 2.0]).is_none());
    }

    #[test]
    fn xirr_of_one_year() {
        let r = xirr(&[-100.0, 112.0], &[0, 365]).unwrap();
        assert!((r - 0.12).abs() < 1e-12);
    }

    #[test]
    fn payback_and_peak() {
        let s = series(
            vec![-100, 30, -20, 60, 50],
            &[30, -20, 60, 50],
            &[0, 31, 59, 90, 120],
        );
        assert_eq!(s.peak_equity, 100);
        assert_eq!(s.payback_month, Some(4));
        assert_eq!(s.profit, 20);
        assert_eq!(s.equity_multiple, Some(140.0 / 120.0));
    }
}
