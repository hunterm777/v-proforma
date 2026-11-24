// Expenses tab: opex lines, management fee, other income, vacancy and credit
// loss, and the named recovery structures leases can refer to.
import { checkField, h, numField } from "../../dom";
import type { Deal, ExpenseLine, IncomeLine, Recovery, RecoveryType } from "../../types";
import { boolCol, copyLast, numCol, put, selectCol, textCol } from "../cols";
import { editTable } from "../table";
import { capCol, newRecovery, paramCol, RECOVERY_TYPES } from "./recovery";
import { fields } from "./property";

function opexLines(deal: Deal, changed: () => void): HTMLElement {
  const opex = (deal.opex ??= {});
  const rows = (opex.lines ??= []);
  const blank: ExpenseLine = { id: "e1", amount_yr: 0, recoverable: false };
  return editTable<ExpenseLine>({
    rows: () => rows,
    make: (rs) => copyLast(rs, blank, "e"),
    errorKey: (_r, i) => `opex.lines[${i}]`,
    addLabel: "Add expense",
    onChange: changed,
    cols: [
      textCol("ID", (r) => r.id, (r, v) => (r.id = v ?? r.id), { required: true }),
      textCol("Name", (r) => r.name, (r, v) => put(r, "name", v), { wide: true }),
      numCol("$ / yr", (r) => r.amount_yr, (r, v) => (r.amount_yr = v ?? r.amount_yr), { required: true }),
      numCol("Growth %", (r) => r.growth, (r, v) => put(r, "growth", v), { scale: 100 }),
      boolCol("Recoverable", (r) => r.recoverable, (r, v) => (r.recoverable = v)),
      boolCol("Controllable", (r) => r.controllable === true, (r, v) => put(r, "controllable", v || undefined)),
    ],
  });
}

function otherIncome(deal: Deal, changed: () => void): HTMLElement {
  const rows = (deal.other_income ??= []);
  const blank: IncomeLine = { id: "i1", amount_yr: 0 };
  return editTable<IncomeLine>({
    rows: () => rows,
    make: (rs) => copyLast(rs, blank, "i"),
    errorKey: (_r, i) => `other_income[${i}]`,
    addLabel: "Add income",
    onChange: changed,
    cols: [
      textCol("ID", (r) => r.id, (r, v) => (r.id = v ?? r.id), { required: true }),
      textCol("Name", (r) => r.name, (r, v) => put(r, "name", v), { wide: true }),
      numCol("$ / yr", (r) => r.amount_yr, (r, v) => (r.amount_yr = v ?? r.amount_yr), { required: true }),
      numCol("Growth %", (r) => r.growth, (r, v) => put(r, "growth", v), { scale: 100 }),
    ],
  });
}

interface Named {
  name: string;
  rec: Recovery;
}

function namedRecoveries(deal: Deal, changed: () => void): HTMLElement {
  // The deal keeps these as a map; the table edits a list and writes it back.
  const rows: Named[] = Object.entries(deal.recoveries ?? {}).map(([name, rec]) => ({ name, rec }));
  const sync = (): void => {
    if (rows.length) deal.recoveries = Object.fromEntries(rows.map((r) => [r.name, r.rec]));
    else delete deal.recoveries;
    changed();
  };
  return editTable<Named>({
    rows: () => rows,
    make: (rs) => {
      const taken = rs.map((r) => r.name);
      const name = taken.includes("default") ? `structure_${rs.length + 1}` : "default";
      return { name, rec: { type: "nnn" } };
    },
    errorKey: (r) => `recoveries.${r.name}`,
    addLabel: "Add structure",
    onChange: sync,
    cols: [
      textCol("Name", (r) => r.name, (r, v) => (r.name = v ?? r.name), { required: true }),
      selectCol<Named>("Type", () => RECOVERY_TYPES, (r) => r.rec.type, (r, v) => (r.rec = newRecovery(v as RecoveryType, deal, r.rec)), true),
      paramCol<Named>((r) => r.rec),
      capCol<Named>((r) => r.rec),
    ],
  });
}

export function expensesTab(deal: Deal, changed: (structural?: boolean) => void): HTMLElement {
  const opex = (deal.opex ??= {});
  const rev = (deal.revenue ??= {});
  const mf = deal.property.type === "multifamily";
  return h(
    "section",
    { class: "tab-body" },
    h("h3", {}, "Operating expenses"),
    opexLines(deal, changed),
    fields(
      numField({ label: "Management fee", value: opex.management_fee_pct, scale: 100, suffix: "% of EGI", onValue: (v) => (put(opex, "management_fee_pct", v), changed()) }),
    ),
    h("h3", {}, "Other income"),
    otherIncome(deal, changed),
    h("h3", {}, "Revenue"),
    fields(
      numField({ label: "General vacancy", value: rev.general_vacancy_pct, scale: 100, suffix: "% of PGR", onValue: (v) => (put(rev, "general_vacancy_pct", v), changed()) }),
      numField({ label: "Credit loss", value: rev.credit_loss_pct, scale: 100, suffix: "% of PGR", onValue: (v) => (put(rev, "credit_loss_pct", v), changed()) }),
    ),
    checkField("General vacancy on top of modeled downtime", rev.vacancy_on_top_of_downtime === true, (v) => {
      put(rev, "vacancy_on_top_of_downtime", v || undefined);
      changed();
    }),
    mf ? null : h("h3", {}, "Named recovery structures"),
    mf ? null : namedRecoveries(deal, changed),
    mf ? null : h("p", { class: "note" }, "A lease with no recovery uses the structure named default. Paste rows from a spreadsheet into any cell."),
  );
}
