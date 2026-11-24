// Cash flow tab: the annual table from potential gross revenue to levered
// cash flow. A line opens to its months and to the entries (leases,
// expense lines, assumptions) behind it, from result.entries.
import { h } from "../../dom";
import { source } from "../../format";
import type { Entry, Result } from "../../types";
import { traceMonth, traces, traceYear } from "../audit";
import { money } from "./view";

const TOTALS = new Set(["pgr", "egi", "noi", "cash_flow_before_debt", "cash_flow_after_debt", "unlevered_cash_flow", "levered_cash_flow"]);
/** Sub-rows drawn before a "show all" button. */
const FIRST = 100;

const expanded = new Set<string>();
const showAll = new Set<string>();

/** The click target for a line's value in analysis year `y`. */
function yearAttrs(key: string, y: number, src?: string): Record<string, string> {
  return traces(key, y) ? traceYear(key, y, src) : {};
}

function monthsStrip(r: Result, key: string, span: number): HTMLTableRowElement | null {
  const row = r.monthly.rows.find((x) => x.key === key);
  if (!row) return null;
  const cells = row.values.map((v, i) => money(v, traces(key, i + 1) ? traceMonth(key, i + 1) : {}));
  const t = h(
    "table",
    { class: "view months" },
    h("thead", {}, h("tr", {}, ...r.monthly.columns.map((c) => h("th", {}, c)))),
    h("tbody", {}, h("tr", {}, ...cells)),
  );
  return h("tr", { class: "sub months-row" }, h("td", { class: "l" }, "By month"), h("td", { colspan: span }, h("div", { class: "table-scroll" }, t)));
}

function entryRow(key: string, e: Entry, years: number): HTMLTableRowElement {
  const name = `${source(e.source)} · ${e.detail}`;
  return h(
    "tr",
    { class: "sub" },
    h("td", { class: "l entry", title: `${name} (${e.step})` }, name),
    h("td"),
    ...Array.from({ length: years }, (_, i) => money(e.years[i] ?? 0, yearAttrs(key, i + 1, e.source.id))),
  );
}

export function cashFlowView(r: Result): HTMLElement {
  const years = r.annual.columns.length - 1;
  const span = years + 1;
  const byLine = new Map<string, Entry[]>();
  for (const e of r.entries) {
    const list = byLine.get(e.line) ?? [];
    list.push(e);
    byLine.set(e.line, list);
  }
  const body = h("tbody");
  for (const row of r.annual.rows) {
    const entries = byLine.get(row.key) ?? [];
    const hasMonths = r.monthly.rows.some((x) => x.key === row.key);
    const open = expanded.has(row.key);
    let toggle: HTMLElement | null = null;
    if (entries.length || hasMonths) {
      toggle = h("button", { type: "button", class: "caret", "aria-expanded": String(open), title: open ? "Close" : "Show months and sources" }, open ? "▾" : "▸");
      toggle.addEventListener("click", () => {
        if (open) expanded.delete(row.key);
        else expanded.add(row.key);
        body.closest("section")?.replaceWith(cashFlowView(r));
      });
    }
    body.append(
      h(
        "tr",
        { class: TOTALS.has(row.key) ? "total" : "" },
        h("td", { class: "l" }, toggle, row.label),
        ...row.values.map((v, y) => money(v, yearAttrs(row.key, y))),
      ),
    );
    if (!open) continue;
    const strip = monthsStrip(r, row.key, span);
    if (strip) body.append(strip);
    if (entries.length) {
      body.append(h("tr", { class: "sub note-row" }, h("td", { colspan: span + 1, class: "l" }, `${entries.length} source entr${entries.length === 1 ? "y" : "ies"}, as recorded in the ledger (positive amounts; the line above applies its sign)`)));
      const shown = showAll.has(row.key) ? entries : entries.slice(0, FIRST);
      for (const e of shown) body.append(entryRow(row.key, e, years));
      if (shown.length < entries.length) {
        const more = h("button", { type: "button", class: "ghost" }, `Show all ${entries.length}`);
        more.addEventListener("click", () => {
          showAll.add(row.key);
          body.closest("section")?.replaceWith(cashFlowView(r));
        });
        body.append(h("tr", { class: "sub" }, h("td", { colspan: span + 1, class: "l" }, more)));
      }
    }
  }
  return h(
    "section",
    { class: "out" },
    h("h2", {}, "Annual cash flow"),
    h("p", { class: "note" }, `Dollars. Click a number for its audit trail; open a line for its months and sources. Exit in month ${r.exit_month}.`),
    h(
      "div",
      { class: "table-scroll" },
      h("table", { class: "view cashflow" }, h("thead", {}, h("tr", {}, h("th", { class: "l" }, "Line"), ...r.annual.columns.map((c) => h("th", {}, c)))), body),
    ),
  );
}

