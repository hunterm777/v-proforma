// The PRD's benchmark deal: 200 leases over a 10-year hold, with a spread of
// starts, terms and expiries so rollover runs many times. Deterministic.
export function benchDeal() {
  let seed = 20261124;
  const rnd = () => ((seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648);
  const pick = (xs) => xs[Math.floor(rnd() * xs.length)];
  const iso = (y, m, d) => `${y}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
  const lastDay = (y, m) => new Date(Date.UTC(y, m, 0)).getUTCDate();
  const leases = [];
  let sfTotal = 0;
  for (let i = 0; i < 200; i++) {
    const sf = 1000 + Math.round(rnd() * 19) * 500;
    sfTotal += sf;
    const term = pick([36, 60, 84, 120, 180]);
    // Expiries land from 2027 to 2040: some inside the hold, some after it.
    const expY = 2027 + Math.floor(rnd() * 14);
    const expM = 1 + Math.floor(rnd() * 12);
    const startTotal = expY * 12 + expM - term;
    const sy = Math.floor(startTotal / 12);
    const sm = startTotal % 12 + 1;
    const steps = pick([
      { type: "percent", rate: pick([0.02, 0.025, 0.03, 0.035]) },
      { type: "fixed", amount_psf_yr: pick([0.5, 0.75, 1.0]) },
      { type: "cpi", rates: [0.03, 0.025] },
      undefined,
    ]);
    const recovery = pick([
      { type: "nnn" },
      { type: "nnn", cap_controllable_pct: 0.05 },
      { type: "base_year", year: Math.max(2015, sy) },
      { type: "expense_stop", stop_psf_yr: 10.5 },
      { type: "modified_gross", lines: ["taxes", "insurance"] },
      { type: "gross" },
    ]);
    leases.push({
      id: `L${String(i + 1).padStart(3, "0")}`,
      tenant: `Tenant ${i + 1}`,
      suite: String(100 + i),
      sf,
      start: iso(sy, sm, 1),
      expiry: iso(expY, expM, lastDay(expY, expM)),
      rent_psf_yr: 22 + Math.round(rnd() * 160) / 10,
      ...(steps ? { steps } : {}),
      free_rent_months: pick([0, 0, 0, 2, 4]),
      recovery,
      space_type: pick(["office", "office", "retail"]),
    });
  }
  const terms = (free, ti, lc) => ({ free_rent_months: free, ti_psf: ti, lc_pct: lc });
  return {
    property: { name: "Benchmark Tower", type: "office", rentable_sf: sfTotal + 25000, start_date: "2027-01-01", hold_months: 120 },
    rent_roll: leases,
    market: [
      { id: "office", market_rent_psf_yr: 33.5, rent_growth: 0.03, renewal_probability: 0.65, downtime_months: 6,
        term_months: 60, new: terms(6, 45, 0.06), renewal: terms(3, 20, 0.03) },
      { id: "retail", market_rent_psf_yr: 28, rent_growth: 0.025, renewal_probability: 0.7, downtime_months: 8,
        term_months: 36, new: terms(4, 35, 0.06), renewal: terms(1, 5, 0.03) },
    ],
    opex: {
      management_fee_pct: 0.03,
      lines: [
        { id: "taxes", name: "Real estate taxes", amount_yr: 2400000, growth: 0.03, recoverable: true },
        { id: "insurance", name: "Insurance", amount_yr: 380000, growth: 0.04, recoverable: true },
        { id: "cam", name: "Common area maintenance", amount_yr: 1500000, growth: 0.03, recoverable: true, controllable: true },
        { id: "admin", name: "Administration", amount_yr: 180000, growth: 0.03, recoverable: false },
      ],
    },
    other_income: [{ id: "parking", name: "Parking", amount_yr: 250000, growth: 0.02 }],
    revenue: { general_vacancy_pct: 0.05, credit_loss_pct: 0.01 },
    capital: { price: 120000000, closing_costs: 2400000, reserves_per_sf_yr: 0.25,
      capex: [{ id: "lobby", name: "Lobby", month: 12, amount: 1500000 }] },
    debt: { sizing: { max_ltv: 0.6, min_dscr: 1.3, min_debt_yield: 0.09 }, rate: { fixed: 0.06 }, io_months: 24,
      amortization_years: 30, origination_fee_pct: 0.01, prepayment_penalty_pct: 0.01 },
    exit: { cap_rate: 0.07, selling_costs_pct: 0.02 },
    equity: {
      contributions: { lp: 0.9, gp: 0.1 },
      tiers: [
        { name: "Return of capital", split: "pro_rata" },
        { name: "Preferred return", hurdle: { type: "irr", rate: 0.08, compounding: "annual" }, split: "pro_rata" },
        { name: "GP catch-up", split: { lp: 0, gp: 1 }, until: { gp_share_of_profits: 0.2 } },
        { name: "Promote 1", hurdle: { type: "irr", rate: 0.12 }, split: { lp: 0.8, gp: 0.2 } },
        { name: "Promote 2", split: { lp: 0.7, gp: 0.3 } },
      ],
    },
  };
}
