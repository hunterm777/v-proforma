//! Lease cash flows (step 1): contractual rent with steps and free rent.

use crate::calendar::{Calendar, Date};
use crate::deal::{Recovery, Steps};
use crate::money::grow;
use crate::recoveries::{PoolCache, Recover};

/// One lease, contractual or speculative, with its probability weight.
#[derive(Clone, Debug)]
pub struct Run<'a> {
    pub sf: f64,
    pub start: Date,
    pub expiry: Date,
    pub rent_psf_yr: f64,
    pub steps: Option<&'a Steps>,
    pub free_rent_months: u32,
    pub recovery: &'a Recovery,
    pub base_year: Option<i32>,
    pub share: f64,
    pub weight: f64,
    /// TI and LC in cents, paid in the start month (speculative leases).
    pub ti: f64,
    pub lc: f64,
}

/// Unrounded cents by model month (index 0 = month 1).
#[derive(Clone, Debug)]
pub struct Flows {
    pub base: Vec<f64>,
    pub free: Vec<f64>,
    pub rec: Vec<f64>,
    pub ti: Vec<f64>,
    pub lc: Vec<f64>,
    /// Weighted share of each month the space is under this lease.
    pub occupied: Vec<f64>,
}

impl Flows {
    pub fn zero(n: usize) -> Flows {
        Flows {
            base: vec![0.0; n],
            free: vec![0.0; n],
            rec: vec![0.0; n],
            ti: vec![0.0; n],
            lc: vec![0.0; n],
            occupied: vec![0.0; n],
        }
    }
}

/// Rent in $/SF/yr in lease year `k + 1` (after `k` anniversaries).
pub fn rent_in_year(rent: f64, steps: Option<&Steps>, k: i64) -> f64 {
    match steps {
        None => rent,
        Some(Steps::Percent { rate }) => rent * grow(*rate, k),
        Some(Steps::Fixed { amount_psf_yr }) => rent + k as f64 * amount_psf_yr,
        Some(Steps::Cpi { rates }) => {
            let mut r = rent;
            for i in 0..k as usize {
                r *= 1.0 + rates[i.min(rates.len() - 1)];
            }
            r
        }
    }
}

/// Base rent and free rent of `run` over the days `[lo, hi)` of a month with
/// `dim` days, in unrounded cents (before weighting).
fn rent_days(run: &Run, lo: i64, hi: i64, dim: f64) -> (f64, f64) {
    let free_end = run.start.add_months(i64::from(run.free_rent_months)).day();
    let mut k = 0i64;
    while run.start.add_months(12 * (k + 1)).day() <= lo {
        k += 1;
    }
    let (mut base, mut free) = (0.0, 0.0);
    let mut day = lo;
    while day < hi {
        let next_ann = run.start.add_months(12 * (k + 1)).day();
        let mut end = hi.min(next_ann);
        if day < free_end {
            end = end.min(free_end);
        }
        let rate = rent_in_year(run.rent_psf_yr, run.steps, k);
        let amt = rate * run.sf * 100.0 / 12.0 * (end - day) as f64 / dim;
        base += amt;
        if day < free_end {
            free += amt;
        }
        day = end;
        if day >= next_ann {
            k += 1;
        }
    }
    (base, free)
}

/// Adds the cash flows of `run` to `out`.
pub fn flows(run: &Run, cal: &Calendar, pools: &PoolCache, out: &mut Flows) {
    let (s, e) = (run.start.day(), run.expiry.day() + 1);
    let m_first = cal.month_of(run.start).max(1);
    let m_last = cal.month_of(run.expiry).min(cal.months as i64);
    let rec = Recover::new(
        run.recovery,
        run.share,
        run.sf,
        cal.month_of(run.start),
        run.base_year,
        cal,
        pools,
    );
    let w = run.weight;
    for m in m_first..=m_last {
        let (a, b) = cal.month_days(m);
        let (lo, hi) = (a.max(s), b.min(e));
        if lo >= hi {
            continue;
        }
        let dim = (b - a) as f64;
        let (base, free) = rent_days(run, lo, hi, dim);
        let i = m as usize - 1;
        let frac = (hi - lo) as f64 / dim;
        out.base[i] += w * base;
        out.free[i] += w * free;
        out.rec[i] += w * frac * rec.month(m, pools);
        out.occupied[i] += w * frac;
    }
    let ms = cal.month_of(run.start);
    if ms >= 1 && ms <= cal.months as i64 {
        out.ti[ms as usize - 1] += w * run.ti;
        out.lc[ms as usize - 1] += w * run.lc;
    }
}
