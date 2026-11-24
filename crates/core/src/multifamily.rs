//! Multifamily (step 1 for a unit mix): scheduled rent with loss-to-lease
//! burn-off, and concessions on turned units.

use crate::calendar::Calendar;
use crate::deal::UnitType;
use crate::money::grow;
use crate::trace::{Ledger, Line, Source, SourceKind};

/// Market rent per unit per month in month `m`, in cents.
pub fn market(u: &UnitType, m: i64) -> f64 {
    u.market_rent_mo * 100.0 * grow(u.rent_growth, Calendar::year_of(m))
}

/// Loss-to-lease per unit per month in month `m`, in cents.
pub fn loss_to_lease(u: &UnitType, m: i64) -> f64 {
    if u.loss_to_lease_burn_off_months == 0 {
        return 0.0;
    }
    let left = (1.0 - (m - 1) as f64 / f64::from(u.loss_to_lease_burn_off_months)).max(0.0);
    (u.market_rent_mo - u.in_place_rent_mo) * 100.0 * left
}

/// Adds base rent and concession entries for every unit type.
pub fn entries(ledger: &mut Ledger, mix: &[UnitType]) {
    for u in mix {
        let n = f64::from(u.units);
        let ms = 1..=ledger.months as i64;
        let base: Vec<f64> = ms
            .clone()
            .map(|m| n * (market(u, m) - loss_to_lease(u, m)))
            .collect();
        let conc: Vec<f64> = ms
            .map(|m| n * u.turnover_rate / 12.0 * u.concession_months * market(u, m))
            .collect();
        let src = || Source::new(SourceKind::UnitType, &u.id);
        let d = format!(
            "{} units, in place ${}/mo, market ${}/mo, burn-off {} months",
            u.units, u.in_place_rent_mo, u.market_rent_mo, u.loss_to_lease_burn_off_months
        );
        ledger.push(Line::BaseRent, src(), d, &base);
        let d = format!(
            "turnover {}/yr x {} free months at market",
            u.turnover_rate, u.concession_months
        );
        ledger.push(Line::FreeRent, src(), d, &conc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burn_off_is_linear_and_ends() {
        let u = UnitType {
            id: "1br".into(),
            name: String::new(),
            units: 10,
            in_place_rent_mo: 1900.0,
            market_rent_mo: 2000.0,
            rent_growth: 0.0,
            loss_to_lease_burn_off_months: 4,
            turnover_rate: 0.5,
            concession_months: 1.0,
        };
        assert_eq!(loss_to_lease(&u, 1), 10_000.0);
        assert_eq!(loss_to_lease(&u, 3), 5_000.0);
        assert_eq!(loss_to_lease(&u, 5), 0.0);
        assert_eq!(loss_to_lease(&u, 50), 0.0);
    }
}
