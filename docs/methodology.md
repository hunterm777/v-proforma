# proforma calculation method

This is the exact method the Rust core implements. Every rule here is tested.
Where the PRD left a choice open, the choice is marked **(convention)**.

## 1. Units and rounding

- Money in the deal file is in dollars. It is converted to integer cents on load
  (`round(dollars * 100)`, half away from zero).
- Every monthly cell of every source line (one lease, one expense line, one
  assumption) is computed in f64 and rounded **once** to integer cents, half away
  from zero. Every total is an exact integer sum of those cells, so lease-level
  rent always sums to property-level rent. **(convention)**
- Rates, growth, probabilities, IRR and multiples are f64. Powers and logs use the
  `libm` crate so native, WASM and Python builds give bit-identical results.
- Percent inputs are decimals: `0.05` means 5%.

## 2. Calendar

- `property.start_date` must be the first day of a month. Model month `m = 1`
  is that calendar month; month `m` covers `[start + (m-1) months, start + m months)`.
- The model runs `exit.month + 12` months, so forward NOI at exit is available.
- Analysis year of month `m` is `y(m) = floor((m - 1) / 12)` (0-based). The same
  formula is used for months before the start (`m <= 0`), giving negative years.
- Growth compounds once per analysis year: a value with growth `g` is
  `value * (1 + g)^y(m)` in month `m`. **(convention)**
- Partial months are prorated by actual days: a lease in effect for `d` of the
  `D` days of a month earns `d / D` of that month's rent. Lease dates are
  inclusive (`expiry` is the last day in effect).
- Cash flow timing: acquisition and loan funding at month 0 (the start date);
  each month's operating cash flow at the end of that month; sale at the end of
  the exit month. XIRR dates: month `t` is dated `start + t months`.

## 3. Lease cash flows (step 1)

For a contractual lease with area `sf`, start rent `rent_psf_yr` (the rent in
effect on the lease start date, lease year 1):

- **Steps** happen on each lease anniversary (`start + 12k months`, k = 1, 2, ...).
  Rent in lease year `k + 1`:
  - `percent {rate}`: `rent * (1 + rate)^k`
  - `fixed {amount_psf_yr}`: `rent + k * amount_psf_yr`
  - `cpi {rates: [r1, r2, ...]}`: `rent * (1 + r1) * ... * (1 + rk)`; the last rate repeats.
- **Monthly base rent** = sum over the days of the month the lease is in effect of
  `rate_psf_yr * sf / 12 / D` (D = days in that month), at the rate in effect on each day.
- **Free rent**: base rent on days in the first `free_rent_months` months of the
  lease (`[start, start + free months)`) is abated. Base rent is shown gross; the
  abatement is its own line (`free_rent`). Recoveries are not abated.
- **Security deposit** is carried for reporting only; it has no cash-flow effect.

### Rollover (step 1, continued)

When a lease expires inside the model, its space follows the lease's market
profile (`space_type`, with any per-lease `market` overrides). The space becomes
free on `expiry + 1 day`. From a free date `d`, with renewal probability `p`:

- **Renewal** (weight `p`): a lease starting on `d`, no downtime.
- **New tenant** (weight `1 - p`): `downtime_months` vacant, then a lease starting on
  `d + downtime months`.

Each speculative lease runs `term_months`, ending on `start + term months - 1 day`,
then the space is free again and the same rule applies. Probability mass is
propagated forward in date order, so the result is the exact expected value over
every renewal/new path; paths that reach the same start date and scenario are one
entry with their weights added. Speculative leases:

- rent = market rent on the start date, `market_rent_psf_yr * (1 + rent_growth)^y`,
  flat for the term (the PRD gives no steps for speculative leases) **(convention)**;
- free rent, TI and LC from the profile's `renewal` or `new` block;
- **TI** = `ti_psf * sf`, paid in the model month the lease starts;
- **LC** = `lc_pct * monthly rent * (term_months - free_rent_months)`, i.e. on the
  base rent over the term excluding free months, paid in the month the lease starts
  **(convention)**;
