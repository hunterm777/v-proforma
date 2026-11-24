// An editable table for the list blocks of the deal file (rent roll, unit
// mix, market profiles, expense lines, ...). Rows can be pasted from a
// spreadsheet as tab-separated text; the paste starts at the focused cell.
import { h, nextId } from "../dom";

export type Kind = "text" | "num" | "date" | "bool" | "select";

export interface Col<R> {
  label: string;
  kind: Kind;
  /** Cell text (or the checkbox state for "bool"). */
  get: (r: R) => string | boolean;
  /** Applies cell text; returns false when the text can't be read. */
  set: (r: R, v: string | boolean) => boolean;
  options?: () => readonly (readonly [string, string])[];
  /** Suggestions for a text cell. */
  suggest?: () => readonly string[];
  /** Redraw the table after this column changes (its other cells depend on it). */
  structural?: boolean;
  /** The cell can't be edited for this row. */
  disabled?: (r: R) => boolean;
  wide?: boolean;
  title?: string;
}

export interface TableOpts<R> {
  rows: () => R[];
  cols: Col<R>[];
  /** A new row; given the rows so far. */
  make: (rows: R[]) => R;
  /** The prefix the core uses for this row in its errors, e.g. "rent_roll[3]". */
  errorKey: (r: R, i: number) => string;
  addLabel: string;
  /** Called after every accepted edit; the table redraws itself as needed. */
  onChange: () => void;
}

const TRUE = new Set(["true", "yes", "y", "1", "x"]);

/** Applies a select cell's text: an option value or its label. */
export function pickOption(options: readonly (readonly [string, string])[], t: string): string | undefined {
  const s = t.trim().toLowerCase();
  return options.find(([v, l]) => v.toLowerCase() === s || l.toLowerCase() === s)?.[0];
}

interface Live {
  body: HTMLTableSectionElement;
  keys: () => string[];
}
const live = new Set<Live>();

/** Marks rows the core's errors name, and puts the message on the row. */
export function markTableErrors(errors: string[]): void {
  for (const t of live) {
    if (!t.body.isConnected) {
      live.delete(t);
      continue;
    }
    const keys = t.keys();
    [...t.body.rows].forEach((tr, i) => {
      const key = keys[i] ?? "\u0000";
      const mine = errors.filter((e) => e.startsWith(key) && /^[\s:.]/.test(e.slice(key.length)));
      tr.classList.toggle("row-error", mine.length > 0);
      if (mine.length) tr.title = mine.join("\n");
      else tr.removeAttribute("title");
    });
  }
}

export function editTable<R>(o: TableOpts<R>): HTMLElement {
  const wrap = h("div", { class: "edit-wrap" });
  const status = h("p", { class: "note" });

  function changed(c: Col<R>): void {
    if (c.structural === true) render();
    o.onChange();
  }

  function cell(r: R, c: Col<R>, ri: number, ci: number): HTMLTableCellElement {
    const off = c.disabled?.(r) === true;
    const at = { "data-r": ri, "data-c": ci, disabled: off, title: c.title };
    let control: HTMLElement;
    if (c.kind === "bool") {
      const box = h("input", { type: "checkbox", checked: c.get(r) === true, ...at });
      box.addEventListener("change", () => {
        c.set(r, box.checked);
        changed(c);
      });
      control = box;
    } else if (c.kind === "select") {
      const value = String(c.get(r));
      const opts = c.options?.() ?? [];
      const sel = h("select", at, ...opts.map(([v, l]) => h("option", { value: v, selected: v === value }, l)));
      sel.addEventListener("change", () => {
        if (c.set(r, sel.value)) changed(c);
      });
      control = sel;
    } else {
      const list = c.suggest ? nextId("dl") : undefined;
      const input = h("input", {
        type: "text",
        value: String(c.get(r)),
        class: c.kind === "text" ? "" : "num",
        autocomplete: "off",
        spellcheck: "false",
        placeholder: c.kind === "date" ? "YYYY-MM-DD" : undefined,
        list,
        ...at,
      });
      input.addEventListener("input", () => {
        const ok = c.set(r, input.value);
        input.classList.toggle("invalid", !ok);
        if (ok) changed(c);
      });
      control = list
        ? h("span", {}, input, h("datalist", { id: list }, ...(c.suggest?.() ?? []).map((s) => h("option", { value: s }))))
        : input;
    }
    return h("td", { class: c.wide ? "wide" : "" }, control);
  }

  function render(): void {
    const rows = o.rows();
    const body = h("tbody");
    rows.forEach((r, ri) => {
      const rm = h("button", { type: "button", class: "ghost icon", title: "Remove row", "aria-label": "Remove row" }, "×");
      rm.addEventListener("click", () => {
        rows.splice(ri, 1);
        render();
        o.onChange();
      });
      body.append(h("tr", {}, ...o.cols.map((c, ci) => cell(r, c, ri, ci)), h("td", { class: "rm" }, rm)));
    });
    body.addEventListener("paste", (ev) => paste(ev as ClipboardEvent));
    live.add({ body, keys: () => o.rows().map((r, i) => o.errorKey(r, i)) });
    const add = h("button", { type: "button" }, o.addLabel);
    add.addEventListener("click", () => {
      rows.push(o.make(rows));
      render();
      o.onChange();
    });
    const head = h("thead", {}, h("tr", {}, ...o.cols.map((c) => h("th", { title: c.title }, c.label)), h("th")));
    wrap.replaceChildren(
      h("div", { class: "table-scroll" }, h("table", { class: "edit" }, head, body)),
      h("div", { class: "row-actions" }, add, status),
    );
  }

  function paste(ev: ClipboardEvent): void {
    const target = ev.target as HTMLElement;
    const r0 = Number(target.dataset["r"]);
    const c0 = Number(target.dataset["c"]);
    const text = ev.clipboardData?.getData("text/plain") ?? "";
    if (!Number.isInteger(r0) || !Number.isInteger(c0) || !/[\t\n]/.test(text)) return;
    ev.preventDefault();
    const lines = text.replace(/\r/g, "").split("\n");
    while (lines.length && lines[lines.length - 1]?.trim() === "") lines.pop();
    const rows = o.rows();
    let bad = 0;
    lines.forEach((line, i) => {
      while (rows.length <= r0 + i) rows.push(o.make(rows));
      const row = rows[r0 + i] as R;
      line.split("\t").forEach((t, j) => {
        const c = o.cols[c0 + j];
        if (!c || c.disabled?.(row) === true) return;
        const ok = c.kind === "bool" ? c.set(row, TRUE.has(t.trim().toLowerCase())) : c.set(row, t.trim());
        if (!ok) bad++;
      });
    });
    render();
    status.textContent = `Pasted ${lines.length} row${lines.length === 1 ? "" : "s"}${bad ? `; ${bad} cell${bad === 1 ? "" : "s"} could not be read and kept their old values` : ""}.`;
    o.onChange();
  }

  render();
  return wrap;
}
