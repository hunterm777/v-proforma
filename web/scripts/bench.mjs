// The PRD's speed targets on the WASM core: a 200-lease, 10-year monthly
// model runs in under 50 ms, and a 10 x 10 sensitivity table in under 2 s.
// Times both in Node on the same WASM the page bundles (run `npm run wasm`
// first). With a headless Chrome (CHROME=/path/to/chrome, needs
// puppeteer-core) it also loads the built page (`npm run build` first),
// checks that nothing is requested after load and no CSP violation or error
// shows, and times the same model and table in the browser.
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { extname, join, resolve } from "node:path";
import { benchDeal } from "./bench-deal.mjs";

const here = import.meta.dirname;
const RUN_MS = 50;
const SENS_MS = 2000;
const deal = JSON.stringify(benchDeal());
const spec = {
  x: "exit.cap_rate",
  y: "market.rent_growth",
  x_values: [0.06, 0.0625, 0.065, 0.0675, 0.07, 0.0725, 0.075, 0.0775, 0.08, 0.0825],
  y_values: [0.015, 0.02, 0.025, 0.03, 0.035, 0.04, 0.045, 0.05, 0.055, 0.06],
};
const failures = [];
const r1 = (x) => Math.round(x * 10) / 10;
const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];

// ---- Node ----
const glue = await import(join(here, "../src/wasm/proforma_wasm.js"));
glue.initSync({ module: readFileSync(join(here, "../src/wasm/proforma_wasm_bg.wasm")) });
const first = JSON.parse(glue.run(deal));
const leases = benchDeal().rent_roll.length;
const rollovers = first.entries.filter((e) => e.line === "base_rent" && e.detail.startsWith("rollover")).length;
for (let i = 0; i < 3; i++) glue.run(deal);
const runs = [];
for (let i = 0; i < 25; i++) {
  const t0 = performance.now();
  JSON.parse(glue.run(deal));
  runs.push(performance.now() - t0);
}
const sens = [];
for (let i = 0; i < 3; i++) {
  const t0 = performance.now();
  const g = JSON.parse(glue.sensitivity(deal, JSON.stringify(spec)));
  sens.push(performance.now() - t0);
  if (g.levered_irr.length !== 10 || g.levered_irr.some((row) => row.length !== 10)) failures.push("sensitivity grid is not 10 x 10");
}
const node = { run_ms_median: r1(median(runs)), run_ms_max: r1(Math.max(...runs)), sensitivity_10x10_ms_median: r1(median(sens)) };
console.log(`deal: ${leases} leases, ${first.exit_month}-month hold, ${first.model_months} model months, ${first.entries.length} ledger entries (${rollovers} rollover base-rent entries), result ${(glue.run(deal).length / 1024).toFixed(0)} KB`);
console.log(`node  run():           median ${node.run_ms_median} ms, max ${node.run_ms_max} ms (target < ${RUN_MS} ms)`);
console.log(`node  sensitivity 10x10: median ${node.sensitivity_10x10_ms_median} ms (target < ${SENS_MS} ms)`);
if (node.run_ms_median >= RUN_MS) failures.push(`node run() median ${node.run_ms_median} ms is over ${RUN_MS} ms`);
if (node.sensitivity_10x10_ms_median >= SENS_MS) failures.push(`node sensitivity ${node.sensitivity_10x10_ms_median} ms is over ${SENS_MS} ms`);