- the original lease's recovery structure and pro-rata share; a base-year
  structure uses the calendar year of the speculative lease's start.

Every contribution is weighted by its path probability before rounding.
**Turnover vacancy** (memo, not cash): for the new-tenant branch, the market rent the
space would have earned during downtime (`market_rent_psf_yr * (1 + rent_growth)^y(m)`
in each downtime month, prorated by days), weighted by its probability. Each downtime
path's monthly cell is rounded once to the cent, like any source cell.

## 4. Operating expenses and recoveries (steps 2 and 5)

- Expense line monthly amount: `amount_yr * (1 + growth)^y(m) / 12`.
- Recoverable pool `P(m)` = sum of lines with `recoverable: true`. Lines marked
  `controllable: true` form the controllable part `C(m)`.
- Management fee = `management_fee_pct * EGI` each month. It is not recoverable,
  because recoveries are computed before EGI (PRD step order).
- **Pro-rata share** `s` = `sf / property.rentable_sf` unless the lease gives
  `pro_rata_share`.
- **Controllable cap** (`cap_controllable_pct = c`): the controllable part used for a
  lease is `min(C(m), C(m0) * (1 + c)^k)` where `m0` is the model month of the lease
  start (projected backwards with the growth rule if before the start date) and `k`
  is whole lease years elapsed at month `m`. Non-cumulative, compounding. **(convention)**
- Recoveries are computed from the unrounded pool (each expense line's formula
  value), then rounded once per lease and month.
- Monthly recovery, times the lease's occupied fraction of the month and its weight
  (`P*` is the pool with the cap applied):
  - `nnn`: `s * P*(m)`
  - `base_year {year}`: `s * max(0, P*(m) - B / 12)`, `B` = uncapped pool summed over
    calendar year `year` (projected with the growth rule)
  - `expense_stop {stop_psf_yr}`: `max(0, s * P*(m) - stop_psf_yr * sf / 12)`
  - `modified_gross {lines}`: `s * ` the capped pool of only the listed lines
    (tenant pays its share of those lines in full) **(convention)**
  - `gross`: 0

## 5. Revenue (steps 3 and 4)

- `PGR = base rent + recoveries + other income` (other income lines grow like expenses).
- General vacancy = `round(max(0, general_vacancy_pct * PGR - turnover vacancy))`, with
  turnover vacancy the month's sum of its rounded cells, so
  modeled downtime is not counted twice. With `revenue.vacancy_on_top_of_downtime:
  true`, it is `general_vacancy_pct * PGR`.
- Credit loss = `credit_loss_pct * PGR`.
- `EGI = PGR - general vacancy - credit loss - free rent`.
- `NOI = EGI - operating expenses - management fee`.
- `Cash flow before debt = NOI - TI - LC - capital reserves - capex`.
- Capital reserves = `reserves_per_sf_yr * rentable_sf / 12` (or per unit), flat.
- Unleased space at the start earns nothing and its share of expenses is not recovered.

## 6. Multifamily

A unit mix replaces the rent roll. Per unit type, with market rent `M(m) =
market_rent_mo * (1 + rent_growth)^y(m)` and loss-to-lease `G0 = market_rent_mo -
in_place_rent_mo`:

- loss-to-lease burns off linearly: `G(m) = G0 * max(0, 1 - (m - 1) / burn_off_months)`
  (0 if `burn_off_months` is 0) **(convention)**;
- base rent = `units * (M(m) - G(m))`;
- concessions (the free rent line) = `units * turnover_rate / 12 * concession_months * M(m)`:
  each month's turning units get `concession_months` free at market **(convention)**;
- no recoveries, TI, LC or downtime; general vacancy and credit loss apply to PGR.

## 7. Debt (step 7)

- **Sizing.** Either a fixed `amount`, or the lowest of the active constraints,
  using year-1 NOI (months 1 to 12):
  - LTV: `max_ltv * price`
  - Debt yield: `NOI1 / min_debt_yield`
  - DSCR: `NOI1 / (min_dscr * 12 * k)`, where `k` is the monthly payment per dollar
    of loan at the month-1 rate: the amortizing constant if `amortization_years` is
    set (even with an IO period), otherwise `rate / 12` **(convention)**.
  The loan is rounded down to the cent; the binding constraint is reported.
