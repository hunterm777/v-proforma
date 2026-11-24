//! Exit (step 8): sale on forward 12-month NOI.

use crate::deal::Exit;
use crate::money::{round, Cents};
use crate::trace::sum;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ExitResult {
    pub month: usize,
    /// NOI over months X + 1 to X + 12.
    pub forward_noi: Cents,
    pub cap_rate: f64,
    pub gross_sale: Cents,
    pub selling_costs: Cents,
    pub net_sale: Cents,
}

pub fn run(noi: &[Cents], month: usize, e: &Exit) -> ExitResult {
    let forward_noi = sum(noi, month + 1, month + 12);
    let gross_sale = round(forward_noi as f64 / e.cap_rate);
    let selling_costs = round(e.selling_costs_pct * gross_sale as f64);
    ExitResult {
        month,
        forward_noi,
        cap_rate: e.cap_rate,
        gross_sale,
        selling_costs,
        net_sale: gross_sale - selling_costs,
    }
}
