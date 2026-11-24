# proforma web page

The dark-mode page for proforma. Every number on it comes from the Rust core, compiled to WebAssembly and inlined into the bundle; the page only formats. Deal files are opened and saved in the browser, and the page makes no network request after it loads (its CSP sets `connect-src 'none'`).

## Build and run locally

Needs Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen` CLI 0.2.130 and Node 22.

```sh
cd web
npm ci
npm run wasm     # cargo build (wasm profile) + wasm-bindgen --target web into src/wasm/
npm run dev      # http://localhost:5173
npm run build    # tsc --noEmit (strict), vite build into dist/, then scripts/check-dist.mjs
npx vite preview # serves dist/
```

## Checks

```sh
node crates/wasm/tests/parity.mjs   # from the repo root: every deal in fixtures/deals and deals
                                    # through WASM, byte-identical to fixtures/expected/<name>.json
npm run bench                       # 200-lease, 10-year deal: run() < 50 ms, 10 x 10 sensitivity < 2 s
CHROME=/path/to/chrome npm run bench  # also loads dist/ in headless Chrome: timings, no requests after load
```
