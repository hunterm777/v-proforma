// The main area: one tab per report. Only the open tab is drawn, on every run.
import { h, tabBar } from "../../dom";
import { store } from "../../state";
import type { Result } from "../../types";
import { cashFlowView } from "./cashflow";
import { debtView } from "./debt";
import { rentRollView } from "./rentroll";
import { returnsView } from "./returns";
import { markStale, sensitivityView } from "./sensitivity";
import { waterfallView } from "./waterfall";

const TABS: readonly (readonly [string, string, (r: Result) => HTMLElement])[] = [
  ["cashflow", "Cash flow", cashFlowView],
  ["rentroll", "Rent roll summary", rentRollView],
  ["debt", "Debt", debtView],
  ["returns", "Returns", returnsView],
  ["waterfall", "Waterfall", waterfallView],
  ["sensitivity", "Sensitivity", () => sensitivityView()],
];

export function outputsPanel(): HTMLElement {
  let active = "cashflow";
  const bar = h("div");
  const body = h("div", { class: "panel-body" });
  const el = h("main", { class: "panel outputs" }, bar, body);

  function draw(structural = false): void {
    if (structural) body.dataset["tab"] = "";
    bar.replaceChildren(
      tabBar(TABS.map(([id, label]) => [id, label] as const), active, (id) => {
        active = id;
        draw(true);
      }),
    );
    el.classList.toggle("stale", store.errors.length > 0);
    const r = store.result;
    const tab = TABS.find(([id]) => id === active) ?? TABS[0];
    if (!tab) return;
    if (!r) {
      body.replaceChildren(h("p", { class: "banner" }, "Fix the inputs on the left to see results."));
      return;
    }
    // The sensitivity inputs keep their own state; redrawing them on every
    // edit would drop focus, so that tab redraws only when opened.
    if (active === "sensitivity" && body.dataset["tab"] === "sensitivity") return markStale();
    body.dataset["tab"] = active;
    body.replaceChildren(tab[2](r));
  }

  store.subscribe(draw);
  return el;
}
