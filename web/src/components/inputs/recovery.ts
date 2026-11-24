// Recovery structures and rent steps as table cells. Used by the rent roll
// (inline or named recoveries) and by the named recoveries table.
import { read, show } from "../../dom";
import type { Deal, Recovery, RecoveryType, Steps } from "../../types";
import { numCol, put } from "../cols";
import { pickOption, type Col } from "../table";

export const RECOVERY_TYPES: readonly (readonly [RecoveryType, string])[] = [
  ["nnn", "nnn"],
  ["base_year", "base_year"],
  ["expense_stop", "expense_stop"],
  ["modified_gross", "modified_gross"],
  ["gross", "gross"],
];

/** A fresh structure of `type`, keeping the controllable cap where both have one. */
export function newRecovery(type: RecoveryType, deal: Deal, prev?: Recovery): Recovery {
  const cap = prev && prev.type !== "gross" ? prev.cap_controllable_pct : undefined;
  const withCap = <T extends Recovery>(r: T): T => {
    if (cap !== undefined && r.type !== "gross") (r as { cap_controllable_pct?: number }).cap_controllable_pct = cap;
    return r;
  };
  switch (type) {
    case "nnn":
      return withCap({ type });
    case "base_year":
      return withCap({ type, year: Number(deal.property.start_date.slice(0, 4)) });
    case "expense_stop":
      return withCap({ type, stop_psf_yr: 0 });
    case "modified_gross":
      return withCap({ type, lines: (deal.opex?.lines ?? []).filter((l) => l.recoverable).slice(0, 1).map((l) => l.id) });
    case "gross":
      return { type };
  }
}

const hasParam = (r: Recovery | undefined): boolean =>
  r !== undefined && (r.type === "base_year" || r.type === "expense_stop" || r.type === "modified_gross");

/** The parameter of a structure as cell text: base year, stop $/SF or line ids. */
export function paramCol<R>(get: (r: R) => Recovery | undefined): Col<R> {
  return {
    label: "Rec. param",
    kind: "text",
    title: "base_year: the year; expense_stop: $/SF/yr; modified_gross: recoverable line ids",
    disabled: (r) => !hasParam(get(r)),
    get: (r) => {
      const x = get(r);
      if (x?.type === "base_year") return String(x.year);
      if (x?.type === "expense_stop") return show(x.stop_psf_yr);
      if (x?.type === "modified_gross") return x.lines.join(", ");
      return "";
    },
    set: (r, t) => {
      const x = get(r);
      const s = String(t).trim();
      if (x?.type === "base_year") {
        const y = Number(s);
        if (!Number.isInteger(y)) return false;
        x.year = y;
      } else if (x?.type === "expense_stop") {
        const v = read(s);
        if (v === undefined || Number.isNaN(v)) return false;
        x.stop_psf_yr = v;
      } else if (x?.type === "modified_gross") {
        x.lines = s.split(/[,\s]+/).filter(Boolean);
      } else return s === "";
      return true;
    },
  };
}

/** The controllable-expense cap of a structure, as a percent. */
export function capCol<R>(get: (r: R) => Recovery | undefined): Col<R> {
  return numCol<R>(
    "Ctrl cap %",
    (r) => {
      const x = get(r);
      return x && x.type !== "gross" ? x.cap_controllable_pct : undefined;
    },
    (r, v) => {
      const x = get(r);
      if (x && x.type !== "gross") put(x, "cap_controllable_pct", v);
    },
    { scale: 100, title: "Cap on controllable expense growth per year", disabled: (r) => !get(r) || get(r)?.type === "gross" },
  );
}

/** The lease's recovery: default, a named structure, or an inline one. */
export function leaseRecoveryCol<R extends { recovery?: string | Recovery }>(deal: () => Deal): Col<R> {
  const options = (): [string, string][] => [
    ["", "default"],
    ...Object.keys(deal().recoveries ?? {}).map((k): [string, string] => [`name:${k}`, k]),
    ...RECOVERY_TYPES.map(([t, l]): [string, string] => [`type:${t}`, l]),
  ];
  return {
    label: "Recovery",
    kind: "select",
    structural: true,
    title: "default uses recoveries.default; a name uses that named structure; a type is set on the lease",
    options,
    get: (r) => (r.recovery === undefined ? "" : typeof r.recovery === "string" ? `name:${r.recovery}` : `type:${r.recovery.type}`),
    set: (r, t) => {
      const v = pickOption(options(), String(t));
      if (v === undefined) return false;
      if (v === "") Reflect.deleteProperty(r, "recovery");
      else if (v.startsWith("name:")) r.recovery = v.slice(5);
      else {
        const prev = typeof r.recovery === "object" ? r.recovery : undefined;
        r.recovery = newRecovery(v.slice(5) as RecoveryType, deal(), prev);
      }
      return true;
    },
  };
}

export const inline = (r: { recovery?: string | Recovery }): Recovery | undefined =>
  typeof r.recovery === "object" ? r.recovery : undefined;

const STEP_TYPES = [
  ["", "none"],
  ["percent", "percent"],
  ["fixed", "fixed"],
  ["cpi", "cpi"],
] as const;

export function stepTypeCol<R extends { steps?: Steps }>(): Col<R> {
  return {
    label: "Steps",
    kind: "select",
    structural: true,
    options: () => STEP_TYPES,
    get: (r) => r.steps?.type ?? "",
    set: (r, t) => {
      const v = pickOption(STEP_TYPES, String(t));
      if (v === undefined) return false;
      if (v === "") Reflect.deleteProperty(r, "steps");
      else if (v !== r.steps?.type) {
        r.steps = v === "percent" ? { type: v, rate: 0 } : v === "fixed" ? { type: v, amount_psf_yr: 0 } : { type: "cpi", rates: [0] };
      }
      return true;
    },
  };
}

/** percent: % per year; fixed: $/SF/yr per year; cpi: % for lease years 2, 3, ... */
export function stepValueCol<R extends { steps?: Steps }>(): Col<R> {
  return {
    label: "Step",
    kind: "text",
    title: "percent: % a year; fixed: $/SF/yr a year; cpi: % for lease years 2, 3, ... (the last repeats)",
    disabled: (r) => r.steps === undefined,
    get: (r) => {
      const s = r.steps;
      if (!s) return "";
      if (s.type === "percent") return show(s.rate, 100);
      if (s.type === "fixed") return show(s.amount_psf_yr);
      return s.rates.map((x) => show(x, 100)).join(", ");
    },
    set: (r, t) => {
      const s = r.steps;
      const text = String(t).trim();
      if (!s) return text === "";
      if (s.type === "cpi") {
        const rates = text.split(/[,;\s]+/).filter(Boolean).map((x) => read(x, 100));
        if (!rates.length || rates.some((x) => x === undefined || Number.isNaN(x))) return false;
        s.rates = rates as number[];
        return true;
      }
      const v = read(text, s.type === "percent" ? 100 : 1);
      if (v === undefined || Number.isNaN(v)) return false;
      if (s.type === "percent") s.rate = v;
      else s.amount_psf_yr = v;
      return true;
    },
  };
}
