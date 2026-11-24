# Reference model (M1 to M4)

`reference.py` is a second, independent implementation of `docs/methodology.md`
in plain Python (standard library plus PyYAML). It was written from the method
document and the fixture files alone, without reading the engine's code, so the
two can be compared cell by cell.

`python3 tools/handcheck.py` (from the repo root) runs it on every fixture in
`fixtures/deals/` and compares it with the engine's golden files: every monthly
line to the exit month, each lease's rent, free rent, recoveries, TI and LC, the
exit, both cash flow series and IRRs, the loan schedule, and the waterfall by
tier and partner. CI runs the same script.
