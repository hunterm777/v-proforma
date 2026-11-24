// Tiny DOM helpers. No styles are set through attributes, so the page runs
// under a CSP without 'unsafe-inline'.

type Attrs = Record<string, string | number | boolean | undefined | EventListener>;
export type Child = Node | string | null | undefined | false;

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: (Child | Child[])[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === false) continue;
    if (typeof v === "function") el.addEventListener(k.replace(/^on/, ""), v);
    else if (k === "class") el.className = String(v);
    else if (v === true) el.setAttribute(k, "");
    else el.setAttribute(k, String(v));
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return el;
}

let uid = 0;
export const nextId = (p: string): string => `${p}-${++uid}`;

/** Trims float noise for showing a stored number back in an input. */
export function show(x: number | undefined, scale = 1): string {
  if (x === undefined || !Number.isFinite(x)) return "";
  return String(Number((x * scale).toPrecision(12)));
}

/** Reads an input; "" is undefined, junk is NaN. Accepts "$3,400", "5%". */
export function read(v: string, scale = 1): number | undefined {
  const t = v.trim().replace(/[,$%\s]/g, "");
  if (t === "") return undefined;
  const n = Number(t);
  return Number.isFinite(n) ? Number((n / scale).toPrecision(15)) : Number.NaN;
}

/** Reads a whole number of at least 0 (months, units, years). */
export function readInt(v: string): number | undefined {
  const n = read(v);
  if (n === undefined || Number.isNaN(n)) return n;
  return Number.isInteger(n) && n >= 0 ? n : Number.NaN;
}

/** Reads a date as YYYY-MM-DD; also takes M/D/YYYY from US spreadsheets. */
export function readDate(v: string): string | undefined {
  const t = v.trim();
  let m = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(t);
  let y: string | undefined, mo: string | undefined, d: string | undefined;
  if (m) [, y, mo, d] = m;
  else if ((m = /^(\d{1,2})\/(\d{1,2})\/(\d{4})$/.exec(t))) [, mo, d, y] = m;
  if (!y || !mo || !d) return undefined;
  return `${y}-${mo.padStart(2, "0")}-${d.padStart(2, "0")}`;
}

export function field(label: string, control: HTMLElement, id: string, hint?: string): HTMLLabelElement {
  return h(
    "label",
    { class: "field", for: id },
    h("span", { class: "flabel" }, label),
    control,
    hint ? h("span", { class: "hint" }, hint) : null,
  );
}

export interface NumOpts {
  label: string;
  value: number | undefined;
  /** 100 shows a decimal share as a percent. */
  scale?: number;
  suffix?: string;
  required?: boolean;
  integer?: boolean;
  placeholder?: string;
  onValue: (v: number | undefined) => void;
}

export function numField(o: NumOpts): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", {
    id,
    type: "text",
    inputmode: "decimal",
    autocomplete: "off",
    spellcheck: "false",
    class: "num",
    value: show(o.value, o.scale),
    placeholder: o.placeholder,
  });
  input.addEventListener("input", () => {
    const v = o.integer ? readInt(input.value) : read(input.value, o.scale);
    const bad = Number.isNaN(v) || (o.required === true && v === undefined);
    input.classList.toggle("invalid", bad);
    input.setAttribute("aria-invalid", String(bad));
    if (!bad) o.onValue(v);
  });
  const box = h("span", { class: "fbox" }, input, o.suffix ? h("span", { class: "suffix" }, o.suffix) : null);
  return field(o.label, box, id);
}

export function textField(label: string, value: string, onValue: (v: string) => void, placeholder = ""): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", { id, type: "text", value, placeholder, autocomplete: "off", spellcheck: "false" });
  input.addEventListener("input", () => onValue(input.value));
  return field(label, input, id);
}

export function dateField(label: string, value: string, onValue: (v: string) => void): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", { id, type: "text", value, placeholder: "YYYY-MM-DD", class: "num", autocomplete: "off" });
  input.addEventListener("input", () => {
    const d = readDate(input.value);
    input.classList.toggle("invalid", d === undefined);
    if (d) onValue(d);
  });
  return field(label, input, id);
}

export function selectField<T extends string>(
  label: string,
  value: T,
  options: readonly (readonly [T, string])[],
  onValue: (v: T) => void,
): HTMLLabelElement {
  const id = nextId("f");
  const sel = h("select", { id }, ...options.map(([v, t]) => h("option", { value: v, selected: v === value }, t)));
  sel.addEventListener("change", () => onValue(sel.value as T));
  return field(label, sel, id);
}

export function checkField(label: string, value: boolean, onValue: (v: boolean) => void): HTMLLabelElement {
  const id = nextId("f");
  const box = h("input", { id, type: "checkbox", checked: value });
  box.addEventListener("change", () => onValue(box.checked));
  return h("label", { class: "check", for: id }, box, h("span", {}, label));
}

export function segmented<T extends string>(
  name: string,
  value: T,
  options: readonly (readonly [T, string])[],
  onValue: (v: T) => void,
): HTMLDivElement {
  const group = h("div", { class: "segmented", role: "radiogroup", "aria-label": name });
  for (const [v, t] of options) {
    const on = v === value;
    const b = h("button", { type: "button", role: "radio", "aria-checked": String(on), class: on ? "on" : "" }, t);
    b.addEventListener("click", () => {
      if (v !== value) onValue(v);
    });
    group.append(b);
  }
  return group;
}

/** A row of tab buttons; `onPick` gets the chosen id. */
export function tabBar(ids: readonly (readonly [string, string])[], active: string, onPick: (id: string) => void): HTMLElement {
  const bar = h("div", { class: "tabbar", role: "tablist" });
  for (const [id, label] of ids) {
    const b = h("button", { type: "button", role: "tab", "aria-selected": String(id === active) }, label);
    b.addEventListener("click", () => onPick(id));
    bar.append(b);
  }
  return bar;
}

/** Saves text through a temporary link; nothing leaves the browser. */
export function download(name: string, text: string, type: string): void {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = h("a", { href: url, download: name, class: "hidden" });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
