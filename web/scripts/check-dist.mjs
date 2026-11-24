// Checks the built page for the PRD's offline rules: a CSP that forbids
// network requests (connect-src 'none'), no remote URL anywhere in the
// bundle (scripts, styles, fonts), and no network API in the scripts.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { gzipSync } from "node:zlib";

const dist = resolve(import.meta.dirname, "../dist");
const fail = (m) => {
  console.error(`check-dist: ${m}`);
  process.exit(1);
};

const html = readFileSync(join(dist, "index.html"), "utf8");
const csp = /<meta\s+http-equiv="Content-Security-Policy"\s+content="([^"]+)"/.exec(html)?.[1];
if (!csp) fail("index.html has no Content-Security-Policy meta");
for (const rule of ["default-src 'none'", "connect-src 'none'", "font-src 'self'", "style-src 'self'"]) {
  if (!csp.includes(rule)) fail(`CSP must include ${rule}`);
}

const files = [];
const walk = (d) =>
  readdirSync(d).forEach((f) => {
    const p = join(d, f);
    statSync(p).isDirectory() ? walk(p) : files.push(p);
  });
walk(dist);
let total = 0;
let fonts = 0;
for (const f of files) {
  total += statSync(f).size;
  if (/\.woff2?$/.test(f)) fonts++;
  if (!/\.(html|js|css)$/.test(f)) continue;
  const text = readFileSync(f, "utf8");
  const url = /(?:https?:)?\/\/[a-z0-9.-]+\.[a-z]{2,}[/"')\s]/i.exec(text.replace(/http-equiv/g, ""));
  if (url) fail(`${f} references a remote URL: ${url[0]}`);
  if (/\.css$/.test(f) && /@import/.test(text)) fail(`${f} imports another stylesheet`);
  for (const m of text.matchAll(/(fetch\(|XMLHttpRequest|WebSocket|EventSource|sendBeacon|import\()/g)) {
    fail(`${f} contains ${m[0]}`);
  }
}
if (fonts === 0) fail("no bundled font files in dist");

const wasm = readFileSync(resolve(import.meta.dirname, "../src/wasm/proforma_wasm_bg.wasm"));
const gz = gzipSync(wasm, { level: 9 }).length;
console.log(
  `check-dist: CSP ok (connect-src 'none'); no remote URLs; ${fonts} bundled font files; ` +
    `WASM ${(wasm.length / 1024).toFixed(0)} KB (${(gz / 1024).toFixed(0)} KB gzipped); ${files.length} files, ${(total / 1024).toFixed(0)} KB in dist`,
);
