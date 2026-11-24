# Spreadsheet hand-check (M1 and M3)

Two workbooks built cell by cell with formulas, written from `docs/methodology.md`
and the fixture YAML alone, without the engine's code:

- `m1_nnn_industrial.xlsx`: the M1 10-year NNN industrial deal (rent with steps
  and day proration, every expense line, NNN recoveries, vacancy, management fee,
  NOI, reserves, forward NOI, sale, unlevered cash flows, `IRR()` and multiple).
- `m3_fixed_sized.xlsx`: the same property plus loan sizing (LTV, DSCR, debt
  yield; lowest wins, rounded down) and the payment schedule built the standard
  mortgage-calculator way (`ROUND(PMT(rate/12, n, -loan), 2)` after the IO period).
  It also checks the textbook case: $1,000,000 at 6% over 30 years = $5,995.51.

The `.json` files are the values LibreOffice calculated, in integer cents.
`tools/handcheck.py` compares them with the engine's golden files.

Rebuild:

```
python3 build.py ../deals/m1_nnn_industrial.yaml m1_nnn_industrial.xlsx
soffice --headless --convert-to xlsx --outdir recalc m1_nnn_industrial.xlsx
python3 export.py recalc/m1_nnn_industrial.xlsx m1_nnn_industrial.json
```

(`--debt` on `export.py` for the M3 workbook.)
