"""Compares the engine's golden files with the two independent hand-checks.

- fixtures/handcheck/spreadsheet/: formula workbooks for M1 and M3, recalculated
  by LibreOffice (values committed as JSON).
- fixtures/handcheck/reference/reference.py: a separate Python model of
  docs/methodology.md, written without the engine's code; run here on every
  fixture in fixtures/deals/.

Every monthly cell to the exit month, every cash flow, the debt schedule and the
waterfall must match to the cent; IRRs must agree to 1e-9.
Run from the repo root: python3 tools/handcheck.py   (needs PyYAML)
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "fixtures/handcheck/reference"))
import reference  # noqa: E402

LINES = ["base_rent", "recoveries", "other_income", "pgr", "general_vacancy", "credit_loss", "free_rent", "egi",
         "opex", "management_fee", "noi", "ti", "lc", "reserves", "capex", "cash_flow_before_debt"]
BY_LEASE = ["base_rent", "free_rent", "recoveries", "ti", "lc"]


class Check:
    def __init__(self) -> None:
        self.cells = 0
        self.failures: list[str] = []

    def eq(self, what: str, a: object, b: object) -> None:
        self.cells += 1
        if a != b:
            self.failures.append(f"{what}: engine {a} vs hand-check {b}")

    def seq(self, what: str, a: list[int], b: list[int]) -> None:
        if len(a) != len(b):
            self.failures.append(f"{what}: length {len(a)} vs {len(b)}")
        for i, (x, y) in enumerate(zip(a, b)):
            self.eq(f"{what}[{i}]", x, y)

    def close(self, what: str, a: float, b: float, tol: float = 1e-9) -> None:
        self.cells += 1
        if abs(a - b) > tol:
            self.failures.append(f"{what}: engine {a} vs hand-check {b}")


def monthly(e: dict) -> dict[str, list[int]]:
    return {r["key"]: [abs(v) for v in r["values"]] for r in e["monthly"]["rows"]}


def by_lease(e: dict, n: int) -> dict[tuple[str, str], list[int]]:
    out: dict[tuple[str, str], list[int]] = {}
    for en in e["entries"]:
        if en["source"]["kind"] != "lease":
            continue
        a = out.setdefault((en["source"]["id"], en["line"]), [0] * n)
        for i, v in enumerate(en["values"]):
            a[en["from_month"] - 1 + i] += v
    return out


def against_reference(c: Check, name: str, e: dict, r: dict) -> None:
    x = e["exit_month"]
    m = monthly(e)
    for k in LINES:
        c.seq(f"{name} {k}", m[k][:x], [abs(v) for v in r["lines"][k][:x]])
    bl = by_lease(e, e["model_months"])
    for lid, lines in r["by_lease"].items():
        for k in BY_LEASE:
            c.seq(f"{name} {lid} {k}", bl.get((lid, k), [0] * e["model_months"])[:x], lines[k][:x])
    for k in ["forward_noi", "gross_sale", "selling_costs", "net_sale"]:
        c.eq(f"{name} exit {k}", e["exit"][k], r["exit"][k])
    for side in ["unlevered", "levered"]:
        c.seq(f"{name} {side} cash flow", e["returns"][side]["cash_flows"], r[side]["cash_flows"])
        c.close(f"{name} {side} irr", e["returns"][side]["irr"], r[side]["irr"])
        c.close(f"{name} {side} multiple", e["returns"][side]["equity_multiple"], r[side]["equity_multiple"])
    if e.get("debt"):
        d, rd = e["debt"], r["debt"]
        for k in ["loan", "fee", "payoff", "prepayment"]:
            c.eq(f"{name} debt {k}", d[k], rd[k])
        c.eq(f"{name} binding constraint", d["sizing"]["binding"], rd["binding"])
        for row, rr in zip(d["schedule"], rd["schedule"]):
            for k in ["opening", "interest", "principal", "payment", "closing"]:
                c.eq(f"{name} debt month {row['month']} {k}", row[k], rr[k])
    if e.get("waterfall"):
        w, rw = e["waterfall"], r["waterfall"]
        c.seq(f"{name} LP", w["lp"]["cash_flows"], rw["lp_cash_flows"])
        c.seq(f"{name} GP", w["gp"]["cash_flows"], rw["gp_cash_flows"])
        for t, rt in zip(w["tiers"], rw["tiers"]):
            c.seq(f"{name} tier {t['name']} LP", t["lp"], rt["lp"][: len(t["lp"])])
            c.seq(f"{name} tier {t['name']} GP", t["gp"], rt["gp"][: len(t["gp"])])
        c.eq(f"{name} promote", w["gp_promote"], rw["gp_promote"])
        c.close(f"{name} LP irr", w["lp"]["irr"], rw["lp_irr"])
        c.close(f"{name} GP irr", w["gp"]["irr"], rw["gp_irr"])


def against_sheet(c: Check, name: str, e: dict, s: dict) -> None:
    x = e["exit_month"]
    m = monthly(e)
    for k in ["base_rent", "recoveries", "opex", "management_fee", "general_vacancy", "egi", "noi", "reserves",
              "cash_flow_before_debt"]:
        c.seq(f"sheet {name} {k}", m[k][:x], [abs(v) for v in s[k][:x]])
    for k in ["forward_noi", "gross_sale", "selling_costs", "net_sale"]:
        c.eq(f"sheet {name} exit {k}", e["exit"][k], s[k])
    c.seq(f"sheet {name} unlevered cash flow", e["returns"]["unlevered"]["cash_flows"], s["unlevered_cf"])
    c.close(f"sheet {name} unlevered irr", e["returns"]["unlevered"]["irr"], s["unlevered_irr"])
    if "schedule" in s:
        d = e["debt"]
        c.eq(f"sheet {name} loan", d["loan"], s["loan"])
        c.eq(f"sheet {name} binding constraint", d["sizing"]["binding"], s["binding_constraint"])
        c.eq(f"sheet {name} payoff", d["payoff"], s["payoff"])
        for row, rr in zip(d["schedule"], s["schedule"]):
            for k in ["opening", "interest", "principal", "payment", "closing"]:
                c.eq(f"sheet {name} debt month {row['month']} {k}", row[k], rr[k])


def main() -> int:
    c = Check()
    with tempfile.TemporaryDirectory() as tmp:
        for deal in sorted((ROOT / "fixtures/deals").glob("*.yaml")):
            out = Path(tmp) / f"{deal.stem}.json"
            reference.run_file(str(deal), str(out))
            e = json.loads((ROOT / "fixtures/expected" / f"{deal.stem}.json").read_text())
            before = c.cells
            against_reference(c, deal.stem, e, json.loads(out.read_text()))
            print(f"reference model  {deal.stem:22} {c.cells - before:6} values compared")
    for sheet in sorted((ROOT / "fixtures/handcheck/spreadsheet").glob("*.json")):
        e = json.loads((ROOT / "fixtures/expected" / sheet.name).read_text())
        before = c.cells
        against_sheet(c, sheet.stem, e, json.loads(sheet.read_text()))
        print(f"spreadsheet      {sheet.stem:22} {c.cells - before:6} values compared")
    if c.failures:
        print(f"\n{len(c.failures)} mismatches of {c.cells}:")
        for f in c.failures[:40]:
            print("  " + f)
        return 1
    print(f"\nall {c.cells} values match")
    return 0


if __name__ == "__main__":
    sys.exit(main())
