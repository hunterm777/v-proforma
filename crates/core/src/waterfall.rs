//! Equity waterfall (step 10): American timing, every levered cash flow runs
//! through the tiers in order. Hurdles are measured on the LP's cash flows.

use crate::deal::{Compounding, Equity, Hurdle, Shares, Split, Tier};
use crate::money::{round, Cents};
use crate::returns::{annualise, irr_monthly, xirr};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Irr {
        rate: f64,
        period: u32,
        compound: bool,
    },
    Multiple(f64),
    CatchUp(f64),
    Residual,
}

fn kind(t: &Tier, last: bool) -> Kind {
    match (t.hurdle, t.until) {
        (Some(Hurdle::Irr { rate, compounding }), _) => {
            let (period, compound) = match compounding {
                Compounding::Monthly => (1, true),
                Compounding::Quarterly => (3, true),
                Compounding::Annual => (12, true),
                Compounding::None => (12, false),
            };
            Kind::Irr {
                rate,
                period,
                compound,
            }
        }
        (Some(Hurdle::Multiple { multiple }), _) => Kind::Multiple(multiple),
        (None, Some(u)) => Kind::CatchUp(u.gp_share_of_profits),
        (None, None) if last => Kind::Residual,
        (None, None) => Kind::Multiple(1.0),
    }
}

fn split(s: &Split, c: Shares) -> Option<Shares> {
    match s {
        Split::Named(n) if n == "pro_rata" => Some(c),
        Split::Named(_) => None,
        Split::Shares(x) => Some(*x),
    }
}

fn shares_ok(s: Shares) -> bool {
    s.lp >= 0.0 && s.gp >= 0.0 && (s.lp + s.gp - 1.0).abs() < 1e-9
}

