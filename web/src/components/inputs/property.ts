// Property tab, Leasing tab (market profiles) and Exit tab.
import { dateField, h, numField, selectField, textField } from "../../dom";
import { TYPES } from "../../format";
import type { Deal, MarketProfile, PropertyType } from "../../types";
import { copyLast, numCol, put, textCol } from "../cols";
import { editTable } from "../table";

export const fields = (...f: HTMLElement[]): HTMLElement => h("div", { class: "fields" }, ...f);

export function propertyTab(deal: Deal, changed: (structural?: boolean) => void): HTMLElement {
  const p = deal.property;
  const mf = p.type === "multifamily";
  return h(
    "section",
    { class: "tab-body" },
    fields(
      textField("Property name", p.name, (v) => ((p.name = v), changed())),
      selectField<PropertyType>("Type", p.type, TYPES, (v) => ((p.type = v), changed(true))),
      mf
        ? numField({ label: "Units", value: p.units, integer: true, placeholder: "unit mix total", onValue: (v) => (put(p, "units", v), changed()) })
        : numField({ label: "Rentable SF", value: p.rentable_sf, suffix: "SF", onValue: (v) => (put(p, "rentable_sf", v), changed()) }),
      dateField("Analysis start", p.start_date, (v) => ((p.start_date = v), changed())),
      numField({ label: "Hold", value: p.hold_months, integer: true, required: true, suffix: "months", onValue: (v) => ((p.hold_months = v ?? p.hold_months), changed()) }),
    ),
  );
}

export function leasingTab(deal: Deal, changed: () => void): HTMLElement {
  if (deal.property.type === "multifamily") {
    return h("section", { class: "tab-body" }, h("p", { class: "note" }, "Multifamily leasing comes from the unit mix (Rent roll tab)."));
  }
  const rows = (deal.market ??= []);
  const blank: MarketProfile = {
    id: "m1",
    market_rent_psf_yr: 0,
    rent_growth: 0,
    renewal_probability: 0,
    downtime_months: 0,
    term_months: 60,
    new: {},
    renewal: {},
  };
  const req = { required: true };
  const terms = (side: "new" | "renewal", tag: string) => [
    numCol<MarketProfile>(`${tag} free mo`, (r) => r[side].free_rent_months, (r, v) => put(r[side], "free_rent_months", v), { integer: true }),
    numCol<MarketProfile>(`${tag} TI $/SF`, (r) => r[side].ti_psf, (r, v) => put(r[side], "ti_psf", v)),
    numCol<MarketProfile>(`${tag} LC %`, (r) => r[side].lc_pct, (r, v) => put(r[side], "lc_pct", v), { scale: 100 }),
  ];
  return h(
    "section",
    { class: "tab-body" },
    h("h3", {}, "Market leasing profiles"),
    editTable<MarketProfile>({
      rows: () => rows,
      make: (rs) => copyLast(rs, blank, "m"),
      errorKey: (r, i) => `market[${i}] (${r.id})`,
      addLabel: "Add profile",
      onChange: changed,
      cols: [
        textCol("Space type", (r) => r.id, (r, v) => (r.id = v ?? r.id), req),
        numCol("Market $/SF/yr", (r) => r.market_rent_psf_yr, (r, v) => (r.market_rent_psf_yr = v ?? r.market_rent_psf_yr), req),
        numCol("Growth %", (r) => r.rent_growth, (r, v) => (r.rent_growth = v ?? r.rent_growth), { scale: 100, required: true }),
        numCol("Renew %", (r) => r.renewal_probability, (r, v) => (r.renewal_probability = v ?? r.renewal_probability), { scale: 100, required: true }),
        numCol("Downtime mo", (r) => r.downtime_months, (r, v) => (r.downtime_months = v ?? r.downtime_months), { integer: true, required: true }),
        numCol("Term mo", (r) => r.term_months, (r, v) => (r.term_months = v ?? r.term_months), { integer: true, required: true }),
        ...terms("new", "New"),
        ...terms("renewal", "Renewal"),
      ],
    }),
    h("p", { class: "note" }, "Leases name a profile in their Space type column. Paste rows from a spreadsheet into any cell."),
  );
}

export function exitTab(deal: Deal, changed: () => void): HTMLElement {
  const x = deal.exit;
  return h(
    "section",
    { class: "tab-body" },
    fields(
      numField({ label: "Sale month", value: x.month, integer: true, placeholder: "end of hold", onValue: (v) => (put(x, "month", v), changed()) }),
      numField({ label: "Exit cap rate", value: x.cap_rate, scale: 100, suffix: "%", required: true, onValue: (v) => ((x.cap_rate = v ?? x.cap_rate), changed()) }),
      numField({ label: "Selling costs", value: x.selling_costs_pct, scale: 100, suffix: "%", onValue: (v) => (put(x, "selling_costs_pct", v), changed()) }),
    ),
    h("p", { class: "note" }, "The exit cap rate applies to the 12 months of NOI after the sale month."),
  );
}
