// Shapes of the deal file (crates/core/src/deal*.rs) and of the result JSON
// (crates/core/src/report.rs). Money in deal files is dollars; money in
// results is integer cents.

export type PropertyType = "office" | "retail" | "industrial" | "multifamily";

export interface Property {
  name: string;
  type: PropertyType;
  rentable_sf?: number;
  units?: number;
  start_date: string;
  hold_months: number;
}

export type Steps =
  | { type: "percent"; rate: number }
  | { type: "fixed"; amount_psf_yr: number }
  | { type: "cpi"; rates: number[] };

export type Recovery =
  | { type: "nnn"; cap_controllable_pct?: number }
  | { type: "base_year"; year: number; cap_controllable_pct?: number }
  | { type: "expense_stop"; stop_psf_yr: number; cap_controllable_pct?: number }
  | { type: "modified_gross"; lines: string[]; cap_controllable_pct?: number }
  | { type: "gross" };

export type RecoveryType = Recovery["type"];

export interface LeasingTerms {
  free_rent_months?: number;
  ti_psf?: number;
  lc_pct?: number;
}

export interface Lease {
  id: string;
  tenant: string;
  suite?: string;
  sf: number;
  start: string;
  expiry: string;
  rent_psf_yr: number;
  steps?: Steps;
  free_rent_months?: number;
  recovery?: string | Recovery;
  pro_rata_share?: number;
  space_type?: string;
  /** Per-lease market overrides; kept as loaded, not edited on the page. */
  market?: Record<string, unknown>;
  security_deposit?: number;
}

export interface UnitType {
  id: string;
  name?: string;
  units: number;
  in_place_rent_mo: number;
  market_rent_mo: number;
  rent_growth: number;
  loss_to_lease_burn_off_months: number;
  turnover_rate: number;
  concession_months?: number;
}

export interface MarketProfile {
  id: string;
  market_rent_psf_yr: number;
  rent_growth: number;
  renewal_probability: number;
  downtime_months: number;
  term_months: number;
  new: LeasingTerms;
  renewal: LeasingTerms;
}

export interface ExpenseLine {
  id: string;
  name?: string;
  amount_yr: number;
  growth?: number;
  recoverable: boolean;
  controllable?: boolean;
}

export interface IncomeLine {
  id: string;
  name?: string;
  amount_yr: number;
  growth?: number;
}

export interface CapexItem {
  id: string;
  name?: string;
  month: number;
  amount: number;
}

export interface Debt {
  amount?: number;
  sizing?: { max_ltv?: number; min_dscr?: number; min_debt_yield?: number };
  rate: { fixed?: number; floating?: { spread: number; curve: { month: number; index: number }[] } };
  io_months?: number;
  amortization_years?: number;
  origination_fee_pct?: number;
  prepayment_penalty_pct?: number | number[];
}

export type Compounding = "monthly" | "quarterly" | "annual" | "none";
export type Hurdle = { type: "irr"; rate: number; compounding?: Compounding } | { type: "multiple"; multiple: number };
export interface Shares {
  lp: number;
  gp: number;
}
export interface Tier {
  name: string;
  hurdle?: Hurdle;
  split: string | Shares;
  until?: { gp_share_of_profits: number };
}

export interface Deal {
  property: Property;
  rent_roll?: Lease[];
  unit_mix?: UnitType[];
  market?: MarketProfile[];
  opex?: { lines?: ExpenseLine[]; management_fee_pct?: number };
  other_income?: IncomeLine[];
  revenue?: { general_vacancy_pct?: number; credit_loss_pct?: number; vacancy_on_top_of_downtime?: boolean };
  recoveries?: Record<string, Recovery>;
  capital: {
    price: number;
    closing_costs?: number;
    reserves_per_sf_yr?: number;
    reserves_per_unit_yr?: number;
    capex?: CapexItem[];
  };
  debt?: Debt;
  exit: { month?: number; cap_rate: number; selling_costs_pct?: number };
  equity?: { contributions: Shares; tiers: Tier[] };
}

// ---- Result ----

export interface Row {
  key: string;
  label: string;
  values: number[];
}

export interface Table {
  columns: string[];
  rows: Row[];
}

export interface Source {
  kind: "lease" | "unit_type" | "expense" | "income" | "capex" | "assumption";
  id: string;
}

export interface Entry {
  line: string;
  source: Source;
  step: string;
  detail: string;
  years: number[];
  from_month: number;
  values: number[];
}

export interface Series {
  cash_flows: number[];
  irr: number | null;
  irr_monthly: number | null;
  xirr: number | null;
  irr_npv_residual: number | null;
  equity_multiple: number | null;
  profit: number;
  cash_on_cash: number[];
  peak_equity: number;
  payback_month: number | null;
}

export interface Payment {
  month: number;
  rate: number;
  opening: number;
  interest: number;
  principal: number;
  payment: number;
  closing: number;
}

export interface DebtReport {
  loan: number;
  fee: number;
  sizing: { noi_year1: number; rate: number; candidates: { constraint: string; amount: number }[]; binding: string } | null;
  schedule: Payment[];
  payoff: number;
  prepayment_pct: number;
  prepayment: number;
  by_year: { year: number; debt_service: number; dscr: number | null; debt_yield: number | null }[];
}

export interface Partner {
  cash_flows: number[];
  contributed: number;
  distributed: number;
  irr: number | null;
  xirr: number | null;
  equity_multiple: number | null;
}

export interface Result {
  engine: string;
  deal_bar: {
    name: string;
    type: PropertyType;
    price: number;
    going_in_cap: number;
    levered_irr: number | null;
    equity_multiple: number | null;
    unlevered_irr: number | null;
  };
  start_date: string;
  exit_month: number;
  model_months: number;
  annual: Table;
  monthly: Table;
  entries: Entry[];
  rent_roll: {
    walt_years: number | null;
    occupancy_by_year: number[];
    expirations: { year: number | null; sf: number; pct_sf: number; pct_rent: number; leases: string[] }[];
    unit_mix: { id: string; units: number; in_place_rent_mo: number; market_rent_mo: number; loss_to_lease_pct: number }[];
  };
  tenants: {
    lease_id: string;
    tenant: string;
    suite: string;
    sf: number;
    start: string;
    expiry: string;
    security_deposit: number;
    in_place_rent_psf_yr: number;
    years: Record<string, number[]>;
  }[];
  debt: DebtReport | null;
  exit: { month: number; forward_noi: number; cap_rate: number; gross_sale: number; selling_costs: number; net_sale: number };
  returns: {
    unlevered: Series;
    levered: Series;
    sources_uses: { equity: number; loan: number; price: number; closing_costs: number; loan_fees: number };
  };
  waterfall: {
    /** lp and gp by month (0 = close); the totals are cents over the hold. */
    tiers: { name: string; lp: number[]; gp: number[]; lp_total: number; gp_total: number }[];
    lp: Partner;
    gp: Partner;
    gp_promote: number;
  } | null;
  checks: { name: string; passed: boolean; detail: string }[];
}

export interface TraceResult {
  line: string;
  from_month: number;
  to_month: number;
  total: number;
  rows: { source: Source; line: string; step: string; detail: string; cents: number }[];
}

export interface SensitivityGrid {
  x: string;
  y: string;
  x_values: number[];
  y_values: number[];
  levered_irr: (number | null)[][];
  equity_multiple: (number | null)[][];
  errors: string[][];
}
