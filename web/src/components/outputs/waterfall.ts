// Waterfall tab: LP and GP returns, the GP promote, distributions by tier
// and partner over the hold, and the same in every month that has one.
import { h } from "../../dom";
import { mult, pct, usd } from "../../format";
import type { Result } from "../../types";
import { facts, money, section, viewTable } from "./view";

export function waterfallView(r: Result): HTMLElement {
  const w = r.waterfall;
  if (!w) return section("Waterfall", h("p", { class: "note" }, "This deal has no equity waterfall."));
  const partners = [
    ["LP", w.lp],
    ["GP", w.gp],
  ] as const;
  const months = r.returns.levered.cash_flows.map((_, m) => m).filter((m) => w.tiers.some((t) => (t.lp[m] ?? 0) !== 0 || (t.gp[m] ?? 0) !== 0));
  const when = (m: number): string => (m === 0 ? `${r.start_date} (close)` : (r.monthly.columns[m - 1] ?? ""));
  return h(
    "div",
    { class: "stack" },
    section(
      "Partners",
      viewTable(
        ["Partner", "Contributed", "Distributed", "IRR", "XIRR", "Multiple"],
        partners.map(([name, p]) => [name, money(p.contributed), money(p.distributed), pct(p.irr), pct(p.xirr), mult(p.equity_multiple)]),
      ),
      facts([["GP promote", usd(w.gp_promote)]]),
      h("p", { class: "note" }, "The promote is the GP's distributions less its contribution share of all distributions."),
    ),
    section(
      "Distributions by tier and partner",
      viewTable(
        ["Tier", "LP", "GP"],
        [
          ...w.tiers.map((t) => [t.name, money(t.lp_total), money(t.gp_total)]),
          [h("strong", {}, "All tiers"), money(w.lp.distributed), money(w.gp.distributed)],
        ],
      ),
      h("p", { class: "note" }, "Dollars over the hold. All tiers is each partner's total distributions."),
    ),
    section(
      "Distributions by month",
      h("p", { class: "note" }, "Dollars by tier and partner; months with no distribution are left out."),
      viewTable(
        ["Month", "Date", ...w.tiers.flatMap((t) => [`${t.name} LP`, `${t.name} GP`])],
        months.map((m) => [String(m), when(m), ...w.tiers.flatMap((t) => [money(t.lp[m] ?? 0), money(t.gp[m] ?? 0)])]),
        "waterfall",
      ),
    ),
  );
}
