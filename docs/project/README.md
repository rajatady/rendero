# Rendero Project Documentation

A portable rendering engine that lets unmodified React/Vue apps run natively on macOS, Windows, Linux, iOS, and Android -- without a WebView, without Electron, without rewriting your app.

On web production builds: zero overhead (it's just a normal web app).
For parity/debugging builds: the same app can also run through the DOM shim +
WASM engine in the browser.
On native: a DOM shim routes framework calls to a Rust rendering engine.

---

## Table of Contents

| Document | What it covers |
|----------|---------------|
| [Vision and Architecture](vision-and-architecture.md) | Motivation, competitive landscape, three-layer architecture, provider pattern, platform matrix, performance model, roadmap |
| [Core Engine](core-engine.md) | Scene graph, node types, document model, Taffy layout, provider traits |
| [Renderer](renderer.md) | Tile-based CPU rasterizer, scene building, text rendering, effects, SVG export |
| [DOM Shim](dom-shim.md) | JS layer that intercepts react-dom/Vue calls, CSS parsing, engine bridges |
| [Native Shell](native-shell.md) | Pure Rust app (winit + QuickJS + softbuffer), event loop, JS runtime trait |
| [WASM and CRDT](wasm-and-crdt.md) | WASM bindings, CRDT collaboration, Figma import |
| [Current Status](current-status.md) | What renders today, blocking issues, short-term runnable path, bugs found and fixed |
| [Commands](commands.md) | Build, run, test, screenshot, development workflow |

---

## Current State (as of 2026-03-31)

### Accuracy

**60.98%** layout accuracy on the Apple demo (browser oracle vs Rendero WASM path, 107 elements).

Synthetic layout corpus: **83.19%**.

7-site corpus with ground truth at 3 viewports: apple-macbook-pro, fin, github, gumroad, hacker-news, linear, tailadmin.

### What Works

| Feature | Status | Notes |
|---------|--------|-------|
| React on browser DOM | Working | Full Apple website, perfect rendering |
| Vue on browser DOM | Working | Same Apple website |
| React on WASM canvas | Working | Full Apple page with gradients, layered benchmark pipeline, 60.98% Apple accuracy |
| React on native macOS | Working | Full Apple page renders through the pure Rust shell, headless mode, system fonts wired |
| Taffy layout engine | Working | 12 tests, flexbox + justify/wrap/margin/position/min-max |
| Provider architecture | Working | LayoutEngine, TextMeasurer, CssParser, FontResolver, GlyphRasterizer |
| Native FFI (C-ABI) | Working | 22 exported C functions, tested with Swift shell |
| Pure Rust native shell | Working | winit + QuickJS + softbuffer, headless PPM dump |
| Lightning CSS parser | Working | 25 tests, CssParser trait, not yet used for visual output |
| Parley text measurer | Working | 7 tests, system font discovery via Fontique |
| System font resolution | Working | fontdb resolves font families, renderer uses per-run fonts |
| Layout oracle loop | Working | Playwright captures browser ground truth, compares vs engine |
| Headless rendering | Working | `RENDERO_HEADLESS_DUMP` env var, PPM output |

### What Needs Work

| Feature | Status | What's Missing |
|---------|--------|----------------|
| Layout accuracy | 60.98% Apple / 83.19% synthetic | Remaining gaps are constrained feature-card text measurement, one residual top-level wrapper height drift, and native/browser visual parity |
| CSS font inheritance | Not started | Shim doesn't inherit fontFamily from parent |
| Text measurement (web) | Partial | `BrowserTextMeasurer` is the deliberate WASM oracle path; browser Rendero still carries a temporary JS-side eager text-size sync for some constrained text |
| Text measurement (native) | Partial | `ParleyTextMeasurer` is the intended path; remaining parity issues are now more about constrained wrapping than total collapse |
| Percentage resolution | Partial | Basic parent-relative width `%` support is wired; extend to height/min-max/flex-basis/positioned cases |
| Scroll on native | Partial | Shared scroll contract is wired, but live native parity still needs more validation and window capture |
| GPU rendering | Not started | CPU tiles work, wgpu backend planned |
| Vue on native | Not tested | Same shim, should work with minimal fixes |
| iOS/Android | Not started | Same Rust code, different cargo target |
| Click/touch events | Partial | winit MouseWheel handled, click not yet |

---

## Architecture Overview

```
YOUR APP (React, Vue, Svelte -- unmodified)
    |
react-dom / vue runtime (UNMODIFIED)
    |
DOM Shim (JS)                            <-- used on browser Rendero + native
    |                                        real browser DOM remains the oracle path
Engine Bridge (engine.js / engine-native.js)
    |
Rendero Engine (Rust)
    |-- rendero-core: nodes, layout (Taffy), properties, provider traits
    |-- rendero-renderer: tiles, rasterization, text (fontdue + fontdb)
    |-- rendero-css: CSS parsing (Lightning CSS)
    |-- rendero-text: text measurement (Parley + Fontique)
    |
Platform Backend (swappable)
    |-- Web: Canvas2D / WebGL2 / raster (WASM)
    |-- macOS/Win/Linux: softbuffer (CPU) / wgpu (GPU, future)
    |-- iOS/Android: winit + wgpu (future)
```

---

## Crate Map

| Crate | Type | Dependencies | Purpose |
|-------|------|-------------|---------|
| `rendero-core` | lib | glam, serde, taffy | Scene graph, nodes, layout, providers |
| `rendero-renderer` | lib | rendero-core, fontdue, fontdb | Tile rasterizer, text (system fonts), SVG |
| `rendero-css` | lib | rendero-core, lightningcss | CSS parsing, CssParser trait impl |
| `rendero-text` | lib | rendero-core, parley | Text measurement (Parley), FontResolver |
| `rendero-crdt` | lib | rendero-core, serde | CRDT operations for collaboration |
| `rendero-wasm` | cdylib | rendero-core, renderer, crdt, wasm-bindgen | WASM bindings |
| `rendero-native-ffi` | cdylib+staticlib | rendero-core, renderer | C-ABI for Swift/C++ |
| `rendero-native-shell` | bin | rendero-core, renderer, winit, rquickjs, softbuffer | Pure Rust native app |
| `rendero-fig-import` | lib | rendero-core | Figma .fig file parser |

---

## Quick Start

### Run Native App
```bash
cd /path/to/rendero
cargo run -p rendero-native-shell
```

### Run Web Demo
```bash
# Build WASM
wasm-pack build crates/wasm --target web --out-dir ../../pkg --out-name rendero

# Build JS bundles
cd docs/demos/dom-shim && node build.mjs && cd ../../..

# Start a no-cache dev server
serve -l tcp://127.0.0.1:5555 docs -C --no-etag -c ./serve.rendero.json
# Open http://127.0.0.1:5555/demos/dom-shim/
```

### Run Tests
```bash
cargo test -p rendero-core
```

---

## Key Design Decisions

| Decision | Why |
|----------|-----|
| Pure Rust native shell (not Swift) | Rust gives full control of the event loop, viewport sync, and native/browser parity work. |
| QuickJS (not JavaScriptCore) | Embeddable, has execute_pending_job() for event loop, compiles everywhere |
| Provider traits for everything | Swap implementations without touching API. Mock for testing. Fall back on edge cases. |
| Taffy for layout (not custom) | Full CSS flexbox + grid. Battle-tested. 0.6ms for 1000 nodes. |
| softbuffer for now (not wgpu) | Simpler. CPU pixel blit. Swap to wgpu later for GPU rendering. |
| DOM shim (not custom renderer) | react-dom/Vue work unmodified. Same shim surface is now benchmarked across browser Rendero and native, with layered artifacts for shim input, engine model, and engine layout. |
| IIFE bundles for native | QuickJS doesn't support ES modules. IIFE wraps everything in one function. |

---

## File Counts

| Component | Files | Lines |
|-----------|-------|-------|
| Core engine (Rust) | ~15 | ~3000 |
| Renderer (Rust) | ~10 | ~2500 |
| WASM bindings (Rust) | ~5 | ~6000 |
| Native shell (Rust) | 4 | ~600 |
| Native FFI (Rust) | 2 | ~500 |
| DOM shim (JS) | 10 | ~2000 |
| Demo apps (JSX/JS) | 7 | ~800 |
| CRDT (Rust) | ~5 | ~600 |
| **Total** | **~58** | **~16000** |
