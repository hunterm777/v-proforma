//! Rent roll summary and tenant detail.

use crate::calendar::{Calendar, Date};
use crate::deal::Lease;
use crate::engine::Outcome;
use crate::lease::rent_in_year;
use crate::money::Cents;
use crate::trace::{sum, Line, SourceKind};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Expiration {
    /// Analysis year, or none for leases expiring after the hold.
    pub year: Option<usize>,
    pub sf: f64,
    pub pct_sf: f64,
    /// Share of in-place annual base rent at the start date.
    pub pct_rent: f64,
    pub leases: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnitMixRow {
    pub id: String,
    pub units: u32,
    pub in_place_rent_mo: f64,
    pub market_rent_mo: f64,
    pub loss_to_lease_pct: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RentRoll {
    /// Weighted by in-place annual base rent, in years from the start date.
    pub walt_years: Option<f64>,
    /// Average expected occupied SF / rentable SF by analysis year.
    pub occupancy_by_year: Vec<f64>,
    pub expirations: Vec<Expiration>,
    pub unit_mix: Vec<UnitMixRow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TenantRow {
    pub lease_id: String,
    pub tenant: String,
    pub suite: String,
    pub sf: f64,
    pub start: String,
    pub expiry: String,
    pub security_deposit: f64,
    /// Contract rent in $/SF/yr in effect on the later of start and analysis start.
    pub in_place_rent_psf_yr: f64,
    /// Lease and rollover totals by line, by analysis year.
    pub years: BTreeMap<&'static str, Vec<Cents>>,
}

/// $/SF/yr in effect on `on`.
fn rent_on(l: &Lease, on: Date) -> f64 {
    let mut k = 0;
    while l.start.add_months(12 * (k + 1)) <= on {
        k += 1;
    }
    rent_in_year(l.rent_psf_yr, l.steps.as_ref(), k)
}

pub fn summary(o: &Outcome) -> RentRoll {
    let m = &o.model;
    let x = m.exit_month;
    let years = Calendar::years(x);
    let start = m.cal.start;
    let inplace: Vec<(f64, &Lease)> = m
        .leases
        .iter()
        .map(|lm| {
            (
                rent_on(&lm.lease, lm.lease.start.max(start)) * lm.lease.sf,
                &lm.lease,
            )
        })
        .collect();
    let total_rent: f64 = inplace.iter().map(|(r, _)| r).sum();
    let walt = (total_rent > 0.0).then(|| {
        inplace
            .iter()
            .map(|(r, l)| r * (l.expiry.day() + 1 - start.day()) as f64 / 365.0)
            .sum::<f64>()
            / total_rent
    });
    let occupancy_by_year = if m.rsf > 0.0 {
        (0..years)
            .map(|y| {
                let (a, b) = (y * 12, ((y + 1) * 12).min(x));
                o.occupied_sf[a..b].iter().sum::<f64>() / (b - a) as f64 / m.rsf
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut buckets: BTreeMap<Option<usize>, Expiration> = BTreeMap::new();
    for (r, l) in &inplace {
        let em = m.cal.month_of(l.expiry);
        let year = (em <= x as i64).then(|| Calendar::year_of(em) as usize + 1);
        let e = buckets.entry(year).or_insert(Expiration {
            year,
            sf: 0.0,
            pct_sf: 0.0,
            pct_rent: 0.0,
            leases: Vec::new(),
        });
        e.sf += l.sf;
        e.pct_sf += if m.rsf > 0.0 { l.sf / m.rsf } else { 0.0 };
        e.pct_rent += if total_rent > 0.0 {
            r / total_rent
        } else {
            0.0
        };
        e.leases.push(l.id.clone());
    }
    let mut expirations: Vec<Expiration> = buckets.into_values().collect();
    expirations.sort_by_key(|e| e.year.unwrap_or(usize::MAX));
    let unit_mix = m
        .deal
        .unit_mix
        .iter()
        .map(|u| UnitMixRow {
            id: u.id.clone(),
            units: u.units,
            in_place_rent_mo: u.in_place_rent_mo,
            market_rent_mo: u.market_rent_mo,
            loss_to_lease_pct: if u.market_rent_mo > 0.0 {
                1.0 - u.in_place_rent_mo / u.market_rent_mo
            } else {
                0.0
            },
        })
        .collect();
    RentRoll {
        walt_years: walt,
        occupancy_by_year,
        expirations,
        unit_mix,
    }
}

pub fn tenants(o: &Outcome) -> Vec<TenantRow> {
    let m = &o.model;
    let x = m.exit_month;
    let years = Calendar::years(x);
    // Index the lease entries once: (lease id, line) -> annual sums.
    let mut by: BTreeMap<(&str, Line), Vec<Cents>> = BTreeMap::new();
    for e in o
        .ledger
        .entries
        .iter()
        .filter(|e| e.source.kind == SourceKind::Lease)
    {
        let v = by
            .entry((e.source.id.as_str(), e.line))
            .or_insert_with(|| vec![0; years]);
        for (y, a) in v.iter_mut().enumerate() {
            *a += sum(&e.cents, y * 12 + 1, ((y + 1) * 12).min(x));
        }
    }
    m.leases
        .iter()
        .map(|lm| {
            let l = &lm.lease;
            let mut ys: BTreeMap<&'static str, Vec<Cents>> = BTreeMap::new();
            for line in [
                Line::BaseRent,
                Line::FreeRent,
                Line::Recoveries,
                Line::Ti,
                Line::Lc,
            ] {
                let v = by
                    .get(&(l.id.as_str(), line))
                    .cloned()
                    .unwrap_or_else(|| vec![0; years]);
                ys.insert(line.key(), v);
            }
            TenantRow {
                lease_id: l.id.clone(),
                tenant: l.tenant.clone(),
                suite: l.suite.clone(),
                sf: l.sf,
                start: l.start.to_string(),
                expiry: l.expiry.to_string(),
                security_deposit: l.security_deposit,
                in_place_rent_psf_yr: rent_on(l, l.start.max(m.cal.start)),
                years: ys,
            }
        })
        .collect()
}
