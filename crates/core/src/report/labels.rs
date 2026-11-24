//! Display labels for report lines.

pub(super) fn label(key: &str) -> &'static str {
    match key {
        "base_rent" => "Base rent",
        "recoveries" => "Expense recoveries",
        "other_income" => "Other income",
        "pgr" => "Potential gross revenue",
        "general_vacancy" => "General vacancy",
        "credit_loss" => "Credit loss",
        "free_rent" => "Free rent",
        "egi" => "Effective gross income",
        "opex" => "Operating expenses",
        "management_fee" => "Management fee",
        "noi" => "Net operating income",
        "ti" => "Tenant improvements",
        "lc" => "Leasing commissions",
        "reserves" => "Capital reserves",
        "capex" => "Capital expenditures",
        "cash_flow_before_debt" => "Cash flow before debt",
        "debt_service" => "Debt service",
        "cash_flow_after_debt" => "Cash flow after debt",
        "purchase" => "Purchase price and closing costs",
        "loan_net" => "Loan proceeds net of fees",
        "net_sale" => "Net sale proceeds",
        "loan_payoff" => "Loan payoff and prepayment",
        "unlevered_cash_flow" => "Unlevered cash flow",
        "levered_cash_flow" => "Levered cash flow",
        _ => "",
    }
}
