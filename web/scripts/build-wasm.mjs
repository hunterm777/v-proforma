// Builds the WASM core and writes the wasm-bindgen glue to src/wasm/.
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: "inherit" });
run("cargo", ["build", "-p", "proforma-wasm", "--target", "wasm32-unknown-unknown", "--profile", "wasm"]);
run("wasm-bindgen", [
  "target/wasm32-unknown-unknown/wasm/proforma_wasm.wasm",
  "--out-dir", "web/src/wasm", "--target", "web",
  "--omit-default-module-path", "--remove-name-section", "--remove-producers-section",
]);
