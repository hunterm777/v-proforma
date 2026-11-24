// Sensitivity tab: pick two inputs by dotted path into the deal file, give
// values for each, and the core re-runs the deal for every pair and returns
// the two-way levered IRR and equity multiple tables.
import * as core from "../../core";
import { h, read, show, textField } from "../../dom";
import { mult, pct } from "../../format";
import { store } from "../../state";
import type { Deal, SensitivityGrid } from "../../types";
import { section } from "./view";

interface Spec {
  x: string;
  y: string;
  xs: string;
  ys: string;
}
let spec: Spec | null = null;
let last: { grid: SensitivityGrid; rev: number; ms: number } | null = null;
let error = "";
let staleNote: HTMLElement | null = null;

/** Shows whether the table predates the deal's latest edit. */
export function markStale(): void {
  staleNote?.classList.toggle("hidden", !last || last.rev === store.rev);
}

function defaults(d: Deal): Spec {
  const mf = d.property.type === "multifamily";
  return {
    x: "exit.cap_rate",
    xs: "0.06, 0.065, 0.07, 0.075, 0.08",
    y: mf ? "unit_mix.rent_growth" : "market.rent_growth",
    ys: "0.02, 0.025, 0.03, 0.035, 0.04",
  };
}

/** Every numeric field of the deal as a dotted path; lists both as a whole and by index. */
function paths(d: Deal): string[] {
  const out = new Set<string>();
  const walk = (v: unknown, at: string[]): void => {
    if (typeof v === "number") out.add(at.join("."));
    else if (Array.isArray(v)) {
      v.forEach((x, i) => {
        walk(x, at);
        walk(x, [...at, String(i)]);
      });
    } else if (v && typeof v === "object") for (const [k, x] of Object.entries(v)) walk(x, [...at, k]);
  };
  walk(d, []);
  return [...out].sort();
}

function values(t: string): number[] | null {
  const xs = t.split(/[,;\s]+/).filter(Boolean).map((x) => read(x));
  return xs.length && xs.every((x) => x !== undefined && !Number.isNaN(x)) ? (xs as number[]) : null;
}

function grid(g: SensitivityGrid, title: string, pick: (g: SensitivityGrid) => (number | null)[][], fmt: (x: number | null) => string): HTMLElement {
  const m = pick(g);
  return section(
    title,
    h(
      "div",
      { class: "table-scroll" },
      h(
        "table",
        { class: "view sens" },
        h("thead", {}, h("tr", {}, h("th", { class: "l corner" }, `${g.y} ↓  ${g.x} →`), ...g.x_values.map((x) => h("th", {}, show(x))))),
        h(
          "tbody",
          {},
          ...g.y_values.map((y, i) =>
            h(
              "tr",
              {},
              h("th", { class: "l" }, show(y)),
              ...g.x_values.map((_, j) => {
                const err = g.errors[i]?.[j] ?? "";
                return h("td", { class: err ? "err" : "", title: err || undefined }, err ? "error" : fmt(m[i]?.[j] ?? null));
              }),
            ),
          ),
        ),
      ),
    ),
  );
}

export function sensitivityView(): HTMLElement {
  const s = (spec ??= defaults(store.deal));
  const list = h("datalist", { id: "sens-paths" }, ...paths(store.deal).map((p) => h("option", { value: p })));
  const pathField = (label: string, k: "x" | "y"): HTMLElement => {
    const f = textField(label, s[k], (v) => (s[k] = v.trim()));
    f.querySelector("input")?.setAttribute("list", "sens-paths");
    return f;
  };
  const out = h("div", { class: "stack" });
  staleNote = h("p", { class: "note warn" }, "The deal has changed since this table ran; run it again to update.");
  markStale();
  const runBtn = h("button", { type: "button", class: "primary" }, "Run");
  runBtn.addEventListener("click", () => {
    const xv = values(s.xs);
    const yv = values(s.ys);
    if (!xv || !yv) {
      error = "Give numbers for both value lists, separated by commas.";
    } else {
      const t0 = performance.now();
      const r = core.sensitivity(store.text(), { x: s.x, y: s.y, x_values: xv, y_values: yv });
      if ("ok" in r) {
        last = { grid: r.ok, rev: store.rev, ms: performance.now() - t0 };
        error = "";
      } else error = r.err;
    }
    out.replaceWith(sensitivityView());
  });
  out.append(
    section(
      "Two-way sensitivity",
      h("p", { class: "note" }, "Paths are dotted fields of the deal file; a path through a list sets every item (market.rent_growth), an index sets one (market.0.rent_growth). Values are in deal-file units: decimals for rates (0.065 is 6.5%)."),
      h(
        "div",
        { class: "fields sens-inputs" },
        pathField("Across (x) path", "x"),
        textField("x values", s.xs, (v) => (s.xs = v)),
        pathField("Down (y) path", "y"),
        textField("y values", s.ys, (v) => (s.ys = v)),
      ),
      list,
      h("div", { class: "row-actions" }, runBtn, last ? h("span", { class: "note" }, `${last.grid.x_values.length * last.grid.y_values.length} runs in ${last.ms.toFixed(0)} ms`) : null),
      error ? h("p", { class: "banner" }, error) : null,
      staleNote,
    ),
  );
  if (last) {
    out.append(grid(last.grid, "Levered IRR", (g) => g.levered_irr, (x) => pct(x)), grid(last.grid, "Equity multiple", (g) => g.equity_multiple, (x) => mult(x)));
  }
  return out;
}

/** A new deal starts from fresh defaults. */
export function resetSensitivity(): void {
  spec = null;
  last = null;
  error = "";
}
