//! Recoveries (step 2): each tenant's share of recoverable expenses.

use crate::calendar::Calendar;
use crate::deal::Recovery;
use crate::money::grow;
use crate::opex::Pool;

/// The recoverable pool split by month over the model, cached.
#[derive(Clone, Debug)]
pub struct PoolCache<'a> {
    pub pool: Pool<'a>,
    /// (controllable, non-controllable) for model months 1..=months.
    months: Vec<(f64, f64)>,
}

impl<'a> PoolCache<'a> {
    pub fn new(pool: Pool<'a>, months: usize) -> PoolCache<'a> {
        let m = (1..=months as i64).map(|m| pool.split(m, None)).collect();
        PoolCache { pool, months: m }
    }

    fn split(&self, m: i64, only: Option<&[String]>) -> (f64, f64) {
        match only {
            None if m >= 1 && (m as usize) <= self.months.len() => self.months[m as usize - 1],
            _ => self.pool.split(m, only),
        }
    }
}

/// A recovery structure prepared for one lease run.
#[derive(Clone, Debug)]
pub struct Recover<'a> {
    rec: &'a Recovery,
    share: f64,
    sf: f64,
    /// Model month of the lease start, for the cap.
    m0: i64,
    /// Controllable pool in the lease's start month.
    c0: f64,
    /// Base-year pool, cents per month (base-year structures).
    base: f64,
    cap: Option<f64>,
}

fn cap_of(r: &Recovery) -> Option<f64> {
    match r {
        Recovery::Nnn {
            cap_controllable_pct,
        }
        | Recovery::BaseYear {
            cap_controllable_pct,
            ..
        }
        | Recovery::ExpenseStop {
            cap_controllable_pct,
            ..
        }
        | Recovery::ModifiedGross {
            cap_controllable_pct,
            ..
        } => *cap_controllable_pct,
        Recovery::Gross => None,
    }
}

fn only(r: &Recovery) -> Option<&[String]> {
    match r {
        Recovery::ModifiedGross { lines, .. } => Some(lines),
        _ => None,
    }
}

impl<'a> Recover<'a> {
    /// `base_year` overrides the structure's year (speculative leases use the
    /// calendar year they start in).
    pub fn new(
        rec: &'a Recovery,
        share: f64,
        sf: f64,
        m0: i64,
        base_year: Option<i32>,
        cal: &Calendar,
        pools: &PoolCache,
    ) -> Recover<'a> {
        let (c0, _) = pools.split(m0, only(rec));
        let base = match rec {
            Recovery::BaseYear { year, .. } => {
                let y = base_year.unwrap_or(*year);
                let jan = cal.month_of(crate::calendar::Date { y, m: 1, d: 1 });
                (jan..jan + 12)
                    .map(|m| pools.pool.total(m, None))
                    .sum::<f64>()
                    / 12.0
            }
            _ => 0.0,
        };
        Recover {
            rec,
            share,
            sf,
            m0,
            c0,
            base,
            cap: cap_of(rec),
        }
    }

    /// The recovery for a full month `m`, in unrounded cents.
    pub fn month(&self, m: i64, pools: &PoolCache) -> f64 {
        if matches!(self.rec, Recovery::Gross) {
            return 0.0;
        }
        let (c, n) = pools.split(m, only(self.rec));
        let c = match self.cap {
            Some(cap) => c.min(self.c0 * grow(cap, (m - self.m0).div_euclid(12).max(0))),
            None => c,
        };
        let p = c + n;
        match self.rec {
            Recovery::Nnn { .. } | Recovery::ModifiedGross { .. } => self.share * p,
            Recovery::BaseYear { .. } => self.share * (p - self.base).max(0.0),
            Recovery::ExpenseStop { stop_psf_yr, .. } => {
                (self.share * p - stop_psf_yr * self.sf * 100.0 / 12.0).max(0.0)
            }
            Recovery::Gross => 0.0,
        }
    }
}
