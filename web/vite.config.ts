import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";

// Inlines a .wasm file as base64 text (`import b64 from "x.wasm?b64"`), so the
// page compiles its core from the bundle and never fetches it.
function wasmBase64(): Plugin {
  return {
    name: "wasm-base64",
    enforce: "pre",
    load(id) {
      if (!id.endsWith(".wasm?b64")) return null;
      const bytes = readFileSync(id.slice(0, -"?b64".length));
      return `export default ${JSON.stringify(bytes.toString("base64"))};`;
    },
  };
}

// The dev server injects styles inline, which the page's CSP forbids; the CSP
// meta is dropped while serving and kept in the build.
function devCsp(): Plugin {
  return {
    name: "dev-csp",
    apply: "serve",
    transformIndexHtml: (html) => html.replace(/\s*<meta\s+http-equiv="Content-Security-Policy"[^>]*>/, ""),
  };
}

// The page is one static bundle: the WASM core and the example deals are
// inlined into the script and the fonts ship from the same origin, so nothing
// is fetched after load.
export default defineConfig({
  base: "./",
  plugins: [wasmBase64(), devCsp()],
  // The example deals live in the repo's deals/ folder.
  server: { fs: { allow: [resolve(import.meta.dirname, "..")] } },
  build: {
    target: "es2022",
    assetsInlineLimit: 0,
    modulePreload: false,
    sourcemap: false,
    // The core is inlined as base64, so the one script is large by design.
    chunkSizeWarningLimit: 4096,
  },
});
