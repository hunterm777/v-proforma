// Capital tab: price, closing costs, reserves and capital expenditures.
import { h, numField } from "../../dom";
import type { CapexItem, Deal } from "../../types";
import { copyLast, numCol, put, textCol } from "../cols";
import { editTable } from "../table";
import { fields } from "./property";

export function capitalTab(deal: Deal, changed: () => void): HTMLElement {
  const c = deal.capital;
  const mf = deal.property.type === "multifamily";
  const rows = (c.capex ??= []);
  const blank: CapexItem = { id: "c1", month: 1, amount: 0 };
  return h(
    "section",
    { class: "tab-body" },
    fields(
      numField({ label: "Purchase price", value: c.price, suffix: "$", required: true, onValue: (v) => ((c.price = v ?? c.price), changed()) }),
      numField({ label: "Closing costs", value: c.closing_costs, suffix: "$", onValue: (v) => (put(c, "closing_costs", v), changed()) }),
      mf
        ? numField({ label: "Reserves", value: c.reserves_per_unit_yr, suffix: "$/unit/yr", onValue: (v) => (put(c, "reserves_per_unit_yr", v), changed()) })
        : numField({ label: "Reserves", value: c.reserves_per_sf_yr, suffix: "$/SF/yr", onValue: (v) => (put(c, "reserves_per_sf_yr", v), changed()) }),
    ),
    h("h3", {}, "Capital expenditures"),
    editTable<CapexItem>({
      rows: () => rows,
      make: (rs) => copyLast(rs, blank, "c"),
      errorKey: (_r, i) => `capital.capex[${i}]`,
      addLabel: "Add item",
      onChange: changed,
      cols: [
        textCol("ID", (r) => r.id, (r, v) => (r.id = v ?? r.id), { required: true }),
        textCol("Name", (r) => r.name, (r, v) => put(r, "name", v), { wide: true }),
        numCol("Month", (r) => r.month, (r, v) => (r.month = v ?? r.month), { integer: true, required: true, title: "Model month; 1 is the first month" }),
        numCol("Amount $", (r) => r.amount, (r, v) => (r.amount = v ?? r.amount), { required: true }),
      ],
    }),
  );
}
