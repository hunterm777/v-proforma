// Display formatting only; every value shown here was computed by the core.
// Money arrives as integer cents and is formatted with integer arithmetic.

function commas(n: number): string {
  const s = String(n);
  let out = "";
  for (let i = 0; i < s.length; i++) {
    if (i > 0 && (s.length - i) % 3 === 0) out += ",";
    out += s[i];
  }
  return out;
}

/** Whole dollars from cents, half away from zero: "1,234", "-56". */
export function dollars(c: number): string {
  const a = Math.abs(c);
  const d = Math.trunc(a / 100) + (a % 100 >= 50 ? 1 : 0);
  return `${c < 0 && d > 0 ? "-" : ""}${commas(d)}`;
}

/** "$1,234" from cents. */
export const usd = (c: number): string => (c < 0 ? `-$${dollars(-c)}` : `$${dollars(c)}`);

/** Exact dollars and cents from cents: "$1,234.56". */
export function usdExact(c: number): string {
  const a = Math.abs(c);
  return `${c < 0 ? "-" : ""}$${commas(Math.trunc(a / 100))}.${String(a % 100).padStart(2, "0")}`;
}

/** A decimal share as a percent: 0.0725 -> "7.25%"; null -> "–". */
export function pct(x: number | null | undefined, dp = 2): string {
  if (x === null || x === undefined || !Number.isFinite(x)) return "–";
  return `${(x * 100).toFixed(dp)}%`;
}

/** A multiple: "1.40x". */
export function mult(x: number | null | undefined, dp = 2): string {
  if (x === null || x === undefined || !Number.isFinite(x)) return "–";
  return `${x.toFixed(dp)}x`;
}

/** A plain number with separators and up to `dp` decimals. */
export function num(x: number | null | undefined, dp = 2): string {
  if (x === null || x === undefined || !Number.isFinite(x)) return "–";
  return x.toLocaleString("en-US", { maximumFractionDigits: dp });
}

const KINDS: Record<string, string> = {
  lease: "Lease",
  unit_type: "Unit type",
  expense: "Expense",
  income: "Income",
  capex: "Capex",
  assumption: "Assumption",
};

/** "Lease O1". */
export const source = (s: { kind: string; id: string }): string => `${KINDS[s.kind] ?? s.kind} ${s.id}`;

export const TYPES = [
  ["office", "Office"],
  ["retail", "Retail"],
  ["industrial", "Industrial"],
  ["multifamily", "Multifamily"],
] as const;

export const typeLabel = (t: string): string => TYPES.find(([k]) => k === t)?.[1] ?? t;
