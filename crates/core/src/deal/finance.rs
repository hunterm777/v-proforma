//! Debt, exit and equity blocks of the deal file.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Debt {
    /// A fixed loan amount; otherwise `sizing` applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizing: Option<Sizing>,
    pub rate: Rate,
    /// Interest-only months from close.
    #[serde(default)]
    pub io_months: u32,
    /// Omitted = interest only for the whole hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amortization_years: Option<u32>,
    /// Fee at close as a share of the loan.
    #[serde(default)]
    pub origination_fee_pct: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Share of the payoff balance; a list gives it by loan year (last repeats).
    pub prepayment_penalty_pct: Option<PctSchedule>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Sizing {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Maximum loan / price.
    pub max_ltv: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Minimum year-1 NOI / annual debt service.
    pub min_dscr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Minimum year-1 NOI / loan.
    pub min_debt_yield: Option<f64>,
}

/// Give exactly one of `fixed` or `floating`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Rate {
    /// Annual fixed rate as a decimal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floating: Option<Floating>,
}

/// Index + spread, annual decimals; the curve is a step function.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Floating {
    pub spread: f64,
    /// Index by model month; the first point is month 1.
    pub curve: Vec<CurvePoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CurvePoint {
    /// Model month from which this index applies.
    pub month: u32,
    /// Annual index rate as a decimal.
    pub index: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum PctSchedule {
    Flat(f64),
    ByLoanYear(Vec<f64>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Exit {
    /// Sale month; default the hold period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u32>,
    /// Applied to forward 12-month NOI.
    pub cap_rate: f64,
    /// Share of gross sale.
    #[serde(default)]
    pub selling_costs_pct: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Equity {
    /// LP and GP shares of contributions.
    pub contributions: Shares,
    /// Applied in order to every distribution; the last takes what is left.
    pub tiers: Vec<Tier>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Shares {
    pub lp: f64,
    pub gp: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Tier {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hurdle: Option<Hurdle>,
    pub split: Split,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Until>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Hurdle {
    /// Until the LP reaches this annual IRR.
    Irr {
        rate: f64,
        #[serde(default)]
        compounding: Compounding,
    },
    /// Until LP distributions reach this multiple of LP contributions.
    Multiple { multiple: f64 },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Compounding {
    Monthly,
    Quarterly,
    #[default]
    Annual,
    /// Cumulative, non-compounding.
    None,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum Split {
    /// `pro_rata`: by contributions.
    Named(String),
    Shares(Shares),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Until {
    /// Catch-up runs until GP profit / total profit reaches this share.
    pub gp_share_of_profits: f64,
}
