//! Checks across fields, and the normalized model the engine runs on.

use super::rules::{check_profile, check_recovery, finite_nonneg, merge, unit, Errors};
use super::*;
use crate::calendar::Calendar;
use crate::money::{cents, Cents};
use std::collections::BTreeSet;

/// A lease with its references resolved.
#[derive(Clone, Debug)]
pub struct LeaseModel {
    pub lease: Lease,
    pub recovery: Recovery,
    pub share: f64,
    /// The market profile with per-lease overrides applied.
    pub profile: Option<MarketProfile>,
}

/// A validated deal, ready to run.
#[derive(Clone, Debug)]
pub struct Model {
    pub deal: Deal,
    pub cal: Calendar,
    /// Sale month X; the model runs X + 12 months.
    pub exit_month: usize,
    pub rsf: f64,
    pub units: u32,
    pub leases: Vec<LeaseModel>,
    pub price: Cents,
    pub closing: Cents,
}

/// Validates a deal and resolves its references. All errors are reported together.
pub fn validate(deal: &Deal) -> Result<Model, String> {
    let mut e = Errors(Vec::new());
    let p = &deal.property;
    let mf = p.kind == PropertyType::Multifamily;
    e.check(p.start_date.d == 1, || {
        "property.start_date must be the first day of a month".into()
    });
    e.check(p.hold_months >= 1 && p.hold_months <= 600, || {
        "property.hold_months must be 1 to 600".into()
    });
    let exit_month = deal.exit.month.unwrap_or(p.hold_months) as usize;
    e.check(
        exit_month >= 1 && exit_month <= p.hold_months as usize,
        || "exit.month must be 1 to property.hold_months".into(),
    );
    e.check(
        deal.exit.cap_rate.is_finite() && deal.exit.cap_rate > 0.0,
        || "exit.cap_rate must be > 0".into(),
    );
    e.check(unit(deal.exit.selling_costs_pct), || {
        "exit.selling_costs_pct must be in [0, 1]".into()
    });

    let rsf = p.rentable_sf.unwrap_or(0.0);
    let mut units = 0u32;
    if mf {
        e.check(!deal.unit_mix.is_empty(), || {
            "multifamily needs a unit_mix".into()
        });
        e.check(
            deal.rent_roll.is_empty() && deal.market.is_empty() && deal.recoveries.is_empty(),
            || "multifamily uses unit_mix, not rent_roll, market or recoveries".into(),
        );
        units = deal.unit_mix.iter().map(|u| u.units).sum();
        e.check(p.units.is_none_or(|n| n == units), || {
            format!(
                "property.units ({}) must equal the unit_mix total ({units})",
                p.units.unwrap_or(0)
            )
        });
        e.check(deal.capital.reserves_per_sf_yr.is_none(), || {
            "multifamily reserves are capital.reserves_per_unit_yr".into()
        });
        let mut ids = BTreeSet::new();
        for (i, u) in deal.unit_mix.iter().enumerate() {
            let at = format!("unit_mix[{i}] ({})", u.id);
            e.check(ids.insert(u.id.as_str()), || format!("{at}: duplicate id"));
            e.check(u.units >= 1, || format!("{at}: units must be at least 1"));
            e.check(
                finite_nonneg(u.in_place_rent_mo) && finite_nonneg(u.market_rent_mo),
                || format!("{at}: rents must be >= 0"),
            );
            e.check(u.rent_growth.is_finite() && u.rent_growth > -1.0, || {
                format!("{at}: rent_growth must be > -1")
            });
            e.check(unit(u.turnover_rate), || {
                format!("{at}: turnover_rate must be in [0, 1]")
            });
            e.check(
                finite_nonneg(u.concession_months) && u.concession_months <= 12.0,
                || format!("{at}: concession_months must be 0 to 12"),
            );
        }
    } else {
        e.check(rsf.is_finite() && rsf > 0.0, || {
            "property.rentable_sf must be > 0".into()
        });
        e.check(p.units.is_none() && deal.unit_mix.is_empty(), || {
            "units and unit_mix are for multifamily only".into()
        });
        e.check(deal.capital.reserves_per_unit_yr.is_none(), || {
            "commercial reserves are capital.reserves_per_sf_yr".into()
        });
    }

    let mut lines: BTreeMap<&str, &ExpenseLine> = BTreeMap::new();
    for (i, l) in deal.opex.lines.iter().enumerate() {
        e.check(lines.insert(l.id.as_str(), l).is_none(), || {
            format!("opex.lines[{i}]: duplicate id \"{}\"", l.id)
        });
        e.check(l.amount_yr.is_finite(), || {
            format!("opex.lines[{i}]: amount_yr must be a number")
        });
        e.check(l.growth.is_finite() && l.growth > -1.0, || {
            format!("opex.lines[{i}]: growth must be > -1")
        });
    }
    e.check(unit(deal.opex.management_fee_pct), || {
        "opex.management_fee_pct must be in [0, 1]".into()
    });
    let mut inc = BTreeSet::new();
    for (i, l) in deal.other_income.iter().enumerate() {
        e.check(inc.insert(l.id.as_str()), || {
            format!("other_income[{i}]: duplicate id \"{}\"", l.id)
        });
        e.check(
            l.amount_yr.is_finite() && l.growth.is_finite() && l.growth > -1.0,
            || format!("other_income[{i}]: amount_yr and growth must be numbers, growth > -1"),
        );
    }
    let r = &deal.revenue;
    e.check(
        unit(r.general_vacancy_pct) && unit(r.credit_loss_pct),
        || "revenue: general_vacancy_pct and credit_loss_pct must be in [0, 1]".into(),
    );
    for (name, rec) in &deal.recoveries {
        check_recovery(&mut e, rec, &lines, &format!("recoveries.{name}"));
    }
    let mut profiles = BTreeMap::new();
    for (i, m) in deal.market.iter().enumerate() {
        e.check(profiles.insert(m.id.as_str(), m).is_none(), || {
            format!("market[{i}]: duplicate id \"{}\"", m.id)
        });
        check_profile(&mut e, m, &format!("market[{i}] ({})", m.id));
    }

    let cal_months = exit_month + 12;
    let cal = Calendar {
        start: p.start_date,
        months: cal_months,
    };
    let mut leases = Vec::new();
    let mut lease_ids = BTreeSet::new();
    for (i, l) in deal.rent_roll.iter().enumerate() {
        let at = format!("rent_roll[{i}] ({})", l.id);
        e.check(lease_ids.insert(l.id.as_str()), || {
            format!("{at}: duplicate id")
        });
        e.check(l.sf.is_finite() && l.sf > 0.0, || {
            format!("{at}: sf must be > 0")
        });
        e.check(l.expiry >= l.start, || {
            format!("{at}: expiry is before start")
        });
        e.check(l.expiry >= p.start_date, || {
            format!("{at}: lease expired before the analysis start")
        });
        e.check(finite_nonneg(l.rent_psf_yr), || {
            format!("{at}: rent_psf_yr must be >= 0")
        });
        e.check(finite_nonneg(l.security_deposit), || {
            format!("{at}: security_deposit must be >= 0")
        });
        match &l.steps {
            Some(Steps::Percent { rate }) => e.check(rate.is_finite() && *rate > -1.0, || {
                format!("{at}: step rate must be > -1")
            }),
            Some(Steps::Fixed { amount_psf_yr }) => e.check(amount_psf_yr.is_finite(), || {
                format!("{at}: step amount must be a number")
            }),
            Some(Steps::Cpi { rates }) => e.check(
                !rates.is_empty() && rates.iter().all(|r| r.is_finite() && *r > -1.0),
                || format!("{at}: cpi rates must be a non-empty list, each > -1"),
            ),
            None => {}
        }
        let recovery = match &l.recovery {
            Some(RecoveryRef::Inline(rec)) => {
                check_recovery(&mut e, rec, &lines, &format!("{at}.recovery"));
                Some(rec.clone())
            }
            Some(RecoveryRef::Name(n)) => deal.recoveries.get(n).cloned(),
            None => deal.recoveries.get("default").cloned(),
        };
        e.check(recovery.is_some(), || match &l.recovery {
            Some(RecoveryRef::Name(n)) => format!("{at}: unknown recovery \"{n}\""),
            _ => format!("{at}: no recovery given and no recoveries.default"),
        });
        let share = l
            .pro_rata_share
            .unwrap_or(if rsf > 0.0 { l.sf / rsf } else { 0.0 });
        e.check(unit(share), || {
            format!("{at}: pro_rata_share must be in [0, 1]")
        });
        let profile = match &l.space_type {
            Some(s) => match profiles.get(s.as_str()) {
                Some(m) => {
                    let merged = merge(m, &l.market);
                    check_profile(&mut e, &merged, &format!("{at} market"));
                    Some(merged)
                }
                None => {
                    e.0.push(format!("{at}: unknown space_type \"{s}\""));
                    None
                }
            },
            None => None,
        };
        e.check(l.market.is_none() || l.space_type.is_some(), || {
            format!("{at}: market overrides need a space_type")
        });
        let expires_inside = l.expiry.next_day().day() < cal.end_day();
        e.check(!expires_inside || l.space_type.is_some(), || {
            format!("{at}: expires inside the model, so it needs a space_type for rollover")
        });
        if let Some(rec) = recovery {
            leases.push(LeaseModel {
                lease: l.clone(),
                recovery: rec,
                share,
                profile,
            });
        }
    }
    if !mf {
        let leased: f64 = deal.rent_roll.iter().map(|l| l.sf).sum();
        e.check(leased <= rsf * (1.0 + 1e-9), || {
            format!("rent roll SF ({leased}) exceeds rentable_sf ({rsf})")
        });
    }

    let c = &deal.capital;
    e.check(c.price.is_finite() && c.price > 0.0, || {
        "capital.price must be > 0".into()
    });
    e.check(finite_nonneg(c.closing_costs), || {
        "capital.closing_costs must be >= 0".into()
    });
    e.check(
        c.reserves_per_sf_yr.is_none_or(finite_nonneg)
            && c.reserves_per_unit_yr.is_none_or(finite_nonneg),
        || "capital reserves must be >= 0".into(),
    );
    let mut capex_ids = BTreeSet::new();
    for (i, x) in c.capex.iter().enumerate() {
        e.check(capex_ids.insert(x.id.as_str()), || {
            format!("capital.capex[{i}]: duplicate id \"{}\"", x.id)
        });
        e.check(x.month >= 1 && x.month as usize <= exit_month, || {
            format!("capital.capex[{i}]: month must be 1 to the exit month")
        });
        e.check(x.amount.is_finite(), || {
            format!("capital.capex[{i}]: amount must be a number")
        });
    }
    if let Some(d) = &deal.debt {
        crate::debt::check(d, &mut e.0);
    }
    if let Some(q) = &deal.equity {
        crate::waterfall::check(q, &mut e.0);
    }
    if !e.0.is_empty() {
        return Err(e.0.join("; "));
    }
    Ok(Model {
        deal: deal.clone(),
        cal,
        exit_month,
        rsf,
        units,
        leases,
        price: cents(c.price),
        closing: cents(c.closing_costs),
    })
}
