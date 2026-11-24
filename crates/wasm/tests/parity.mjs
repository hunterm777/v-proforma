// Runs every fixture and example deal through the WASM build (Node target)
// and requires the result JSON to be byte-identical to its golden file
// (fixtures/expected/<name>.json, written by the native build).
// Usage: node crates/wasm/tests/parity.mjs [path to a wasm-bindgen nodejs pkg]
// With no path it builds one into crates/wasm/pkg-node (gitignored).
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { join, parse, resolve } from "node:path";

const root = resolve(import.meta.dirname, "../../..");
let pkg = process.argv[2] ? resolve(process.argv[2]) : null;
if (!pkg) {
  pkg = join(root, "crates/wasm/pkg-node");
  const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: "inherit" });
  run("cargo", ["build", "-p", "proforma-wasm", "--target", "wasm32-unknown-unknown", "--profile", "wasm"]);
  run("wasm-bindgen", ["target/wasm32-unknown-unknown/wasm/proforma_wasm.wasm", "--target", "nodejs", "--out-dir", pkg]);
}
const pf = createRequire(import.meta.url)(join(pkg, "proforma_wasm.js"));

const deals = [];
for (const dir of ["fixtures/deals", "deals"]) {
  for (const f of readdirSync(join(root, dir))) {
    if (/\.(ya?ml|json)$/.test(f)) deals.push(join(root, dir, f));
  }
}
deals.sort();

let failed = 0;
for (const path of deals) {
  const name = parse(path).name;
  const goldenPath = join(root, "fixtures/expected", `${name}.json`);
  let want;
  try {
    want = readFileSync(goldenPath);
  } catch {
    failed++;
    console.error(`FAIL ${name}: no golden file ${goldenPath}`);
    continue;
  }
  let out;
  try {
    out = pf.run(readFileSync(path, "utf8"));
  } catch (e) {
    failed++;
    console.error(`FAIL ${name}: run threw ${e}`);
    continue;
  }
  // The native build writes the result followed by one newline.
  const got = Buffer.from(`${out}\n`, "utf8");
  if (got.equals(want)) {
    console.log(`ok   ${name} (${want.length} bytes)`);
    continue;
  }
  failed++;
  let i = 0;
  while (i < got.length && i < want.length && got[i] === want[i]) i++;
  const ctx = (b) => b.subarray(Math.max(0, i - 60), i + 60).toString("utf8");
  console.error(`FAIL ${name}: first difference at byte ${i} (wasm ${got.length} bytes, golden ${want.length})`);
  console.error(`  wasm:   ...${ctx(got)}...`);
  console.error(`  golden: ...${ctx(want)}...`);
}

if (failed || deals.length === 0) {
  console.error(`WASM parity: ${failed} of ${deals.length} deals differ`);
  process.exit(1);
}
console.log(`WASM parity: ${deals.length} deals byte-identical to their golden files`);
