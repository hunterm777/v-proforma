// The deal bar, always in view: name, type, price, going-in cap rate,
// levered IRR and equity multiple, from result.deal_bar.
import { h } from "../dom";
import { mult, pct, typeLabel, usd } from "../format";
import { store } from "../state";
import { traceYear } from "./audit";

const stat = (label: string, value: string, attrs: Record<string, string> = {}): HTMLElement =>
  h("div", { class: "stat" }, h("span", { class: "flabel" }, label), h("span", { class: `v${attrs["data-trace"] ? " click" : ""}`, ...attrs }, value));

export function dealBar(): HTMLElement {
  const bar = h("section", { class: "dealbar", "aria-label": "Deal summary", "aria-live": "polite" });
  store.subscribe(() => {
    const b = store.result?.deal_bar;
    bar.classList.toggle("stale", store.errors.length > 0);
    if (!b) {
      bar.replaceChildren(h("p", { class: "muted" }, "No result yet"));
      return;
    }
    const warn = store.errors.length ? h("span", { class: "badge warn" }, `${store.errors.length} input error${store.errors.length === 1 ? "" : "s"}`) : "";
    bar.replaceChildren(
      h("div", { class: "stat name" }, h("span", { class: "flabel" }, typeLabel(b.type)), h("span", { class: "deal-name" }, b.name)),
      stat("Price", usd(b.price)),
      stat("Going-in cap", pct(b.going_in_cap), { ...traceYear("noi", 1), title: "Year-1 NOI / price; click for the year-1 NOI trail" }),
      stat("Levered IRR", pct(b.levered_irr)),
      stat("Equity multiple", mult(b.equity_multiple)),
      warn,
    );
  });
  return bar;
}
