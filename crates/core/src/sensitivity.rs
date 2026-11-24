//! Two-way sensitivity tables of levered IRR and equity multiple, computed in
//! the core by re-running the deal with two inputs changed.

use crate::deal::Deal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Dotted path into the deal, e.g. `exit.cap_rate`. A path that passes
    /// through a list sets the field on every element (`market.rent_growth`),
    /// unless the segment is an index (`market.0.rent_growth`).
    pub x: String,
    pub y: String,
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Grid {
    pub x: String,
    pub y: String,
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
    /// `[y][x]`; null where the run has no IRR or fails.
    pub levered_irr: Vec<Vec<Option<f64>>>,
    pub equity_multiple: Vec<Vec<Option<f64>>>,
    /// Errors by cell, `[y][x]`, empty when the run succeeded.
    pub errors: Vec<Vec<String>>,
}

/// Sets `value` at a dotted path. Returns how many fields were set.
pub fn set(v: &mut Value, path: &[&str], value: f64) -> Result<usize, String> {
    let Some((head, rest)) = path.split_first() else {
        *v = serde_json::json!(value);
        return Ok(1);
    };
    match v {
        Value::Object(map) => match map.get_mut(*head) {
            Some(child) => set(child, rest, value),
            None if rest.is_empty() => {
                map.insert(head.to_string(), serde_json::json!(value));
                Ok(1)
            }
            None => Err(format!("no field \"{head}\"")),
        },
        Value::Array(items) => match head.parse::<usize>() {
            Ok(i) => set(
                items.get_mut(i).ok_or(format!("no element {i}"))?,
                rest,
                value,
            ),
            Err(_) => {
                let mut n = 0;
                for it in items.iter_mut() {
                    n += set(it, path, value)?;
                }
                Ok(n)
            }
        },
        _ => Err(format!("\"{head}\" is not inside an object or list")),
    }
}

fn with(base: &Value, x: &str, xv: f64, y: &str, yv: f64) -> Result<Deal, String> {
    let mut v = base.clone();
    for (p, val) in [(x, xv), (y, yv)] {
        let parts: Vec<&str> = p.split('.').collect();
        if set(&mut v, &parts, val)? == 0 {
            return Err(format!("path \"{p}\" matched nothing"));
        }
    }
    serde_json::from_value(v).map_err(|e| e.to_string())
}

/// Runs the grid. Path errors that make every cell fail are returned as an error.
pub fn run(deal: &Deal, spec: &Spec) -> Result<Grid, String> {
    if spec.x_values.is_empty() || spec.y_values.is_empty() {
        return Err("x_values and y_values must not be empty".into());
    }
    if spec.x_values.len() * spec.y_values.len() > 2500 {
        return Err("at most 2,500 cells".into());
    }
    let base = serde_json::to_value(deal).map_err(|e| e.to_string())?;
    with(&base, &spec.x, spec.x_values[0], &spec.y, spec.y_values[0])?;
    let (mut irr, mut em, mut errs) = (Vec::new(), Vec::new(), Vec::new());
    for yv in &spec.y_values {
        let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
        for xv in &spec.x_values {
            match with(&base, &spec.x, *xv, &spec.y, *yv).and_then(|d| crate::engine::run(&d)) {
                Ok(o) => {
                    a.push(o.levered.irr);
                    b.push(o.levered.equity_multiple);
                    c.push(String::new());
                }
                Err(e) => {
                    a.push(None);
                    b.push(None);
                    c.push(e);
                }
            }
        }
        irr.push(a);
        em.push(b);
        errs.push(c);
    }
    Ok(Grid {
        x: spec.x.clone(),
        y: spec.y.clone(),
        x_values: spec.x_values.clone(),
        y_values: spec.y_values.clone(),
        levered_irr: irr,
        equity_multiple: em,
        errors: errs,
    })
}
