// The audit trail. Any number marked data-trace="line|y|year" or
// "line|m|month" opens the core's trace of that line for that period: every
// source, its calculation step, its detail and its signed cents. The trail of
// debt service also links to the loan schedule for the same months.
import * as core from "../core";
import { h } from "../dom";
import { source, usdExact } from "../format";
import { store } from "../state";
import type { Result } from "../types";

/** Ledger lines: traced from year 1 and month 1. */
export const TRACEABLE = new Set([
  "base_rent", "recoveries", "other_income", "pgr", "general_vacancy", "credit_loss", "free_rent", "egi",
  "opex", "management_fee", "noi", "ti", "lc", "reserves", "capex", "cash_flow_before_debt",
]);
/** Lines below cash flow before debt: traced from year 0 and month 0, the close. */
export const FINANCE = new Set([
  "debt_service", "cash_flow_after_debt", "purchase", "loan_net", "net_sale", "loan_payoff",
  "unlevered_cash_flow", "levered_cash_flow",
]);

/** Whether the core traces `line` in analysis year (or model month) `n`. */
export const traces = (line: string, n: number): boolean => (FINANCE.has(line) ? n >= 0 : TRACEABLE.has(line) && n >= 1);

/** Attributes that make a cell open its trail. */
export const traceYear = (line: string, year: number, src?: string): Record<string, string> =>
  src ? { "data-trace": `${line}|y|${year}`, "data-src": src } : { "data-trace": `${line}|y|${year}` };
export const traceMonth = (line: string, month: number): Record<string, string> => ({ "data-trace": `${line}|m|${month}` });

type Open = { kind: "trace"; line: string; year?: number; month?: number; src?: string } | { kind: "sched"; from: number; to: number };

const panel = h("aside", { class: "audit hidden", "aria-label": "Audit trail", role: "dialog" });
let open: Open | null = null;

const label = (r: Result | null, key: string): string =>
  r?.annual.rows.find((x) => x.key === key)?.label ?? key;
const monthName = (r: Result | null, m: number): string =>
  m === 0 ? `close (${r?.start_date ?? ""})` : (r?.monthly.columns[m - 1] ?? `month ${m}`);

function close(): void {
  open = null;
  panel.classList.add("hidden");
}

function frame(title: string, sub: string, body: HTMLElement): void {
  const x = h("button", { type: "button", class: "ghost icon", "aria-label": "Close" }, "×");
  x.addEventListener("click", close);
  panel.replaceChildren(h("div", { class: "audit-head" }, h("div", {}, h("h2", {}, title), h("p", { class: "muted" }, sub)), x), body);
  panel.classList.remove("hidden");
}

function drawTrace(o: Extract<Open, { kind: "trace" }>): void {
  const r = store.result;
  const t = core.trace(store.text(), o.line, o.year, o.month);
  const when =
    o.year !== undefined ? (o.year === 0 ? "Year 0 (close)" : `Year ${o.year}`) : o.month !== undefined ? monthName(r, o.month) : "Hold";
  if ("err" in t) {
    frame(label(r, o.line), when, h("p", { class: "banner" }, t.err));
    return;
  }
  const tr = t.ok;
  const sub = `${when} · model months ${tr.from_month} to ${tr.to_month} · ${tr.rows.length} source${tr.rows.length === 1 ? "" : "s"}`;
  const rows = tr.rows.map((x) =>
    h(
      "tr",
      { class: o.src !== undefined && x.source.id === o.src ? "mark" : "" },
      h("td", { class: "l" }, source(x.source)),
      h("td", { class: "l" }, x.step),
      h("td", { class: "l detail" }, x.detail),
      h("td", { class: x.cents < 0 ? "neg" : "" }, usdExact(x.cents)),
      h("td", { class: "muted" }, String(x.cents)),
    ),
  );
  let sched: HTMLElement | null = null;
  if (o.line === "debt_service" && r?.debt && tr.to_month >= 1) {
    const from = Math.max(tr.from_month, 1);
    sched = h("button", { type: "button", class: "ghost" }, "Loan schedule for these months");
    sched.addEventListener("click", () => {
      open = { kind: "sched", from, to: tr.to_month };
      draw();
    });
  }
  frame(
    label(r, o.line),
    sub,
    h(
      "div",
      { class: "table-scroll" },
      h(
        "table",
        { class: "view" },
        h("thead", {}, h("tr", {}, h("th", { class: "l" }, "Source"), h("th", { class: "l" }, "Step"), h("th", { class: "l" }, "Detail"), h("th", {}, "Amount"), h("th", {}, "Cents"))),
        h("tbody", {}, ...rows),
        h("tfoot", {}, h("tr", {}, h("th", { class: "l" }, "Total"), h("th"), h("th"), h("th", {}, usdExact(tr.total)), h("th", { class: "muted" }, String(tr.total)))),
      ),
      sched,
    ),
  );
}

function drawSchedule(o: Extract<Open, { kind: "sched" }>): void {
  const r = store.result;
  const pays = (r?.debt?.schedule ?? []).filter((p) => p.month >= o.from && p.month <= o.to);
  const cols = ["Month", "Rate", "Opening", "Interest", "Principal", "Payment"];
  frame(
    "Debt service",
    `Loan schedule, model months ${o.from} to ${o.to}`,
    h(
      "div",
      { class: "table-scroll" },
      h(
        "table",
        { class: "view" },
        h("thead", {}, h("tr", {}, ...cols.map((c, i) => h("th", { class: i === 0 ? "l" : "" }, c)))),
        h(
          "tbody",
          {},
          ...pays.map((p) =>
            h(
              "tr",
              {},
              h("td", { class: "l" }, `${p.month} · ${monthName(r, p.month)}`),
              h("td", {}, `${(p.rate * 100).toFixed(4)}%`),
              h("td", {}, usdExact(p.opening)),
              h("td", {}, usdExact(p.interest)),
              h("td", {}, usdExact(p.principal)),
              h("td", {}, usdExact(p.payment)),
            ),
          ),
        ),
      ),
    ),
  );
}

function draw(): void {
  if (!open) return;
  if (open.kind === "trace") drawTrace(open);
  else drawSchedule(open);
}

/** Opens the trail for a clicked number, if it has one. */
export function onClick(ev: MouseEvent): void {
  const el = (ev.target as HTMLElement).closest<HTMLElement>("[data-trace]");
  const t = el?.dataset["trace"];
  if (!el || !t) return;
  const [line = "", unit, n] = t.split("|");
  const v = Number(n);
  open = unit === "y" ? { kind: "trace", line, year: v } : { kind: "trace", line, month: v };
  const src = el.dataset["src"];
  if (src !== undefined) open.src = src;
  draw();
}

export function auditPanel(): HTMLElement {
  document.addEventListener("keydown", (e) => e.key === "Escape" && close());
  // An open trail follows the deal as it's edited.
  store.subscribe(() => {
    if (open && store.errors.length === 0) draw();
  });
  return panel;
}