- **Rate.** Fixed, or floating `index + spread` with a forward curve of
  `{month, index}` points (step function; first point at month 1).
- **Schedule.** Interest = `round(balance * rate / 12)` (30/360). IO months pay
  interest only. Then the payment is the standard annuity
  `L * r / (1 - (1 + r)^-n)` with `r = rate / 12`, `n` = amortization months,
  rounded to the cent: computed once for a fixed rate, recomputed each month on the
  remaining balance and term for a floating rate. Principal = payment - interest.
- **Fees.** `origination_fee_pct * loan`, paid at close.
- **Exit.** Payoff = balance after the exit month's payment. Prepayment penalty =
  `prepayment_penalty_pct * payoff`; a list gives the rate by loan year (last repeats).
- Sources = uses: `equity + loan = price + closing costs + loan fees`.

## 8. Exit (step 8)

- Gross sale = forward NOI (months `X + 1` to `X + 12`) / `cap_rate`.
- Net sale = gross sale - `selling_costs_pct * gross sale`.

## 9. Returns (step 9)

- Unlevered: month 0 `-(price + closing costs)`; months 1 to X cash flow before debt;
  plus net sale at X.
- Levered: month 0 `-equity`; months 1 to X cash flow before debt minus debt service;
  plus net sale minus payoff minus prepayment penalty at X.
- IRR: monthly rate `r` with NPV = 0, annualised `(1 + r)^12 - 1`. XIRR on the
  dates above with an actual/365 year. No sign change means no IRR.
- Equity multiple = total positive cash flow / total negative cash flow.
- Cash-on-cash by year = that year's operating cash flow (no sale) / equity at
  close (levered) or total cost (unlevered).
- Peak equity = largest cumulative net investment. Payback month = first month the
  cumulative levered cash flow is at least 0.

## 10. Waterfall (step 10)

American timing: every levered cash flow runs through the tiers in order.

- **Contributions** (negative levered cash flow) are split by `contributions`.
- **Hurdles are measured on the LP's cash flows.** **(convention)**
  - `irr {rate, compounding}`: an account per tier: LP contributions plus interest,
    less LP distributions. Each month interest `rate / 12` accrues on capital plus
    capitalized interest (`none`: on capital only, never capitalized); it is
    capitalized every 1, 3 or 12 months for `monthly`, `quarterly`, `annual`.
    Distributions pay accrued interest, then capitalized interest, then capital.
  - `multiple {multiple}`: until LP distributions reach `multiple * LP contributions`.
  - A tier with neither `hurdle` nor `until` returns capital (a 1.0x multiple),
    except the last tier, which takes everything left. The last tier must be that
    residual tier. **(convention)**
- `until {gp_share_of_profits: t}`: a catch-up runs until GP profit / total profit
  = `t`, profit = distributions - contributions.
- A tier receives `ceil(needed / lp_split)` cents (or the catch-up amount), capped
  by what is left of the month's distribution; the LP gets `round(x * lp_split)`
  and the GP the rest, so LP + GP = the distribution exactly.
- Promote = GP distributions - GP contribution share * total distributions.

## 11. Audit trail

Every monthly cell is an entry with its source (lease, unit type, expense line,
income line, capex item or assumption), the PRD step that produced it, and a
detail (contract, or the rollover scenario with its start date and weight).
Every reported total is the sum of its entries; `trace` lists them, signed as in
the reports (deductions negative). Below cash flow before debt, `trace` lists the
deal inputs behind each number: interest and principal for debt service; price and
closing costs for the purchase; loan proceeds (with the binding constraint) and the
origination fee for the loan; gross sale and selling costs for the sale; payoff and
prepayment penalty at exit; and every operating entry plus these for the cash flow
series. Year 0 and month 0 are the close.
