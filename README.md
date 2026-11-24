# proforma

**An auditable CRE pro forma you can version, diff and run anywhere.**

**[Open proforma](https://hunterm777.github.io/v-proforma/)** · runs entirely in your browser; deal files never leave your machine.

<!-- demo GIF: docs/demo.gif -->

proforma is an open-source commercial real estate underwriting engine. A rent
roll and deal assumptions go in; lease-by-lease cash flows, NOI, debt service,
unlevered and levered returns, and the LP/GP equity waterfall come out. A deal
is one readable YAML or JSON file you can commit, diff and share, and every
number in every report traces back to the lease or assumption that produced it.

It covers office, retail, industrial and multifamily. All math lives in one Rust
core, compiled to WebAssembly for the page and to a Python package, and the three
builds give the same output to the cent.

> Not investment advice. Every assumption is your input; proforma has no market data and scores no deals.

## What it models

| Area | Model |
| --- | --- |
| Leases | Base rent by $/SF/yr with fixed $, % or CPI-style steps on each anniversary, free rent, day-prorated partial months, security deposits. |
| Rollover | At each expiry, an exact probability-weighted blend of renewal and new-tenant paths (downtime, market rent at that date, free rent, TI, leasing commissions), repeated for every later expiry inside the model. |
| Recoveries | NNN, base year, expense stop, modified gross, gross; pro-rata share; caps on controllable expenses. |
| Revenue | PGR, general vacancy and credit loss after modeled downtime (or on top of it, as a setting), EGI, management fee on EGI, NOI. |
| Multifamily | Unit-type rent roll: in-place and market rent, loss-to-lease burn-off, turnover, concessions. |
| Capital | Price, closing costs, reserves per SF or per unit, one-off capex by month. |
| Debt | A loan amount, or sizing by max LTV, min DSCR and min debt yield (lowest wins, binding constraint reported); fixed or floating rate with a forward curve; IO then amortization; fees; prepayment penalty at exit. |
| Exit and returns | Forward 12-month NOI over the exit cap, selling costs, payoff; unlevered and levered IRR, XIRR, equity multiple, cash-on-cash by year, peak equity, payback month. |
| Waterfall | Tiers as data: IRR or multiple hurdles, monthly, quarterly, annual or simple compounding, full or partial GP catch-up, promotes; American (deal-by-deal) timing. |
| Sensitivity | Two-way levered IRR and equity multiple tables across any two inputs. |

The exact method, including every convention where the spec left a choice open,
is in [docs/methodology.md](docs/methodology.md).

## A deal file

```yaml
property:
  name: Ridgeline Logistics Center
  type: industrial
  rentable_sf: 412000
  start_date: 2027-01-01
  hold_months: 120
rent_roll:
  - id: A1
    tenant: Coastal Freight
    suite: "A"
    sf: 210000
    start: 2024-04-01
    expiry: 2031-03-31
    rent_psf_yr: 8.40
    steps: { type: percent, rate: 0.035 }
    recovery: { type: nnn }
    space_type: industrial
# market, opex, revenue, capital, debt, exit, equity ...
```

Four complete examples are in [deals/](deals/) (industrial, office, retail,
multifamily; made-up numbers), and the JSON Schema is
[schema/deal.schema.json](schema/deal.schema.json). Every field carries its unit
in its name or schema, and market assumptions can be overridden per lease.

## Python

```python
import proforma as pf

deal = pf.load("deals/industrial_example.yaml")
result = pf.run(deal)

result.returns()            # unlevered/levered IRR, multiple, cash-on-cash
result.cash_flow("annual")  # DataFrame-compatible table
result.trace("noi", year=3) # which leases and assumptions produced it
pf.sensitivity(deal, x="exit.cap_rate", y="market.rent_growth",
               x_values=[0.055, 0.06, 0.065], y_values=[0.02, 0.03, 0.04])
```

The distribution name is `v-proforma` (the name `proforma` is taken on PyPI);
the import name is `proforma`. It is not on PyPI yet: build it from source with
`pip install maturin && maturin develop -m crates/python/Cargo.toml`, or install
a wheel from a GitHub release.

## How it is checked

- **Two independent hand-checks.** The M1 industrial deal and the M3 loan are
  rebuilt as formula spreadsheets recalculated by LibreOffice, and M1 to M4 are
  rebuilt by a separate Python model written from the method document alone
  ([fixtures/handcheck/](fixtures/handcheck/)). `python3 tools/handcheck.py`
  compares about 20,000 values with the engine; all match to the cent.
- **Invariants on every run:** lease-level rent sums to property rent every
  month; sources equal uses at close; waterfall distributions sum to levered
  cash flow and every tier pays LP and GP by its split; every IRR reprices its series to an
  NPV of zero (relative to the series' size, within 1e-9).
- **Golden files:** the deal file plus the engine version reproduce every number
  exactly ([fixtures/expected/](fixtures/expected/)), and the WASM and Python
  builds must match them byte for byte.

## Build from source

```
cargo test --workspace --exclude proforma-python   # core, golden files, invariants
python3 tools/handcheck.py                         # hand-checks (needs PyYAML)
cd web && npm ci && npm run wasm && npm run dev    # the page
```

See [web/README.md](web/README.md) for the page and
[crates/python/README.md](crates/python/README.md) for Python.

## Layout

```
schema/deal.schema.json   deal file schema (generated from the core's types)
crates/core               the engine: every calculation
crates/wasm               wasm-bindgen wrapper for the page
crates/python             PyO3 wrapper and the proforma package
web/                      the dark-mode page
deals/                    example deals
fixtures/                 milestone fixtures, golden outputs, hand-checks
docs/methodology.md       the calculation method
```

## License

MIT
