// Header: example deals, load and save the deal file, and the exports (CSV
// per report, the full result as JSON). Files are read and written in the
// browser; nothing is uploaded.
import * as core from "../core";
import { EXAMPLES } from "../deals";
import { download, h } from "../dom";
import { store } from "../state";
import { resetSensitivity } from "./outputs/sensitivity";

const REPORTS = [
  ["annual", "Annual cash flow"],
  ["monthly", "Monthly cash flow"],
  ["rent_roll", "Rent roll summary"],
  ["tenants", "Tenant detail"],
  ["debt", "Debt schedule"],
  ["returns", "Returns"],
  ["waterfall", "Waterfall"],
  ["entries", "Ledger entries"],
] as const;

const msg = h("p", { class: "file-msg", role: "status" });
const say = (text: string, bad = false): void => {
  msg.textContent = text;
  msg.classList.toggle("bad", bad);
};

/** Loads an example deal; returns false if the core refused it. */
export function loadExample(id: string): boolean {
  const ex = EXAMPLES.find((e) => e.id === id);
  if (!ex) return false;
  const d = core.readDeal(ex.yaml);
  if ("err" in d) {
    say(d.err, true);
    return false;
  }
  resetSensitivity();
  store.load(d.ok, `${ex.id}_example`);
  say("");
  return true;
}

function loadButton(): HTMLElement {
  const input = h("input", { type: "file", accept: ".yaml,.yml,.json,application/json,text/yaml", class: "hidden", "aria-label": "Deal file" });
  input.addEventListener("change", async () => {
    const f = input.files?.[0];
    input.value = "";
    if (!f) return;
    const d = core.readDeal(await f.text());
    if ("err" in d) return say(`${f.name} could not be read: ${d.err}`, true);
    resetSensitivity();
    store.load(d.ok, f.name.replace(/\.(ya?ml|json)$/i, ""));
    say(`Loaded ${f.name}`);
  });
  const b = h("button", { type: "button" }, "Open deal file");
  b.addEventListener("click", () => input.click());
  return h("span", {}, b, input);
}

function saveButton(label: string, ext: "yaml" | "json"): HTMLElement {
  const b = h("button", { type: "button" }, label);
  b.addEventListener("click", () => {
    const t = ext === "yaml" ? core.toYaml(store.text()) : core.toJson(store.text());
    if ("err" in t) return say(`Fix the inputs before saving: ${t.err}`, true);
    download(`${store.fileName}.${ext}`, t.ok, ext === "yaml" ? "text/yaml" : "application/json");
    say(`Saved ${store.fileName}.${ext} to your machine`);
  });
  return b;
}

function exports(): HTMLElement {
  const sel = h("select", { "aria-label": "Report" }, ...REPORTS.map(([v, l]) => h("option", { value: v }, l)));
  const csvBtn = h("button", { type: "button", class: "ghost" }, "CSV");
  csvBtn.addEventListener("click", () => {
    const t = core.csv(store.text(), sel.value);
    if ("err" in t) return say(`Fix the inputs before exporting: ${t.err}`, true);
    download(`${store.fileName}_${sel.value}.csv`, t.ok, "text/csv");
  });
  const jsonBtn = h("button", { type: "button", class: "ghost" }, "Result JSON");
  jsonBtn.addEventListener("click", () => {
    if (store.errors.length || !store.raw) return say("Fix the inputs before exporting the result.", true);
    download(`${store.fileName}_result.json`, store.raw, "application/json");
  });
  return h("div", { class: "group" }, h("span", { class: "flabel" }, "Export"), sel, csvBtn, jsonBtn);
}

export function header(initial: string): HTMLElement {
  const pick = h("select", { "aria-label": "Example deal" }, ...EXAMPLES.map((e) => h("option", { value: e.id, selected: e.id === initial }, e.label)));
  pick.addEventListener("change", () => loadExample(pick.value));
  return h(
    "header",
    { class: "top" },
    h("div", { class: "brand" }, h("h1", {}, "proforma"), h("p", { class: "tagline" }, "Open-source CRE underwriting. Your deal file never leaves this page.")),
    h(
      "div",
      { class: "file-actions" },
      h("div", { class: "group" }, h("span", { class: "flabel" }, "Example"), pick),
      h("div", { class: "group" }, loadButton(), saveButton("Save YAML", "yaml"), saveButton("Save JSON", "json")),
      exports(),
    ),
    msg,
  );
}
