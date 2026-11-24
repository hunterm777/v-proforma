// Rent roll summary tab: WALT, occupancy and expirations by year, the unit
// mix for multifamily, and the per-lease tenant detail.
import { h, selectField } from "../../dom";
import { num, pct } from "../../format";
import type { Result } from "../../types";
import { traceYear } from "../audit";
import { facts, money, section, viewTable } from "./view";

const LINES = [
  ["base_rent", "Base rent"],
  ["free_rent", "Free rent"],
  ["recoveries", "Expense recoveries"],
  ["ti", "Tenant improvements"],
  ["lc", "Leasing commissions"],
] as const;
type LineKey = (typeof LINES)[number][0];
let line: LineKey = "base_rent";

function tenants(r: Result): HTMLElement {
  const years = r.annual.columns.length - 1;
  const head = ["Lease", "Tenant", "Suite", "SF", "Start", "Expiry", "In-place $/SF/yr", "Deposit $", ...r.annual.columns.slice(1)];
  const rows = r.tenants.map((t) => [
    t.lease_id,
    t.tenant,
    t.suite,
    num(t.sf, 0),
    t.start,
    t.expiry,
    num(t.in_place_rent_psf_yr, 2),
    num(t.security_deposit, 0),
    ...Array.from({ length: years }, (_, i) => money(t.years[line]?.[i] ?? 0, traceYear(line, i + 1, t.lease_id))),
  ]);
  const wrap = h("div");
  const pick = selectField<LineKey>("Line by year", line, LINES, (v) => {
    line = v;
    wrap.replaceWith(tenants(r));
  });
  wrap.append(
    h("div", { class: "controls" }, pick),
    h("p", { class: "note" }, "Each lease's own entries and its rollover leases, as recorded. Click a year to see it in the property's trail."),
    viewTable(head, rows, "tenants"),
  );
  return wrap;
}

export function rentRollView(r: Result): HTMLElement {
  const rr = r.rent_roll;
  const out = h("div", { class: "stack" });
  if (r.tenants.length) {
    out.append(
      section(
        "Rent roll summary",
        facts([
          ["WALT", rr.walt_years === null ? "–" : `${num(rr.walt_years, 2)} years`],
          ["Leases", String(r.tenants.length)],
        ]),
      ),
      section(
        "Occupancy by year",
        h("p", { class: "note" }, "Average expected occupied SF over rentable SF, after modeled rollover and downtime."),
        viewTable(["Year", "Occupancy"], rr.occupancy_by_year.map((o, i) => [`Year ${i + 1}`, pct(o, 1)])),
      ),
      section(
        "Expirations by year",
        viewTable(
          ["Year", "SF", "% of SF", "% of rent", "Leases"],
          rr.expirations.map((e) => [e.year === null ? "After hold" : `Year ${e.year}`, num(e.sf, 0), pct(e.pct_sf, 1), pct(e.pct_rent, 1), e.leases.join(", ")]),
        ),
        h("p", { class: "note" }, "% of rent is the share of in-place annual base rent at the analysis start."),
      ),
      section("Tenant detail", tenants(r)),
    );
  }
  if (rr.unit_mix.length) {
    out.append(
      section(
        "Unit mix",
        viewTable(
          ["Unit type", "Units", "In-place $/mo", "Market $/mo", "Loss to lease"],
          rr.unit_mix.map((u) => [u.id, String(u.units), num(u.in_place_rent_mo, 2), num(u.market_rent_mo, 2), pct(u.loss_to_lease_pct, 1)]),
        ),
      ),
    );
    if (rr.occupancy_by_year.length) {
      out.append(section("Occupancy by year", viewTable(["Year", "Occupancy"], rr.occupancy_by_year.map((o, i) => [`Year ${i + 1}`, pct(o, 1)]))));
    }
  }
  return out;
}
