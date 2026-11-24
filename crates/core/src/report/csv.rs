//! CSV per report. Money is dollars with two decimals, exact from cents.

use super::Output;
use crate::money::Cents;

/// Cents as an exact dollar string.
pub fn usd(c: Cents) -> String {
    let sign = if c < 0 { "-" } else { "" };
    let a = c.unsigned_abs();
    format!("{sign}{}.{:02}", a / 100, a % 100)
}

fn esc(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn opt(x: Option<f64>) -> String {
    x.map_or(String::new(), |v| v.to_string())
}

fn table(t: &super::Table) -> String {
    let mut s = format!(
        "line,{}\n",
        t.columns
            .iter()
            .map(|c| esc(c))
            .collect::<Vec<_>>()
            .join(",")
    );
    for r in &t.rows {
        let v: Vec<String> = r.values.iter().map(|c| usd(*c)).collect();
        s += &format!("{},{}\n", esc(&r.label), v.join(","));
    }
    s
}

/// Report names accepted by [`csv`].
pub const REPORTS: [&str; 8] = [
    "annual",
    "monthly",
    "rent_roll",
    "tenants",
    "debt",
    "returns",
    "waterfall",
    "entries",
];

/// One report as CSV.
pub fn csv(o: &Output, report: &str) -> Result<String, String> {
    Ok(match report {
        "annual" => table(&o.annual),
        "monthly" => table(&o.monthly),
        "rent_roll" => {
            let mut s = String::from("expiration_year,sf,pct_sf,pct_rent,leases\n");
            for e in &o.rent_roll.expirations {
                let y = e.year.map_or("after hold".into(), |y| y.to_string());
                s += &format!(
                    "{y},{},{},{},{}\n",
                    e.sf,
                    e.pct_sf,
                    e.pct_rent,
                    esc(&e.leases.join(" "))
                );
            }
            s += &format!("\nwalt_years,{}\n", opt(o.rent_roll.walt_years));
            s += "\nyear,occupancy\n";
            for (i, v) in o.rent_roll.occupancy_by_year.iter().enumerate() {
                s += &format!("{},{v}\n", i + 1);
            }
            s
        }
        "tenants" => {
            let mut s = String::from(
                "lease_id,tenant,suite,sf,start,expiry,in_place_rent_psf_yr,security_deposit,line,",
            );
            let years = o.annual.columns.len() - 1;
            s += &(1..=years)
                .map(|y| format!("Year {y}"))
                .collect::<Vec<_>>()
                .join(",");
            s.push('\n');
            for t in &o.tenants {
                for (line, v) in &t.years {
                    let v: Vec<String> = v.iter().map(|c| usd(*c)).collect();
                    s += &format!(
                        "{},{},{},{},{},{},{},{},{line},{}\n",
                        esc(&t.lease_id),
                        esc(&t.tenant),
                        esc(&t.suite),
                        t.sf,
                        t.start,
                        t.expiry,
                        t.in_place_rent_psf_yr,
                        t.security_deposit,
                        v.join(",")
                    );
                }
            }
            s
        }
        "debt" => {
            let Some(d) = &o.debt else {
                return Ok("no debt\n".into());
            };
            let mut s = String::from("month,rate,opening,interest,principal,payment,closing\n");
            for p in &d.result.schedule {
                s += &format!(
                    "{},{},{},{},{},{},{}\n",
                    p.month,
                    p.rate,
                    usd(p.opening),
                    usd(p.interest),
                    usd(p.principal),
                    usd(p.payment),
                    usd(p.closing)
                );
            }
            s
        }
        "returns" => {
            let mut s = String::from("measure,unlevered,levered\n");
            let (u, l) = (&o.returns.unlevered, &o.returns.levered);
            s += &format!("irr,{},{}\n", opt(u.irr), opt(l.irr));
            s += &format!("xirr,{},{}\n", opt(u.xirr), opt(l.xirr));
            s += &format!(
                "equity_multiple,{},{}\n",
                opt(u.equity_multiple),
                opt(l.equity_multiple)
            );
            s += &format!("profit,{},{}\n", usd(u.profit), usd(l.profit));
            s += &format!(
                "peak_equity,{},{}\n",
                usd(u.peak_equity),
                usd(l.peak_equity)
            );
            let pb = |x: Option<usize>| x.map_or(String::new(), |v| v.to_string());
            s += &format!(
                "payback_month,{},{}\n",
                pb(u.payback_month),
                pb(l.payback_month)
            );
            for (i, (a, b)) in u.cash_on_cash.iter().zip(&l.cash_on_cash).enumerate() {
                s += &format!("cash_on_cash_year_{},{a},{b}\n", i + 1);
            }
            s
        }
        "waterfall" => {
            let Some(w) = &o.waterfall else {
                return Ok("no equity waterfall\n".into());
            };
            let mut s = String::from("month,tier,lp,gp\n");
            for t in 0..o.returns.levered.cash_flows.len() {
                for tier in &w.tiers {
                    if tier.lp[t] != 0 || tier.gp[t] != 0 {
                        s += &format!(
                            "{t},{},{},{}\n",
                            esc(&tier.name),
                            usd(tier.lp[t]),
                            usd(tier.gp[t])
                        );
                    }
                }
            }
            s
        }
        "entries" => {
            let mut s = String::from("line,source_kind,source_id,step,detail,from_month,values\n");
            for e in &o.entries {
                let kind = serde_json::to_value(e.source.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default();
                let v: Vec<String> = e.values.iter().map(|c| usd(*c)).collect();
                s += &format!(
                    "{},{kind},{},{},{},{},{}\n",
                    e.line.key(),
                    esc(&e.source.id),
                    esc(e.step),
                    esc(&e.detail),
                    e.from_month,
                    esc(&v.join(" "))
                );
            }
            s
        }
        _ => {
            return Err(format!(
                "unknown report \"{report}\"; one of {}",
                REPORTS.join(", ")
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn usd_is_exact() {
        assert_eq!(super::usd(123_456), "1234.56");
        assert_eq!(super::usd(-5), "-0.05");
        assert_eq!(super::usd(0), "0.00");
    }
}
