// Debt tab: sizing with every constraint and the one that bound, DSCR and
// debt yield by year, and the payment schedule.
import { h } from "../../dom";
import { mult, pct, usd } from "../../format";
import type { Result } from "../../types";
import { traceYear } from "../audit";
import { facts, money, section, viewTable } from "./view";

const NAMES: Record<string, string> = { ltv: "Max LTV", dscr: "Min DSCR", debt_yield: "Min debt yield" };

export function debtView(r: Result): HTMLElement {
  const d = r.debt;
  if (!d) return section("Debt", h("p", { class: "note" }, "This deal has no loan."));
  const out = h("div", { class: "stack" });
  out.append(
    section(
      "Loan",
      facts([
        ["Loan amount", usd(d.loan)],
        ["Origination fee", usd(d.fee)],
        ["Payoff at exit", usd(d.payoff)],
        ["Prepayment penalty", `${pct(d.prepayment_pct)} · ${usd(d.prepayment)}`],
      ]),
    ),
  );
  if (d.sizing) {
    const s = d.sizing;
    out.append(
      section(
        "Sizing",
        facts([
          ["Year-1 NOI", usd(s.noi_year1)],
          ["Sizing rate (month 1)", pct(s.rate, 3)],
          ["Binding constraint", NAMES[s.binding] ?? s.binding],
        ]),
        viewTable(
          ["Constraint", "Loan it allows", ""],
          s.candidates.map((c) => [NAMES[c.constraint] ?? c.constraint, money(c.amount), c.constraint === s.binding ? h("span", { class: "badge" }, "binds") : ""]),
        ),
      ),
    );
  } else {
    out.append(section("Sizing", h("p", { class: "note" }, "The loan is a fixed amount in the deal file.")));
  }
  out.append(
    section(
      "Coverage by year",
      viewTable(
        ["Year", "Debt service", "DSCR", "Debt yield"],
        d.by_year.map((y) => [`Year ${y.year}`, money(y.debt_service, traceYear("debt_service", y.year)), mult(y.dscr), pct(y.debt_yield)]),
      ),
      h("p", { class: "note" }, "DSCR is the year's NOI over its debt service; debt yield is the year's NOI over the balance at the start of the year."),
    ),
    section(
      "Payment schedule",
      viewTable(
        ["Month", "Date", "Rate", "Opening", "Interest", "Principal", "Payment", "Closing"],
        d.schedule.map((p) => [String(p.month), r.monthly.columns[p.month - 1] ?? "", pct(p.rate, 3), money(p.opening), money(p.interest), money(p.principal), money(p.payment), money(p.closing)]),
        "schedule",
      ),
    ),
  );
  return out;
}