/// Validation of the equity block.
pub fn check(q: &Equity, e: &mut Vec<String>) {
    if !shares_ok(q.contributions) {
        e.push("equity.contributions: lp and gp must be >= 0 and sum to 1".into());
    }
    if q.tiers.is_empty() {
        e.push("equity.tiers: at least one tier".into());
    }
    let n = q.tiers.len();
    for (i, t) in q.tiers.iter().enumerate() {
        let at = format!("equity.tiers[{i}] ({})", t.name);
        let Some(s) = split(&t.split, q.contributions) else {
            e.push(format!("{at}: split must be pro_rata or {{lp, gp}}"));
            continue;
        };
        if !shares_ok(s) {
            e.push(format!("{at}: split lp and gp must be >= 0 and sum to 1"));
        }
        if t.hurdle.is_some() && t.until.is_some() {
            e.push(format!("{at}: give a hurdle or until, not both"));
        }
        let k = kind(t, i + 1 == n);
        if i + 1 == n && k != Kind::Residual {
            e.push(format!(
                "{at}: the last tier must take what is left (no hurdle, no until)"
            ));
        }
        match k {
            Kind::Irr { rate, .. } if !(rate.is_finite() && rate >= 0.0) => {
                e.push(format!("{at}: hurdle rate must be >= 0"))
            }
            Kind::Multiple(m) if !(m.is_finite() && m >= 0.0) => {
                e.push(format!("{at}: multiple must be >= 0"))
            }
            Kind::CatchUp(x) if !(x > 0.0 && x < 1.0 && s.gp > x) => e.push(format!(
                "{at}: gp_share_of_profits must be in (0, 1) and below the tier's gp split"
            )),
            Kind::Irr { .. } | Kind::Multiple(_) if s.lp <= 0.0 => {
                e.push(format!("{at}: a hurdle tier needs an lp split above 0"))
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Acct {
    cap: f64,
    capint: f64,
    acc: f64,
}

impl Acct {
    fn pay(&mut self, mut x: f64) {
        let a = x.min(self.acc.max(0.0));
        self.acc -= a;
        x -= a;
        let b = x.min(self.capint.max(0.0));
        self.capint -= b;
        x -= b;
        self.cap -= x;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TierFlows {
    pub name: String,
    /// Distributions by month (index = month, 0 = close).
    pub lp: Vec<Cents>,
    pub gp: Vec<Cents>,
    /// Totals over the hold.
    pub lp_total: Cents,
    pub gp_total: Cents,
}

#[derive(Clone, Debug, Serialize)]
pub struct Partner {
    /// Net cash flow by month: distributions less contributions.
    pub cash_flows: Vec<Cents>,
    pub contributed: Cents,
    pub distributed: Cents,
    pub irr: Option<f64>,
    pub xirr: Option<f64>,
    pub equity_multiple: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WaterfallResult {
    pub tiers: Vec<TierFlows>,
    pub lp: Partner,
    pub gp: Partner,
    /// GP distributions less its contribution share of all distributions.
    pub gp_promote: Cents,
}

fn partner(contrib: &[Cents], dist: &[Cents], days: &[i64]) -> Partner {
    let cf: Vec<Cents> = dist.iter().zip(contrib).map(|(d, c)| d - c).collect();
    let f: Vec<f64> = cf.iter().map(|c| *c as f64).collect();
    let (c, d): (Cents, Cents) = (contrib.iter().sum(), dist.iter().sum());
    Partner {
        irr: irr_monthly(&f).map(annualise),
        xirr: xirr(&f, days),
        equity_multiple: (c > 0).then(|| d as f64 / c as f64),
        contributed: c,
        distributed: d,
        cash_flows: cf,
    }
}

/// Distributes levered cash flows (month 0 to exit) through the tiers.
pub fn run(q: &Equity, levered: &[Cents], days: &[i64]) -> WaterfallResult {
    let n = q.tiers.len();
    let kinds: Vec<Kind> = q
        .tiers
        .iter()
        .enumerate()
        .map(|(i, t)| kind(t, i + 1 == n))
        .collect();
    let splits: Vec<Shares> = q
        .tiers
        .iter()
        .map(|t| split(&t.split, q.contributions).expect("validated"))
        .collect();
    let mut accts = vec![Acct::default(); n];
    let len = levered.len();
    let mut tiers: Vec<TierFlows> = q
        .tiers
        .iter()
        .map(|t| TierFlows {
            name: t.name.clone(),
            lp: vec![0; len],
            gp: vec![0; len],
            lp_total: 0,
            gp_total: 0,
        })
        .collect();
    let (mut lpc, mut gpc) = (vec![0; len], vec![0; len]);
    let (mut lp_c, mut gp_c, mut lp_d, mut gp_d) = (0i64, 0i64, 0i64, 0i64);
    for (t, &cf) in levered.iter().enumerate() {
        for (k, a) in kinds.iter().zip(accts.iter_mut()) {
            if let (
                Kind::Irr {
                    rate,
                    period,
                    compound,
                },
                true,
            ) = (k, t >= 1)
            {
                let base = if *compound { a.cap + a.capint } else { a.cap };
                a.acc += base.max(0.0) * rate / 12.0;
                if *compound && t as u32 % period == 0 {
                    a.capint += a.acc;
                    a.acc = 0.0;
                }
            }
        }
        if cf < 0 {
            let lp = round(-cf as f64 * q.contributions.lp);
            lpc[t] = lp;
            gpc[t] = -cf - lp;
            lp_c += lp;
            gp_c += -cf - lp;
            accts.iter_mut().for_each(|a| a.cap += lp as f64);
            continue;
        }
        let mut left = cf;
        for i in 0..n {
            if left == 0 {
                break;
            }
            let s = splits[i];
            let need = match kinds[i] {
                Kind::Irr { .. } => (accts[i].cap + accts[i].capint + accts[i].acc).max(0.0) / s.lp,
                Kind::Multiple(m) => (m * lp_c as f64 - lp_d as f64).max(0.0) / s.lp,
                Kind::CatchUp(x) => {
                    let total = (lp_d + gp_d - lp_c - gp_c) as f64;
                    ((x * total - (gp_d - gp_c) as f64) / (s.gp - x)).max(0.0)
                }
                Kind::Residual => left as f64,
            };
            let amt = (need.ceil() as Cents).min(left);
            if amt <= 0 {
                continue;
            }
            let lp = round(amt as f64 * s.lp);
            let gp = amt - lp;
            tiers[i].lp[t] += lp;
            tiers[i].gp[t] += gp;
            lp_d += lp;
            gp_d += gp;
            accts.iter_mut().for_each(|a| a.pay(lp as f64));
            left -= amt;
        }
    }
    let sum = |f: fn(&TierFlows) -> &Vec<Cents>| -> Vec<Cents> {
        (0..len)
            .map(|t| tiers.iter().map(|x| f(x)[t]).sum())
            .collect()
    };
    let (lpd, gpd) = (sum(|x| &x.lp), sum(|x| &x.gp));
    let promote = gp_d - round(q.contributions.gp * (lp_d + gp_d) as f64);
    for t in &mut tiers {
        t.lp_total = t.lp.iter().sum();
        t.gp_total = t.gp.iter().sum();
    }
    WaterfallResult {
        lp: partner(&lpc, &lpd, days),
        gp: partner(&gpc, &gpd, days),
        tiers,
        gp_promote: promote,
    }
}
