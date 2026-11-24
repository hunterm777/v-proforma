//! Rollover (step 1): the probability-weighted renewal / new-tenant tree,
//! propagated forward in date order so paths that meet are merged exactly.

use crate::calendar::{Calendar, Date};
use crate::deal::{LeaseModel, Recovery};
use crate::lease::{flows, Flows, Run};
use crate::money::grow;
use crate::recoveries::PoolCache;
use crate::trace::{Ledger, Line, Source, SourceKind};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Renewal,
    New,
}

/// A speculative lease: scenario, start date and path probability.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub kind: Kind,
    pub start: Date,
    pub weight: f64,
}

/// The speculative leases that follow `lm`, and turnover vacancy (market rent
/// lost to downtime, weighted) by model month in cents, each downtime path's
/// monthly cell rounded once.
pub fn tree(lm: &LeaseModel, cal: &Calendar) -> (Vec<Spec>, Vec<f64>) {
    let mut vacancy = vec![0.0; cal.months];
    let Some(p) = &lm.profile else {
        return (Vec::new(), vacancy);
    };
    let end = cal.end_day();
    let term = i64::from(p.term_months);
    let mut free: BTreeMap<Date, f64> = BTreeMap::new();
    free.insert(lm.lease.expiry.next_day(), 1.0);
    let mut specs: BTreeMap<(Date, Kind), f64> = BTreeMap::new();
    while let Some((d, mass)) = free.pop_first() {
        if d.day() >= end || mass == 0.0 {
            continue;
        }
        let pr = p.renewal_probability;
        if pr > 0.0 {
            *specs.entry((d, Kind::Renewal)).or_default() += mass * pr;
            *free.entry(d.add_months(term)).or_default() += mass * pr;
        }
        if pr < 1.0 {
            let w = mass * (1.0 - pr);
            let start = d.add_months(i64::from(p.downtime_months));
            downtime(
                &mut vacancy,
                cal,
                lm.lease.sf,
                p.market_rent_psf_yr,
                p.rent_growth,
                d,
                start,
                w,
            );
            if start.day() < end {
                *specs.entry((start, Kind::New)).or_default() += w;
                *free.entry(start.add_months(term)).or_default() += w;
            }
        }
    }
    let specs = specs
        .into_iter()
        .map(|((start, kind), weight)| Spec {
            kind,
            start,
            weight,
        })
        .collect();
    (specs, vacancy)
}

/// Market rent in $/SF/yr for a lease starting on `d`.
pub fn market_rent(rent: f64, growth: f64, cal: &Calendar, d: Date) -> f64 {
    rent * grow(growth, Calendar::year_of(cal.month_of(d)))
}

#[allow(clippy::too_many_arguments)]
fn downtime(
    v: &mut [f64],
    cal: &Calendar,
    sf: f64,
    rent: f64,
    g: f64,
    from: Date,
    to: Date,
    w: f64,
) {
    let (s, e) = (from.day(), to.day().min(cal.end_day()));
    let mut m = cal.month_of(from).max(1);
    while m <= cal.months as i64 {
        let (a, b) = cal.month_days(m);
        if a >= e {
            break;
        }
        let (lo, hi) = (a.max(s), b.min(e));
        if lo < hi {
            let mr = rent * grow(g, Calendar::year_of(m));
            // One downtime path's cell, rounded once like every source cell.
            let cell = w * mr * sf * 100.0 / 12.0 * (hi - lo) as f64 / (b - a) as f64;
            v[m as usize - 1] += crate::money::round(cell) as f64;
        }
        m += 1;
    }
}

