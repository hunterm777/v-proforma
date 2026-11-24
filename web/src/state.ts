// The page's single source of state: one deal object, the same document as
// the deal file. Every edit re-runs the core on that deal.
import * as core from "./core";
import type { Deal, Result } from "./types";

/** `structural` means the inputs must be redrawn (rows added, a type changed). */
type Listener = (structural: boolean) => void;

class Store {
  deal: Deal = { property: { name: "", type: "office", start_date: "2027-01-01", hold_months: 12 }, capital: { price: 0 }, exit: { cap_rate: 0 } };
  /** The last successful run; kept while the inputs are invalid. */
  result: Result | null = null;
  raw: string | null = null;
  /** The core's errors for the current inputs; empty when the run succeeded. */
  errors: string[] = [];
  /** Counts runs, so views can tell when their data predates the deal. */
  rev = 0;
  fileName = "deal";
  private listeners: Listener[] = [];

  /** The deal as the text the core reads. */
  text(): string {
    return JSON.stringify(this.deal);
  }

  subscribe(fn: Listener): void {
    this.listeners.push(fn);
  }

  /** Replaces the whole deal (load, example pick) and redraws everything. */
  load(deal: Deal, fileName: string): void {
    this.deal = deal;
    this.fileName = fileName;
    this.result = null;
    this.raw = null;
    this.run(true);
  }

  /** Applies an edit and re-runs. */
  update(edit: (d: Deal) => void, structural = false): void {
    edit(this.deal);
    this.run(structural);
  }

  run(structural: boolean): void {
    const t0 = performance.now();
    const r = core.run(this.text());
    const t1 = performance.now();
    if ("ok" in r) {
      this.result = r.ok.result;
      this.raw = r.ok.raw;
      this.errors = [];
    } else {
      // Validation joins every error with "; ".
      this.errors = r.err.split("; ");
    }
    this.rev++;
    for (const fn of this.listeners) fn(structural);
    // Read by the browser benchmark.
    document.body.dataset["runMs"] = (t1 - t0).toFixed(2);
    document.body.dataset["renderMs"] = (performance.now() - t0).toFixed(2);
    document.body.dataset["rev"] = String(this.rev);
  }
}

export const store = new Store();
