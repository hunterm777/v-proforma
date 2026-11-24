# proforma (Python)

Python bindings for the proforma Rust core. All math runs in the core; this
package only parses its JSON output into Python objects.

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

Money is returned in dollars (cents / 100) unless you ask for `unit="cents"`,
which gives the exact integer cents the core computes.

Build from source: `pip install maturin && maturin develop -m crates/python/Cargo.toml`.
