//! proforma core: an auditable commercial real estate pro forma.
//!
//! A deal file goes in; lease-by-lease monthly cash flows, NOI, debt,
//! unlevered and levered returns and the LP/GP waterfall come out. All math
//! lives here; the WASM page and the Python package call [`api`]. The method
//! is documented in `docs/methodology.md`.

#![forbid(unsafe_code)]

pub mod api;
pub mod calendar;
pub mod capital;
pub mod deal;
pub mod debt;
pub mod engine;
pub mod exit;
pub mod lease;
pub mod money;
pub mod multifamily;
pub mod opex;
pub mod recoveries;
pub mod report;
pub mod returns;
pub mod revenue;
pub mod rollover;
pub mod sensitivity;
pub mod trace;
pub mod waterfall;
