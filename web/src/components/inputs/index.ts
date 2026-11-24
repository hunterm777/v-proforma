// The left panel: one tab per block of the deal file, with the core's
// validation errors shown above it while the inputs don't run.
import { h, tabBar } from "../../dom";
import { store } from "../../state";
import type { Deal } from "../../types";
import { markTableErrors } from "../table";
import { capitalTab } from "./capital";
import { debtTab } from "./debt";
import { equityTab } from "./equity";
import { expensesTab } from "./expenses";
import { exitTab, leasingTab, propertyTab } from "./property";
import { rentRollTab } from "./rentroll";

type Changed = (structural?: boolean) => void;

const TABS: readonly (readonly [string, string, (d: Deal, c: Changed) => HTMLElement])[] = [
  ["property", "Property", propertyTab],
  ["rentroll", "Rent roll", rentRollTab],
  ["leasing", "Leasing", leasingTab],
  ["expenses", "Expenses", expensesTab],
  ["capital", "Capital", capitalTab],
  ["debt", "Debt", debtTab],
  ["exit", "Exit", exitTab],
  ["equity", "Equity", equityTab],
];

export function inputsPanel(): HTMLElement {
  let active = "property";
  const bar = h("div");
  const body = h("div", { class: "panel-body" });
  const errors = h("div", { class: "errors hidden", role: "alert" });
  const changed: Changed = (structural = false) => store.run(structural);

  function draw(): void {
    bar.replaceChildren(tabBar(TABS.map(([id, label]) => [id, label] as const), active, (id) => ((active = id), draw())));
    const tab = TABS.find(([id]) => id === active) ?? TABS[0];
    if (tab) body.replaceChildren(tab[2](store.deal, changed));
    markTableErrors(store.errors);
  }

  function showErrors(): void {
    errors.classList.toggle("hidden", store.errors.length === 0);
    errors.replaceChildren(
      h("p", {}, "These inputs don't run yet; the results show the last inputs that did."),
      h("ul", {}, ...store.errors.map((e) => h("li", {}, e))),
    );
    markTableErrors(store.errors);
  }

  store.subscribe((structural) => {
    if (structural) draw();
    showErrors();
  });
  return h("aside", { class: "panel inputs", "aria-label": "Deal inputs" }, bar, errors, body);
}
