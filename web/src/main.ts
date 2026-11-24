import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "./styles/tokens.css";
import "./styles/app.css";

import * as core from "./core";
import { h } from "./dom";
import { auditPanel, onClick } from "./components/audit";
import { dealBar } from "./components/dealbar";
import { header, loadExample } from "./components/files";
import { inputsPanel } from "./components/inputs";
import { outputsPanel } from "./components/outputs";

core.init();

const FIRST = "office";
const app = document.getElementById("app");
if (app) {
  app.replaceChildren(
    header(FIRST),
    dealBar(),
    h("div", { class: "workspace" }, inputsPanel(), outputsPanel()),
    auditPanel(),
    h("footer", {}, h("p", {}, `proforma engine ${core.version()}`)),
  );
  document.addEventListener("click", onClick);
  loadExample(FIRST);
}