// ---- Browser ----
const candidates = [process.env.CHROME, "/usr/bin/google-chrome", "/usr/bin/google-chrome-stable", "/usr/bin/chromium", "/usr/bin/chromium-browser"];
const chrome = candidates.find((p) => p && existsSync(p));
if (process.env.CHROME && !existsSync(process.env.CHROME)) failures.push(`CHROME=${process.env.CHROME} does not exist`);
if (!chrome) {
  console.log("browser: no headless Chrome found (set CHROME=/path/to/chrome); browser timing and the no-requests check did not run");
} else {
  const { default: puppeteer } = await import("puppeteer-core");
  const dist = resolve(here, "../dist");
  const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2", ".woff": "font/woff" };
  const server = createServer((req, res) => {
    const path = join(dist, decodeURIComponent((req.url ?? "/").split("?")[0]).replace(/\/$/, "/index.html"));
    if (!path.startsWith(dist) || !existsSync(path)) return res.writeHead(404).end();
    res.writeHead(200, { "content-type": types[extname(path)] ?? "application/octet-stream" }).end(readFileSync(path));
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  const url = `http://127.0.0.1:${server.address().port}/`;
  const args = process.env.CHROME_ARGS ? JSON.parse(process.env.CHROME_ARGS) : ["--no-sandbox"];
  const browser = await puppeteer.launch({ executablePath: chrome, args, headless: true });
  const page = await browser.newPage();
  await page.setViewport({ width: 1600, height: 1000 });
  const problems = [];
  const requests = [];
  let loaded = false;
  page.on("console", (m) => m.type() === "error" && problems.push(`console: ${m.text()}`));
  page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
  page.on("request", (r) => loaded && requests.push(r.url()));
  await page.evaluateOnNewDocument(() => {
    document.addEventListener("securitypolicyviolation", (e) => console.error(`CSP ${e.violatedDirective} ${e.blockedURI}`));
  });
  await page.goto(url, { waitUntil: "networkidle0" });
  loaded = true;

  // Open the benchmark deal through the page's own file picker.
  const file = join(mkdtempSync(join(tmpdir(), "proforma-bench-")), "bench_deal.json");
  writeFileSync(file, deal);
  const rev0 = await page.evaluate(() => Number(document.body.dataset.rev));
  const input = await page.$('input[type="file"]');
  await input.uploadFile(file);
  await page.waitForFunction((r) => Number(document.body.dataset.rev) > r, {}, rev0);
  const name = await page.evaluate(() => document.querySelector(".deal-name")?.textContent);
  if (name !== "Benchmark Tower") problems.push(`the page did not load the benchmark deal (shows ${name})`);

  // Each edit of the exit cap rate re-runs the core and redraws the open tab.
  const browserStats = await page.evaluate(async () => {
    [...document.querySelectorAll(".inputs .tabbar button")].find((b) => b.textContent === "Exit")?.click();
    const cap = [...document.querySelectorAll(".inputs label.field")].find((l) => l.textContent?.startsWith("Exit cap rate"))?.querySelector("input");
    const run = [];
    const total = [];
    for (let i = 0; i < 25; i++) {
      cap.value = String(6.5 + (i % 5) * 0.25);
      cap.dispatchEvent(new Event("input", { bubbles: true }));
      run.push(Number(document.body.dataset.runMs));
      total.push(Number(document.body.dataset.renderMs));
      await new Promise((r) => requestAnimationFrame(r));
    }
    const med = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];
    return { run: med(run.slice(3)), total: med(total.slice(3)) };
  });

  // The 10 x 10 table through the Sensitivity tab.
  const sensMs = await page.evaluate(async (s) => {
    [...document.querySelectorAll(".outputs .tabbar button")].find((b) => b.textContent === "Sensitivity")?.click();
    const field = (label) => [...document.querySelectorAll(".outputs label.field")].find((l) => l.textContent?.startsWith(label))?.querySelector("input");
    const set = (label, v) => {
      const el = field(label);
      el.value = v;
      el.dispatchEvent(new Event("input", { bubbles: true }));
    };
    set("Across (x) path", s.x);
    set("x values", s.x_values.join(", "));
    set("Down (y) path", s.y);
    set("y values", s.y_values.join(", "));
    const t0 = performance.now();
    [...document.querySelectorAll(".outputs button")].find((b) => b.textContent === "Run")?.click();
    const ms = performance.now() - t0;
    const tables = document.querySelectorAll(".outputs table.sens");
    return { ms, tables: tables.length, cells: tables[0]?.querySelectorAll("tbody td").length ?? 0 };
  }, spec);
  await page.screenshot({ path: process.env.SHOT ?? join(tmpdir(), "proforma.png"), fullPage: false });
  await browser.close();
  server.close();

  const b = { run_ms_median: r1(browserStats.run), run_and_redraw_ms_median: r1(browserStats.total), sensitivity_10x10_ms: r1(sensMs.ms) };
  console.log(`chrome run():           median ${b.run_ms_median} ms (run + redraw ${b.run_and_redraw_ms_median} ms; target < ${RUN_MS} ms)`);
  console.log(`chrome sensitivity 10x10: ${b.sensitivity_10x10_ms} ms incl. drawing (target < ${SENS_MS} ms)`);
  console.log(`chrome requests after load: ${requests.length}; problems: ${problems.length}`);
  for (const p of problems) console.log(`  ${p}`);
  for (const r of requests) console.log(`  request ${r}`);
  if (requests.length) failures.push(`${requests.length} request(s) after load`);
  if (problems.length) failures.push(`${problems.length} console error(s), page error(s) or CSP violation(s)`);
  if (sensMs.tables !== 2 || sensMs.cells !== 100) failures.push("the page did not draw two 10 x 10 sensitivity tables");
  if (b.run_ms_median >= RUN_MS) failures.push(`chrome run() median ${b.run_ms_median} ms is over ${RUN_MS} ms`);
  if (b.sensitivity_10x10_ms >= SENS_MS) failures.push(`chrome sensitivity ${b.sensitivity_10x10_ms} ms is over ${SENS_MS} ms`);
}

if (failures.length) {
  console.error(`bench: FAIL\n  ${failures.join("\n  ")}`);
  process.exit(1);
}
console.log("bench: ok");
