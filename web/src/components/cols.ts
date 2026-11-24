// Column builders for editable tables. Each one reads cell text into a deal
// field and refuses text it can't read (the cell turns red, the deal keeps
// its old value).
import { read, readDate, readInt, show } from "../dom";
import { pickOption, type Col } from "./table";

/** Sets an optional field, or removes it for `undefined`. */
export function put<T extends object, K extends keyof T>(o: T, k: K, v: T[K] | undefined): void {
  if (v === undefined) Reflect.deleteProperty(o, k);
  else o[k] = v;
}

interface NumOpts<R> {
  /** 100 shows a decimal share as a percent. */
  scale?: number;
  required?: boolean;
  integer?: boolean;
  title?: string;
  disabled?: (r: R) => boolean;
}

export function numCol<R>(
  label: string,
  get: (r: R) => number | undefined,
  set: (r: R, v: number | undefined) => void,
  o: NumOpts<R> = {},
): Col<R> {
  const c: Col<R> = {
    label,
    kind: "num",
    get: (r) => show(get(r), o.scale),
    set: (r, t) => {
      const v = o.integer ? readInt(String(t)) : read(String(t), o.scale);
      if (Number.isNaN(v) || (o.required === true && v === undefined)) return false;
      set(r, v);
      return true;
    },
  };
  if (o.title) c.title = o.title;
  if (o.disabled) c.disabled = o.disabled;
  return c;
}

export function textCol<R>(
  label: string,
  get: (r: R) => string | undefined,
  set: (r: R, v: string | undefined) => void,
  o: { required?: boolean; wide?: boolean; suggest?: () => readonly string[] } = {},
): Col<R> {
  const c: Col<R> = {
    label,
    kind: "text",
    wide: o.wide === true,
    get: (r) => get(r) ?? "",
    set: (r, t) => {
      const v = String(t).trim();
      if (o.required === true && v === "") return false;
      set(r, v === "" ? undefined : v);
      return true;
    },
  };
  if (o.suggest) c.suggest = o.suggest;
  return c;
}

export function dateCol<R>(label: string, get: (r: R) => string, set: (r: R, v: string) => void): Col<R> {
  return {
    label,
    kind: "date",
    get,
    set: (r, t) => {
      const d = readDate(String(t));
      if (!d) return false;
      set(r, d);
      return true;
    },
  };
}

export function boolCol<R>(label: string, get: (r: R) => boolean, set: (r: R, v: boolean) => void): Col<R> {
  return { label, kind: "bool", get, set: (r, v) => (set(r, v === true), true) };
}

export function selectCol<R>(
  label: string,
  options: () => readonly (readonly [string, string])[],
  get: (r: R) => string,
  set: (r: R, v: string) => void,
  structural = false,
): Col<R> {
  return {
    label,
    kind: "select",
    options,
    structural,
    get,
    set: (r, t) => {
      const v = pickOption(options(), String(t));
      if (v === undefined) return false;
      set(r, v);
      return true;
    },
  };
}

/** A unique id like "L7" for a new row. */
export function freshId(prefix: string, taken: readonly string[]): string {
  let n = taken.length + 1;
  while (taken.includes(`${prefix}${n}`)) n++;
  return `${prefix}${n}`;
}

/** A copy of the last row with a fresh id, or `blank` for an empty table. */
export function copyLast<R extends { id: string }>(rows: R[], blank: R, prefix: string): R {
  const last = rows[rows.length - 1];
  const r = structuredClone(last ?? blank);
  r.id = freshId(prefix, rows.map((x) => x.id));
  return r;
}
