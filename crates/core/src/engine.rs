//! Runs the ten steps in order; each step reads only the steps above it.

use crate::deal::{self, Deal, Model, PropertyType};
use crate::debt::{self, DebtResult};
use crate::exit::{self, ExitResult};
use crate::money::Cents;
use crate::opex::{self, Pool};
use crate::recoveries::PoolCache;
use crate::returns::{series, Series};
use crate::trace::{sum, Ledger};
use crate::waterfall::{self, WaterfallResult};
use crate::{capital, multifamily, revenue, rollover};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct SourcesUses {
    pub equity: Cents,
    pub loan: Cents,
    pub price: Cents,
    pub closing_costs: Cents,
    pub loan_fees: Cents,
}

/// Everything one run produces, before formatting.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub model: Model,
    pub ledger: Ledger,
    /// Modeled downtime vacancy (memo), unrounded cents by month.
    pub turnover: Vec<f64>,
    /// Expected occupied SF by month (commercial).
    pub occupied_sf: Vec<f64>,
    pub noi: Vec<Cents>,
    pub cfbd: Vec<Cents>,
    pub debt: Option<DebtResult>,
    pub exit: ExitResult,
    pub sources_uses: SourcesUses,
    pub unlevered: Series,
    pub levered: Series,
    pub waterfall: Option<WaterfallResult>,
    /// Day offset of month t (0 = close) from the start date, for XIRR.
    pub days: Vec<i64>,
}

/// Parses nothing: runs an already-read deal.
pub fn run(deal: &Deal) -> Result<Outcome, String> {
    let model = deal::validate(deal)?;
    let d = &model.deal;
    let cal = &model.cal;
    let n = cal.months;
    let x = model.exit_month;
    let mut ledger = Ledger::new(n);
    let pools = PoolCache::new(Pool::new(&d.opex.lines), n);
    let mut turnover = vec![0.0; n];
    let mut occupied_sf = vec![0.0; n];
    for lm in &model.leases {
        let (v, occ) = rollover::lease_entries(&mut ledger, lm, cal, &pools);
        for i in 0..n {
            turnover[i] += v[i];
            occupied_sf[i] += occ[i] * lm.lease.sf;
        }
    }
    multifamily::entries(&mut ledger, &d.unit_mix);
    opex::income(&mut ledger, &d.other_income);
    revenue::vacancy(&mut ledger, &d.revenue, &turnover);
    opex::expenses(&mut ledger, &d.opex.lines);
    revenue::management_fee(&mut ledger, d.opex.management_fee_pct);
    let basis = if d.property.kind == PropertyType::Multifamily {
        f64::from(model.units)
    } else {
        model.rsf
    };
    capital::entries(&mut ledger, &d.capital, basis, x);
    let noi = ledger.line("noi").expect("noi");
    let cfbd = ledger.line("cash_flow_before_debt").expect("cfbd");

    let debt = d
        .debt
        .as_ref()
        .map(|dd| debt::run(dd, model.price, sum(&noi, 1, 12), x));
    let exit = exit::run(&noi, x, &d.exit);
    let (loan, fee) = debt.as_ref().map_or((0, 0), |r| (r.loan, r.fee));
    let cost = model.price + model.closing;
    let sources_uses = SourcesUses {
        equity: cost + fee - loan,
        loan,
        price: model.price,
        closing_costs: model.closing,
        loan_fees: fee,
    };

    let days: Vec<i64> = (0..=x as i64)
        .map(|t| cal.start.add_months(t).day() - cal.start.day())
        .collect();
    let mut un = vec![-cost];
    un.extend_from_slice(&cfbd[..x]);
    un[x] += exit.net_sale;
    let ds: Vec<Cents> = (0..x)
        .map(|i| debt.as_ref().map_or(0, |r| r.schedule[i].payment))
        .collect();
    let lev_op: Vec<Cents> = (0..x).map(|i| cfbd[i] - ds[i]).collect();
    let mut lev = vec![-sources_uses.equity];
    lev.extend_from_slice(&lev_op);
    lev[x] += exit.net_sale - debt.as_ref().map_or(0, |r| r.payoff + r.prepayment);
    let unlevered = series(un, &cfbd[..x], &days);
    let levered = series(lev, &lev_op, &days);
    let waterfall = d
        .equity
        .as_ref()
        .map(|q| waterfall::run(q, &levered.cash_flows, &days));
    Ok(Outcome {
        model,
        ledger,
        turnover,
        occupied_sf,
        noi,
        cfbd,
        debt,
        exit,
        sources_uses,
        unlevered,
        levered,
        waterfall,
        days,
    })
}
