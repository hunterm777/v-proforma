// Debt tab: a fixed amount or sizing constraints, fixed or floating rate,
// interest-only period, amortization, fees and prepayment.
import { checkField, h, numField, read, segmented, show, textField } from "../../dom";
import type { Deal, Debt } from "../../types";
import { numCol, put } from "../cols";
import { editTable } from "../table";
import { fields } from "./property";

// What a switch replaced, so switching back restores it.
const kept: {
  debt?: Debt | undefined;
  amount?: number | undefined;
  sizing?: Debt["sizing"] | undefined;
  fixed?: number | undefined;
  floating?: NonNullable<Debt["rate"]["floating"]> | undefined;
} = {};

function rateBlock(d: Debt, changed: (s?: boolean) => void): HTMLElement[] {
  const floating = d.rate.floating !== undefined;
  const pick = segmented("Rate", floating ? "floating" : "fixed", [["fixed", "Fixed"], ["floating", "Floating"]] as const, (v) => {
    if (v === "floating") {
      if (d.rate.fixed !== undefined) kept.fixed = d.rate.fixed;
      d.rate = { floating: kept.floating ?? { spread: 0, curve: [{ month: 1, index: 0 }] } };
    } else {
      if (d.rate.floating) kept.floating = d.rate.floating;
      d.rate = { fixed: kept.fixed ?? 0 };
    }
    changed(true);
  });
  const f = d.rate.floating;
  if (!f) {
    return [
      pick,
      fields(numField({ label: "Fixed rate", value: d.rate.fixed, scale: 100, suffix: "%", required: true, onValue: (v) => ((d.rate.fixed = v ?? 0), changed()) })),
    ];
  }
  return [
    pick,
    fields(numField({ label: "Spread", value: f.spread, scale: 100, suffix: "%", required: true, onValue: (v) => ((f.spread = v ?? f.spread), changed()) })),
    h("p", { class: "note" }, "Index curve: each point applies from its model month until the next."),
    editTable<{ month: number; index: number }>({
      rows: () => f.curve,
      make: (rs) => ({ month: (rs[rs.length - 1]?.month ?? 0) + 12, index: rs[rs.length - 1]?.index ?? 0 }),
      errorKey: () => "debt: rate.floating",
      addLabel: "Add point",
      onChange: () => changed(),
      cols: [
        numCol("From month", (r) => r.month, (r, v) => (r.month = v ?? r.month), { integer: true, required: true }),
        numCol("Index %", (r) => r.index, (r, v) => (r.index = v ?? r.index), { scale: 100, required: true }),
      ],
    }),
  ];
}

function sizingBlock(d: Debt, changed: (s?: boolean) => void): HTMLElement[] {
  const sized = d.amount === undefined;
  const pick = segmented("Loan amount", sized ? "sized" : "amount", [["sized", "Sized by constraints"], ["amount", "Fixed amount"]] as const, (v) => {
    if (v === "amount") {
      kept.sizing = d.sizing;
      delete d.sizing;
      d.amount = kept.amount ?? 0;
    } else {
      kept.amount = d.amount;
      delete d.amount;
      d.sizing = kept.sizing ?? { max_ltv: 0.6 };
    }
    changed(true);
  });
  if (!sized) {
    return [pick, fields(numField({ label: "Loan amount", value: d.amount, suffix: "$", required: true, onValue: (v) => ((d.amount = v ?? 0), changed()) }))];
  }
  const s = (d.sizing ??= {});
  return [
    pick,
    fields(
      numField({ label: "Max LTV", value: s.max_ltv, scale: 100, suffix: "%", onValue: (v) => (put(s, "max_ltv", v), changed()) }),
      numField({ label: "Min DSCR", value: s.min_dscr, suffix: "x", onValue: (v) => (put(s, "min_dscr", v), changed()) }),
      numField({ label: "Min debt yield", value: s.min_debt_yield, scale: 100, suffix: "%", onValue: (v) => (put(s, "min_debt_yield", v), changed()) }),
    ),
    h("p", { class: "note" }, "The loan is the smallest amount the constraints allow, on year-1 NOI. Blank means not used."),
  ];
}

/** "1" (flat) or "1, 0" (by loan year, the last repeats), in percent. */
function prepayField(d: Debt, changed: () => void): HTMLElement {
  const p = d.prepayment_penalty_pct;
  const text = p === undefined ? "" : Array.isArray(p) ? p.map((x) => show(x, 100)).join(", ") : show(p, 100);
  const label = textField("Prepayment penalty %", text, (v) => {
    const parts = v.split(/[,;\s]+/).filter(Boolean).map((x) => read(x, 100));
    const bad = parts.some((x) => x === undefined || Number.isNaN(x));
    label.querySelector("input")?.classList.toggle("invalid", bad);
    if (bad) return;
    const xs = parts as number[];
    put(d, "prepayment_penalty_pct", xs.length === 0 ? undefined : xs.length === 1 ? xs[0] : xs);
    changed();
  }, "e.g. 1 or 2, 1, 0 by loan year");
  return label;
}

export function debtTab(deal: Deal, changed: (structural?: boolean) => void): HTMLElement {
  const has = checkField("This deal has a loan", deal.debt !== undefined, (on) => {
    if (on) deal.debt = kept.debt ?? { sizing: { max_ltv: 0.6 }, rate: { fixed: 0.06 } };
    else {
      kept.debt = deal.debt;
      delete deal.debt;
    }
    changed(true);
  });
  const d = deal.debt;
  if (!d) return h("section", { class: "tab-body" }, has);
  return h(
    "section",
    { class: "tab-body" },
    has,
    h("h3", {}, "Amount"),
    ...sizingBlock(d, changed),
    h("h3", {}, "Rate"),
    ...rateBlock(d, changed),
    h("h3", {}, "Terms"),
    fields(
      numField({ label: "Interest-only", value: d.io_months, integer: true, suffix: "months", onValue: (v) => (put(d, "io_months", v), changed()) }),
      numField({ label: "Amortization", value: d.amortization_years, integer: true, suffix: "years", placeholder: "IO for the hold", onValue: (v) => (put(d, "amortization_years", v), changed()) }),
      numField({ label: "Origination fee", value: d.origination_fee_pct, scale: 100, suffix: "%", onValue: (v) => (put(d, "origination_fee_pct", v), changed()) }),
      prepayField(d, changed),
    ),
  );
}
