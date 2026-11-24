//! Field rules shared by `validate`: market profiles and recovery structures.

use super::*;

pub(super) struct Errors(pub(super) Vec<String>);

impl Errors {
    pub(super) fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        if !ok {
            self.0.push(msg());
        }
    }
}

pub(super) fn unit(x: f64) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}

pub(super) fn finite_nonneg(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

pub(super) fn merge(p: &MarketProfile, o: &Option<MarketOverride>) -> MarketProfile {
    let mut p = p.clone();
    if let Some(o) = o {
        p.market_rent_psf_yr = o.market_rent_psf_yr.unwrap_or(p.market_rent_psf_yr);
        p.rent_growth = o.rent_growth.unwrap_or(p.rent_growth);
        p.renewal_probability = o.renewal_probability.unwrap_or(p.renewal_probability);
        p.downtime_months = o.downtime_months.unwrap_or(p.downtime_months);
        p.term_months = o.term_months.unwrap_or(p.term_months);
        p.new = o.new.unwrap_or(p.new);
        p.renewal = o.renewal.unwrap_or(p.renewal);
    }
    p
}

pub(super) fn check_profile(e: &mut Errors, p: &MarketProfile, at: &str) {
    e.check(finite_nonneg(p.market_rent_psf_yr), || {
        format!("{at}: market_rent_psf_yr must be >= 0")
    });
    e.check(p.rent_growth.is_finite() && p.rent_growth > -1.0, || {
        format!("{at}: rent_growth must be > -1")
    });
    e.check(unit(p.renewal_probability), || {
        format!("{at}: renewal_probability must be in [0, 1]")
    });
    e.check(p.term_months >= 1, || {
        format!("{at}: term_months must be at least 1")
    });
    for (k, t) in [("new", p.new), ("renewal", p.renewal)] {
        e.check(finite_nonneg(t.ti_psf), || {
            format!("{at}.{k}: ti_psf must be >= 0")
        });
        e.check(finite_nonneg(t.lc_pct), || {
            format!("{at}.{k}: lc_pct must be >= 0")
        });
        e.check(t.free_rent_months <= p.term_months, || {
            format!("{at}.{k}: free_rent_months exceeds term_months")
        });
    }
}

pub(super) fn check_recovery(
    e: &mut Errors,
    r: &Recovery,
    lines: &BTreeMap<&str, &ExpenseLine>,
    at: &str,
) {
    let cap = match r {
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
    };
    if let Some(c) = cap {
        e.check(finite_nonneg(c), || {
            format!("{at}: cap_controllable_pct must be >= 0")
        });
    }
    match r {
        Recovery::ExpenseStop { stop_psf_yr, .. } => e.check(finite_nonneg(*stop_psf_yr), || {
            format!("{at}: stop_psf_yr must be >= 0")
        }),
        Recovery::ModifiedGross { lines: ids, .. } => {
            e.check(!ids.is_empty(), || {
                format!("{at}: modified_gross needs at least one line")
            });
            for id in ids {
                match lines.get(id.as_str()) {
                    None => e.0.push(format!("{at}: unknown expense line \"{id}\"")),
                    Some(l) => e.check(l.recoverable, || {
                        format!("{at}: expense line \"{id}\" is not recoverable")
                    }),
                }
            }
        }
        Recovery::BaseYear { year, .. } => e.check((1900..=2200).contains(year), || {
            format!("{at}: base year {year} is out of range")
        }),
        _ => {}
    }
}
