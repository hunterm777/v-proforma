// Equity tab: LP/GP contribution shares and the distribution waterfall tiers.
import { checkField, h, numField } from "../../dom";
import type { Compounding, Deal, Tier } from "../../types";
import { numCol, put, selectCol, textCol } from "../cols";
import { editTable } from "../table";
import { fields } from "./property";

let kept: Deal["equity"];

const HURDLES = [
  ["", "none"],
  ["irr", "LP IRR"],
  ["multiple", "LP multiple"],
] as const;
const COMPOUNDING = [
  ["annual", "annual"],
  ["quarterly", "quarterly"],
  ["monthly", "monthly"],
  ["none", "none"],
] as const;
const SPLITS = [
  ["pro_rata", "pro rata"],
  ["shares", "LP / GP"],
] as const;

const shares = (t: Tier): { lp: number; gp: number } | undefined => (typeof t.split === "object" ? t.split : undefined);

function tiers(rows: Tier[], changed: () => void): HTMLElement {
  return editTable<Tier>({
    rows: () => rows,
    make: () => ({ name: `Tier ${rows.length + 1}`, split: "pro_rata" }),
    errorKey: (r, i) => `equity.tiers[${i}] (${r.name})`,
    addLabel: "Add tier",
    onChange: changed,
    cols: [
      textCol("Tier", (r) => r.name, (r, v) => (r.name = v ?? r.name), { required: true, wide: true }),
      selectCol<Tier>("Hurdle", () => HURDLES, (r) => r.hurdle?.type ?? "", (r, v) => {
        if (v === "") delete r.hurdle;
        else if (v !== r.hurdle?.type) r.hurdle = v === "irr" ? { type: "irr", rate: 0 } : { type: "multiple", multiple: 1 };
      }, true),
      numCol<Tier>("LP IRR %", (r) => (r.hurdle?.type === "irr" ? r.hurdle.rate : undefined), (r, v) => {
        if (r.hurdle?.type === "irr") r.hurdle.rate = v ?? r.hurdle.rate;
      }, { scale: 100, required: true, disabled: (r) => r.hurdle?.type !== "irr" }),
      numCol<Tier>("LP multiple", (r) => (r.hurdle?.type === "multiple" ? r.hurdle.multiple : undefined), (r, v) => {
        if (r.hurdle?.type === "multiple") r.hurdle.multiple = v ?? r.hurdle.multiple;
      }, { required: true, title: "x of LP contributions", disabled: (r) => r.hurdle?.type !== "multiple" }),
      {
        ...selectCol<Tier>("Compounding", () => COMPOUNDING, (r) => (r.hurdle?.type === "irr" ? (r.hurdle.compounding ?? "annual") : "annual"), (r, v) => {
          if (r.hurdle?.type === "irr") r.hurdle.compounding = v as Compounding;
        }),
        disabled: (r: Tier) => r.hurdle?.type !== "irr",
      },
      selectCol<Tier>("Split", () => SPLITS, (r) => (shares(r) ? "shares" : "pro_rata"), (r, v) => {
        if (v === "pro_rata") r.split = "pro_rata";
        else if (!shares(r)) r.split = { lp: 0.5, gp: 0.5 };
      }, true),
      numCol<Tier>("LP %", (r) => shares(r)?.lp, (r, v) => {
        const s = shares(r);
        if (s) s.lp = v ?? s.lp;
      }, { scale: 100, required: true, disabled: (r) => !shares(r) }),
      numCol<Tier>("GP %", (r) => shares(r)?.gp, (r, v) => {
        const s = shares(r);
        if (s) s.gp = v ?? s.gp;
      }, { scale: 100, required: true, disabled: (r) => !shares(r) }),
      numCol<Tier>("Until GP % of profit", (r) => r.until?.gp_share_of_profits, (r, v) => {
        put(r, "until", v === undefined ? undefined : { gp_share_of_profits: v });
      }, { scale: 100, title: "Catch-up: the tier runs until the GP has this share of all profit" }),
    ],
  });
}

export function equityTab(deal: Deal, changed: (structural?: boolean) => void): HTMLElement {
  const has = checkField("This deal has an LP / GP waterfall", deal.equity !== undefined, (on) => {
    if (on) deal.equity = kept ?? { contributions: { lp: 0.9, gp: 0.1 }, tiers: [{ name: "Pro rata", split: "pro_rata" }] };
    else {
      kept = deal.equity;
      delete deal.equity;
    }
    changed(true);
  });
  const q = deal.equity;
  if (!q) return h("section", { class: "tab-body" }, has);
  const c = q.contributions;
  return h(
    "section",
    { class: "tab-body" },
    has,
    h("h3", {}, "Contributions"),
    fields(
      numField({ label: "LP share", value: c.lp, scale: 100, suffix: "%", required: true, onValue: (v) => ((c.lp = v ?? c.lp), changed()) }),
      numField({ label: "GP share", value: c.gp, scale: 100, suffix: "%", required: true, onValue: (v) => ((c.gp = v ?? c.gp), changed()) }),
    ),
    h("h3", {}, "Distribution tiers"),
    tiers(q.tiers, changed),
    h("p", { class: "note" }, "Every distribution runs through the tiers in order; the last tier takes what is left."),
  );
}
