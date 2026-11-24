//! Golden files and invariants for every fixture and example deal.
//! `BLESS=1 cargo test -p proforma-core --test fixtures` rewrites the golden files.

use proforma_core::api;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn deals() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dir in ["fixtures/deals", "deals"] {
        for e in std::fs::read_dir(root().join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "yaml" || x == "json") {
                v.push(p);
            }
        }
    }
    v.sort();
    assert!(
        v.len() >= 9,
        "expected the M1-M4 fixtures and four example deals"
    );
    v
}

fn text(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap()
}

fn result(name: &str) -> Value {
    let p = deals()
        .into_iter()
        .find(|p| p.file_stem().unwrap() == name)
        .unwrap();
    serde_json::from_str(&api::run(&text(&p)).unwrap()).unwrap()
}

fn i(v: &Value) -> i64 {
    v.as_i64().unwrap()
}

#[test]
fn golden_files_match() {
    let bless = std::env::var("BLESS").is_ok();
    for p in deals() {
        let out = api::run(&text(&p)).unwrap();
        let g = root()
            .join("fixtures/expected")
            .join(p.file_stem().unwrap())
            .with_extension("json");
        if bless {
            std::fs::write(&g, format!("{out}\n")).unwrap();
        } else {
            let want =
                std::fs::read_to_string(&g).unwrap_or_else(|_| panic!("missing {}", g.display()));
            assert_eq!(
                out.trim_end(),
                want.trim_end(),
                "{} differs from its golden file",
                p.display()
            );
        }
    }
}

#[test]
fn runs_are_deterministic() {
    for p in deals() {
        assert_eq!(api::run(&text(&p)).unwrap(), api::run(&text(&p)).unwrap());
    }
}

