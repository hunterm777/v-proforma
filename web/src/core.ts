// The only door to the math: every number on the page comes from the Rust
// core, compiled to WebAssembly and inlined into this bundle (no fetch).
// Calls take the deal as text (JSON from the page, or a loaded YAML file).
import { initSync, run as wRun, trace as wTrace, sensitivity as wSens, csv as wCsv,
  to_yaml as wYaml, to_json as wJson, version as wVersion } from "./wasm/proforma_wasm.js";
import wasmBase64 from "./wasm/proforma_wasm_bg.wasm?b64";
import type { Deal, Result, SensitivityGrid, TraceResult } from "./types";

let ready = false;

/** Compiles the inlined module; call once before anything else. */
export function init(): void {
  if (ready) return;
  const bin = atob(wasmBase64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  initSync({ module: bytes });
  ready = true;
}

/** The core throws its errors as strings. */
export const message = (e: unknown): string => (typeof e === "string" ? e : String(e));

export type Outcome<T> = { ok: T } | { err: string };

function attempt<T>(f: () => T): Outcome<T> {
  try {
    return { ok: f() };
  } catch (e) {
    return { err: message(e) };
  }
}

/** Runs the deal; `raw` is the core's JSON text, kept for the JSON export. */
export function run(text: string): Outcome<{ raw: string; result: Result }> {
  return attempt(() => {
    const raw = wRun(text);
    return { raw, result: JSON.parse(raw) as Result };
  });
}

/** The audit trail of `line` in analysis `year` or model `month` (1-based). */
export function trace(text: string, line: string, year?: number, month?: number): Outcome<TraceResult> {
  return attempt(() => JSON.parse(wTrace(text, line, year ?? null, month ?? null)) as TraceResult);
}

export function sensitivity(
  text: string,
  spec: { x: string; y: string; x_values: number[]; y_values: number[] },
): Outcome<SensitivityGrid> {
  return attempt(() => JSON.parse(wSens(text, JSON.stringify(spec))) as SensitivityGrid);
}

export const csv = (text: string, report: string): Outcome<string> => attempt(() => wCsv(text, report));
export const toYaml = (text: string): Outcome<string> => attempt(() => wYaml(text));
export const toJson = (text: string): Outcome<string> => attempt(() => wJson(text));
export const version = (): string => wVersion();

/** Reads a deal file's text (YAML or JSON) into the page's editable object. */
export function readDeal(text: string): Outcome<Deal> {
  const t = text.trimStart();
  if (t.startsWith("{")) return attempt(() => JSON.parse(text) as Deal);
  // YAML goes through the core, which validates it on the way to JSON.
  const j = toJson(text);
  return "ok" in j ? { ok: JSON.parse(j.ok) as Deal } : j;
}
