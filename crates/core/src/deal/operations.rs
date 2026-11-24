//! Leasing, unit mix, expenses, income and capital blocks of the deal file.

use serde::{Deserialize, Serialize};

/// Market leasing assumptions for one space type.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MarketProfile {
    /// Space type id referenced by leases.
    pub id: String,
    /// Market rent in $/SF/yr at the analysis start.
    pub market_rent_psf_yr: f64,
    /// Annual market rent growth as a decimal, compounding each analysis year.
    pub rent_growth: f64,
    /// Probability (0 to 1) that an expiring tenant renews.
    pub renewal_probability: f64,
    /// Months vacant before a new tenant.
    pub downtime_months: u32,
    /// Term of a speculative lease in months.
    pub term_months: u32,
    /// Terms for a new tenant.
    pub new: LeasingTerms,
    /// Terms for a renewing tenant.
    pub renewal: LeasingTerms,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct LeasingTerms {
    /// Months of free base rent.
    #[serde(default)]
    pub free_rent_months: u32,
    /// Tenant improvements in $/SF, paid when the lease starts.
    #[serde(default)]
    pub ti_psf: f64,
    /// Leasing commission as a share of base rent over the term (free months excluded).
    #[serde(default)]
    pub lc_pct: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MarketOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_rent_psf_yr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rent_growth: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renewal_probability: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downtime_months: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub term_months: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new: Option<LeasingTerms>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renewal: Option<LeasingTerms>,
}

/// One multifamily unit type.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitType {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Number of units.
    pub units: u32,
    /// $/unit/month in place at the start.
    pub in_place_rent_mo: f64,
    /// $/unit/month market rent at the start.
    pub market_rent_mo: f64,
    /// Annual market rent growth as a decimal.
    pub rent_growth: f64,
    /// Months for in-place rents to reach market (linear); 0 = none.
    pub loss_to_lease_burn_off_months: u32,
    /// Share of units turning over per year.
    pub turnover_rate: f64,
    /// Free months given on each turned unit.
    #[serde(default)]
    pub concession_months: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Opex {
    #[serde(default)]
    pub lines: Vec<ExpenseLine>,
    /// Share of EGI.
    #[serde(default)]
    pub management_fee_pct: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ExpenseLine {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Dollars per year at the analysis start.
    pub amount_yr: f64,
    /// Annual growth as a decimal.
    #[serde(default)]
    pub growth: f64,
    /// Included in the pool tenants reimburse.
    pub recoverable: bool,
    /// Subject to a lease's controllable-expense cap.
    #[serde(default)]
    pub controllable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct IncomeLine {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Dollars per year at the analysis start.
    pub amount_yr: f64,
    /// Annual growth as a decimal.
    #[serde(default)]
    pub growth: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Revenue {
    /// Share of PGR.
    #[serde(default)]
    pub general_vacancy_pct: f64,
    /// Share of PGR.
    #[serde(default)]
    pub credit_loss_pct: f64,
    /// true: general vacancy is not reduced by modeled downtime.
    #[serde(default)]
    pub vacancy_on_top_of_downtime: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Capital {
    /// Purchase price in dollars.
    pub price: f64,
    /// Closing costs in dollars.
    #[serde(default)]
    pub closing_costs: f64,
    /// Capital reserves in $/SF/yr (office, retail, industrial).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserves_per_sf_yr: Option<f64>,
    /// Capital reserves in $/unit/yr (multifamily).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserves_per_unit_yr: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capex: Vec<CapexItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CapexItem {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// Model month (1 = first month).
    pub month: u32,
    /// Dollars.
    pub amount: f64,
}