#[test]
fn every_invariant_passes() {
    for p in deals() {
        let o: Value = serde_json::from_str(&api::run(&text(&p)).unwrap()).unwrap();
        let checks = o["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 4);
        for c in checks {
            assert!(
                c["passed"].as_bool().unwrap(),
                "{}: {} failed: {}",
                p.display(),
                c["name"],
                c["detail"]
            );
        }
    }
}

#[test]
fn every_annual_cell_traces_to_its_entries() {
    let lines = [
        "base_rent",
        "recoveries",
        "other_income",
        "pgr",
        "general_vacancy",
        "credit_loss",
        "free_rent",
        "egi",
        "opex",
        "management_fee",
        "noi",
        "ti",
        "lc",
        "reserves",
        "capex",
        "cash_flow_before_debt",
    ];
    for p in deals() {
        let t = text(&p);
        let o: Value = serde_json::from_str(&api::run(&t).unwrap()).unwrap();
        let rows = o["annual"]["rows"].as_array().unwrap();
        let years = o["exit_month"].as_u64().unwrap().div_ceil(12) as u32;
        for line in lines {
            let row = rows
                .iter()
                .find(|r| r["key"] == line)
                .unwrap_or_else(|| panic!("no row {line}"));
            for y in 1..=years {
                let tr: Value =
                    serde_json::from_str(&api::trace(&t, line, Some(y), None).unwrap()).unwrap();
                let sum: i64 = tr["rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| i(&r["cents"]))
                    .sum();
                assert_eq!(sum, i(&tr["total"]));
                assert_eq!(
                    sum,
                    i(&row["values"][y as usize]),
                    "{} {line} year {y}",
                    p.display()
                );
            }
        }
    }
}

#[test]
fn every_annual_cell_below_cfbd_traces_too() {
    let lines = [
        "debt_service",
        "cash_flow_after_debt",
        "purchase",
        "loan_net",
        "net_sale",
        "loan_payoff",
        "unlevered_cash_flow",
        "levered_cash_flow",
    ];
    for p in deals() {
        let t = text(&p);
        let o: Value = serde_json::from_str(&api::run(&t).unwrap()).unwrap();
        let rows = o["annual"]["rows"].as_array().unwrap();
        let years = o["exit_month"].as_u64().unwrap().div_ceil(12) as u32;
        for line in lines {
            let row = rows
                .iter()
                .find(|r| r["key"] == line)
                .unwrap_or_else(|| panic!("no row {line}"));
            let mut hold = 0;
            for y in 0..=years {
                let tr: Value =
                    serde_json::from_str(&api::trace(&t, line, Some(y), None).unwrap()).unwrap();
                let sum: i64 = tr["rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| i(&r["cents"]))
                    .sum();
                assert_eq!(sum, i(&tr["total"]));
                assert_eq!(
                    sum,
                    i(&row["values"][y as usize]),
                    "{} {line} year {y}",
                    p.display()
                );
                hold += sum;
            }
            let all: Value =
                serde_json::from_str(&api::trace(&t, line, None, None).unwrap()).unwrap();
            assert_eq!(i(&all["total"]), hold, "{} {line} whole hold", p.display());
        }
        // The levered series is traced month by month too, close included.
        let lev = o["returns"]["levered"]["cash_flows"].as_array().unwrap();
        for (m, cf) in lev.iter().enumerate().step_by(7) {
            let tr: Value = serde_json::from_str(
                &api::trace(&t, "levered_cash_flow", None, Some(m as u32)).unwrap(),
            )
            .unwrap();
            assert_eq!(i(&tr["total"]), i(cf), "{} levered month {m}", p.display());
        }
    }
}

#[test]
fn m2_harbor_legal_rolls_at_least_twice() {
    let o = result("m2_office_rollover");
    let starts: std::collections::BTreeSet<String> = o["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["source"]["id"] == "T1" && e["line"] == "base_rent")
        .filter_map(|e| {
            e["detail"]
                .as_str()
                .unwrap()
                .strip_prefix("rollover ")
                .map(str::to_string)
        })
        .collect();
    // First roll in 2028, second in 2031 (36-month speculative terms).
    assert!(starts.iter().any(|s| s.contains("from 2028-03-15")));
    assert!(starts.iter().any(|s| s.contains("from 2031-03-15")));
    assert!(starts.iter().any(|s| s.contains("from 2031-09-15")));
}

#[test]
fn m3_binding_constraints_and_payment() {
    let fixed = result("m3_fixed_sized");
    assert_eq!(fixed["debt"]["sizing"]["binding"], "dscr");
    let c = fixed["debt"]["sizing"]["candidates"].as_array().unwrap();
    let min = c.iter().map(|x| i(&x["amount"])).min().unwrap();
    assert_eq!(i(&fixed["debt"]["loan"]), min);
    // First amortizing payment is the standard annuity on the loan.
    let loan = i(&fixed["debt"]["loan"]) as f64 / 100.0;
    let r: f64 = 0.0625 / 12.0;
    let pmt = loan * r / (1.0 - (1.0 + r).powi(-360));
    let row = &fixed["debt"]["schedule"][24];
    assert_eq!(row["month"], 25);
    assert_eq!(i(&row["payment"]), (pmt * 100.0).round() as i64);
    assert_eq!(i(&fixed["debt"]["schedule"][23]["principal"]), 0);
    let floating = result("m3_floating");
    assert_eq!(floating["debt"]["sizing"]["binding"], "ltv");
}

#[test]
fn m4_lp_plus_gp_is_levered_cash_flow() {
    let o = result("m4_waterfall");
    let w = &o["waterfall"];
    let lp = w["lp"]["cash_flows"].as_array().unwrap();
    let gp = w["gp"]["cash_flows"].as_array().unwrap();
    let lev = o["returns"]["levered"]["cash_flows"].as_array().unwrap();
    assert_eq!(lp.len(), lev.len());
    for k in 0..lev.len() {
        assert_eq!(i(&lp[k]) + i(&gp[k]), i(&lev[k]), "month {k}");
    }
    assert!(i(&w["gp_promote"]) > 0);
    let tiers = w["tiers"].as_array().unwrap();
    let lp_d: i64 = tiers.iter().map(|t| i(&t["lp_total"])).sum();
    let gp_d: i64 = tiers.iter().map(|t| i(&t["gp_total"])).sum();
    assert_eq!(lp_d, i(&w["lp"]["distributed"]));
    assert_eq!(gp_d, i(&w["gp"]["distributed"]));
}

#[test]
fn sensitivity_grid_matches_single_runs() {
    let p = root().join("deals/industrial_example.yaml");
    let t = text(&p);
    let spec = r#"{"x":"exit.cap_rate","y":"market.rent_growth","x_values":[0.055,0.06,0.065],"y_values":[0.02,0.03,0.04]}"#;
    let g: Value = serde_json::from_str(&api::sensitivity(&t, spec).unwrap()).unwrap();
    let base: Value = serde_json::from_str(&api::run(&t).unwrap()).unwrap();
    assert_eq!(g["levered_irr"].as_array().unwrap().len(), 3);
    // The grid cell at the deal's own inputs reproduces the base run exactly.
    let t2 = t
        .replace("cap_rate: 0.0575", "cap_rate: 0.06")
        .replace("rent_growth: 0.035", "rent_growth: 0.03");
    let single: Value = serde_json::from_str(&api::run(&t2).unwrap()).unwrap();
    assert_eq!(g["levered_irr"][1][1], single["returns"]["levered"]["irr"]);
    assert_ne!(g["levered_irr"][1][1], base["returns"]["levered"]["irr"]);
}

#[test]
fn json_and_yaml_round_trip_to_the_same_result() {
    for p in deals() {
        let t = text(&p);
        let a = api::run(&t).unwrap();
        assert_eq!(
            api::run(&api::to_json(&t).unwrap()).unwrap(),
            a,
            "{}",
            p.display()
        );
        assert_eq!(
            api::run(&api::to_yaml(&t).unwrap()).unwrap(),
            a,
            "{}",
            p.display()
        );
    }
}

#[test]
fn invalid_deals_report_every_error() {
    let t = text(&root().join("fixtures/deals/m1_nnn_industrial.yaml"));
    let bad = t
        .replace("start_date: 2027-01-01", "start_date: 2027-01-15")
        .replace("cap_rate: 0.0625", "cap_rate: 0");
    let e = api::validate(&bad).unwrap_err();
    assert!(e.contains("start_date") && e.contains("cap_rate"), "{e}");
}