fn weight_text(w: f64) -> String {
    let s = format!("{w:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn push(ledger: &mut Ledger, id: &str, detail: &str, f: &Flows, spec: bool) {
    let src = || Source::new(SourceKind::Lease, id);
    ledger.push(Line::BaseRent, src(), detail, &f.base);
    ledger.push(Line::FreeRent, src(), detail, &f.free);
    ledger.push(Line::Recoveries, src(), detail, &f.rec);
    if spec {
        ledger.push(Line::Ti, src(), detail, &f.ti);
        ledger.push(Line::Lc, src(), detail, &f.lc);
    }
}

/// Adds every entry of one rent-roll lease and its rollovers. Returns turnover
/// vacancy and the weighted occupied share of the space by month.
pub fn lease_entries(
    ledger: &mut Ledger,
    lm: &LeaseModel,
    cal: &Calendar,
    pools: &PoolCache,
) -> (Vec<f64>, Vec<f64>) {
    let l = &lm.lease;
    let n = cal.months;
    let contract = Run {
        sf: l.sf,
        start: l.start,
        expiry: l.expiry,
        rent_psf_yr: l.rent_psf_yr,
        steps: l.steps.as_ref(),
        free_rent_months: l.free_rent_months,
        recovery: &lm.recovery,
        base_year: None,
        share: lm.share,
        weight: 1.0,
        ti: 0.0,
        lc: 0.0,
    };
    let mut f = Flows::zero(n);
    flows(&contract, cal, pools, &mut f);
    push(ledger, &l.id, "contract", &f, false);
    let mut occupied = f.occupied;
    let (specs, vacancy) = tree(lm, cal);
    let Some(p) = &lm.profile else {
        return (vacancy, occupied);
    };
    for s in specs {
        let t = if s.kind == Kind::Renewal {
            p.renewal
        } else {
            p.new
        };
        let rent = market_rent(p.market_rent_psf_yr, p.rent_growth, cal, s.start);
        let monthly = rent * l.sf * 100.0 / 12.0;
        let paid = f64::from(p.term_months - t.free_rent_months);
        let run = Run {
            sf: l.sf,
            start: s.start,
            expiry: Date::from_day(s.start.add_months(i64::from(p.term_months)).day() - 1),
            rent_psf_yr: rent,
            steps: None,
            free_rent_months: t.free_rent_months,
            recovery: &lm.recovery,
            base_year: matches!(lm.recovery, Recovery::BaseYear { .. }).then_some(s.start.y),
            share: lm.share,
            weight: s.weight,
            ti: t.ti_psf * l.sf * 100.0,
            lc: t.lc_pct * monthly * paid,
        };
        let mut sf = Flows::zero(n);
        flows(&run, cal, pools, &mut sf);
        let kind = if s.kind == Kind::Renewal {
            "renewal"
        } else {
            "new tenant"
        };
        let detail = format!(
            "rollover {kind} from {}, probability {}, ${:.4}/SF/yr",
            s.start,
            weight_text(s.weight),
            rent
        );
        push(ledger, &l.id, &detail, &sf, true);
        for (a, b) in occupied.iter_mut().zip(&sf.occupied) {
            *a += b;
        }
    }
    (vacancy, occupied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deal::{Lease, LeasingTerms, MarketProfile};

    fn lm(p: f64, down: u32, term: u32) -> LeaseModel {
        LeaseModel {
            lease: Lease {
                id: "a".into(),
                tenant: "A".into(),
                suite: String::new(),
                sf: 1000.0,
                start: Date::parse("2020-01-01").unwrap(),
                expiry: Date::parse("2026-12-31").unwrap(),
                rent_psf_yr: 10.0,
                steps: None,
                free_rent_months: 0,
                recovery: None,
                pro_rata_share: None,
                space_type: Some("x".into()),
                market: None,
                security_deposit: 0.0,
            },
            recovery: Recovery::Gross,
            share: 0.1,
            profile: Some(MarketProfile {
                id: "x".into(),
                market_rent_psf_yr: 12.0,
                rent_growth: 0.0,
                renewal_probability: p,
                downtime_months: down,
                term_months: term,
                new: LeasingTerms::default(),
                renewal: LeasingTerms::default(),
            }),
        }
    }

    #[test]
    fn paths_merge_and_weights_conserve() {
        let cal = Calendar {
            start: Date::parse("2026-01-01").unwrap(),
            months: 72,
        };
        let (specs, vac) = tree(&lm(0.6, 6, 12), &cal);
        // first generation: renewal 2027-01 (0.6), new 2027-07 (0.4)
        assert_eq!(
            specs[0],
            Spec {
                kind: Kind::Renewal,
                start: Date::parse("2027-01-01").unwrap(),
                weight: 0.6
            }
        );
        assert_eq!(
            specs[1],
            Spec {
                kind: Kind::New,
                start: Date::parse("2027-07-01").unwrap(),
                weight: 0.4
            }
        );
        // total probability mass that reaches each free date sums to 1 per path
        let jan28: f64 = specs
            .iter()
            .filter(|s| s.start == Date::parse("2028-01-01").unwrap())
            .map(|s| s.weight)
            .sum();
        assert!((jan28 - 0.36).abs() < 1e-15); // renew, renew
                                               // downtime: 0.4 x 6 months x $1,000/month in year 2
        let total: f64 = vac.iter().sum();
        assert!(total > 0.0);
        assert!((vac[12] - 0.4 * 12.0 * 1000.0 * 100.0 / 12.0).abs() < 1e-6);
    }

    #[test]
    fn certain_renewal_has_no_downtime() {
        let cal = Calendar {
            start: Date::parse("2026-01-01").unwrap(),
            months: 60,
        };
        let (specs, vac) = tree(&lm(1.0, 6, 24), &cal);
        assert!(specs
            .iter()
            .all(|s| s.kind == Kind::Renewal && s.weight == 1.0));
        assert_eq!(specs.len(), 2);
        assert!(vac.iter().all(|v| *v == 0.0));
    }
}
