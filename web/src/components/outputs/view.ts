// Read-only tables for the result views.
import { h, type Child } from "../../dom";
import { dollars } from "../../format";

export type Cell = Child | HTMLTableCellElement;

/** A money cell from cents (whole dollars); extra attributes make it clickable. */
export function money(c: number, attrs: Record<string, string> = {}): HTMLTableCellElement {
  const cls = [c < 0 ? "neg" : "", attrs["data-trace"] ? "click" : ""].filter(Boolean).join(" ");
  return h("td", { class: cls, ...attrs }, dollars(c));
}

/** A table with a left-aligned first column; plain values become cells. */
export function viewTable(head: string[], rows: Cell[][], cls = ""): HTMLElement {
  const td = (c: Cell, i: number): HTMLTableCellElement =>
    c instanceof HTMLTableCellElement ? c : h("td", { class: i === 0 ? "l" : "" }, c);
  return h(
    "div",
    { class: "table-scroll" },
    h(
      "table",
      { class: `view ${cls}` },
      h("thead", {}, h("tr", {}, ...head.map((x, i) => h("th", { class: i === 0 ? "l" : "" }, x)))),
      h("tbody", {}, ...rows.map((r) => h("tr", {}, ...r.map(td)))),
    ),
  );
}

/** Label / value pairs. */
export function facts(items: [string, Child][]): HTMLElement {
  return h("dl", { class: "facts" }, ...items.flatMap(([k, v]) => [h("dt", {}, k), h("dd", {}, v)]));
}

export const section = (title: string, ...body: Child[]): HTMLElement => h("section", { class: "out" }, h("h2", {}, title), ...body);
