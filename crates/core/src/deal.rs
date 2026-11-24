//! The deal file: one readable YAML or JSON document.
//!
//! Money is in dollars, rates and percents are decimals (0.05 = 5%), areas in
//! square feet, terms in months. Unknown fields are rejected. Checks across
//! fields live in `validate`.

use crate::calendar::Date;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod finance;
mod operations;
mod rules;
mod validate;
pub use finance::*;
pub use operations::*;
pub use validate::{validate, LeaseModel, Model};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Deal {
    pub property: Property,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rent_roll: Vec<Lease>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unit_mix: Vec<UnitType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub market: Vec<MarketProfile>,
    #[serde(default)]
    pub opex: Opex,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_income: Vec<IncomeLine>,
    #[serde(default)]
    pub revenue: Revenue,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub recoveries: BTreeMap<String, Recovery>,
    pub capital: Capital,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt: Option<Debt>,
    pub exit: Exit,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equity: Option<Equity>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Office,
    Retail,
    Industrial,
    Multifamily,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Property {
    /// Display name.
    pub name: String,
    /// office, retail, industrial or multifamily.
    #[serde(rename = "type")]
    pub kind: PropertyType,
    /// Rentable square feet (office, retail, industrial).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rentable_sf: Option<f64>,
    /// Unit count (multifamily); must equal the unit mix total.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<u32>,
    /// Analysis start: the first day of a month. Acquisition closes on it.
    pub start_date: Date,
    /// Hold period in months.
    pub hold_months: u32,
}

/// One lease on the rent roll.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Lease {
    /// Unique lease id.
    pub id: String,
    /// Tenant name.
    pub tenant: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub suite: String,
    /// Leased area in square feet.
    pub sf: f64,
    /// First day in effect.
    pub start: Date,
    /// Last day in effect.
    pub expiry: Date,
    /// Base rent in $/SF/yr in effect on the start date (lease year 1).
    pub rent_psf_yr: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<Steps>,
    /// Months of base rent abated from the lease start.
    #[serde(default)]
    pub free_rent_months: u32,
    /// A name from `recoveries`, or an inline structure. Default: `recoveries.default`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<RecoveryRef>,
    /// Override of sf / rentable_sf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pro_rata_share: Option<f64>,
    /// Market profile id used when the lease expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_type: Option<String>,
    /// Per-lease overrides of the market profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market: Option<MarketOverride>,
    /// Dollars held; reported only, no cash-flow effect.
    #[serde(default)]
    pub security_deposit: f64,
}

/// Rent steps on each lease anniversary.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Steps {
    /// `rate` per year, compounding.
    Percent {
        /// Annual increase as a decimal.
        rate: f64,
    },
    /// A fixed $/SF/yr increase per year.
    Fixed {
        /// $/SF/yr added each lease year.
        amount_psf_yr: f64,
    },
    /// One % per lease year (CPI-style); the last repeats.
    Cpi {
        /// Increase for lease year 2, 3, ... as decimals; the last repeats.
        rates: Vec<f64>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RecoveryRef {
    Name(String),
    Inline(Recovery),
}

/// How a tenant reimburses operating expenses.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Recovery {
    Nnn {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cap_controllable_pct: Option<f64>,
    },
    BaseYear {
        year: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cap_controllable_pct: Option<f64>,
    },
    ExpenseStop {
        stop_psf_yr: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cap_controllable_pct: Option<f64>,
    },
    ModifiedGross {
        lines: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cap_controllable_pct: Option<f64>,
    },
    Gross,
}

/// Reads a deal from YAML or JSON text (JSON is valid YAML, but JSON is
/// parsed by the JSON parser for exact error positions).
pub fn parse(text: &str) -> Result<Deal, String> {
    let t = text.trim_start();
    if t.starts_with('{') {
        serde_json::from_str(text).map_err(|e| format!("deal JSON: {e}"))
    } else {
        serde_norway::from_str(text).map_err(|e| format!("deal YAML: {e}"))
    }
}

/// Writes a deal as YAML.
pub fn to_yaml(deal: &Deal) -> Result<String, String> {
    serde_norway::to_string(deal).map_err(|e| e.to_string())
}
