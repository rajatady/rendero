# Rendero Project Documentation

A portable rendering engine that lets unmodified React/Vue apps run natively on macOS, Windows, Linux, iOS, and Android -- without a WebView, without Electron, without rewriting your app.

On web: zero overhead (it's just a normal web app).
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

**19.2%** layout accuracy (WASM web path, Apple demo, 107 elements, browser as oracle).

7-site corpus with ground truth at 3 viewports: apple-macbook-pro, fin, github, gumroad, hacker-news, linear, tailadmin.

### What Works

| Feature | Status | Notes |
|---------|--------|-------|
| React on browser DOM | Working | Full Apple website, perfect rendering |
| Vue on browser DOM | Working | Same Apple website |
| React on WASM canvas | Working | Full Apple page with gradients, 19.2% layout accuracy |
| React on native macOS | Working | Full Apple page renders, headless mode, system fonts wired |
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
| Layout accuracy | 19.2% | 20-item fix list in CLAUDE.md |
| CSS font inheritance | Not started | Shim doesn't inherit fontFamily from parent |
| Text measurement (web) | Partial | canvas.measureText working but font string needs work |
| Text measurement (native) | Heuristic | Uses `len * fontSize * 0.65`, Parley ready but not wired in |
| Percentage resolution | Partial | style.js resolves % against viewport, should pass to Taffy |
| Scroll on native | Partial | Camera movement wired, content height clamping issue |
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
DOM Shim (JS, ~2000 LOC)                 <-- only on native
    |                                         on web: bypassed entirely
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

# Start server
cd docs && python3 -m http.server 5555
# Open http://localhost:5555/demos/dom-shim/
```

### Run Tests
```bash
cargo test -p rendero-core
```

---

## Key Design Decisions

| Decision | Why |
|----------|-----|
| Pure Rust native shell (not Swift) | Swift's AppKit run loop conflicts with JS event loop. Rust gives full control. Cross-platform. |
| QuickJS (not JavaScriptCore) | Embeddable, has execute_pending_job() for event loop, compiles everywhere |
| Provider traits for everything | Swap implementations without touching API. Mock for testing. Fall back on edge cases. |
| Taffy for layout (not custom) | Full CSS flexbox + grid. Battle-tested. 0.6ms for 1000 nodes. |
| softbuffer for now (not wgpu) | Simpler. CPU pixel blit. Swap to wgpu later for GPU rendering. |
| DOM shim (not custom renderer) | react-dom/Vue work unmodified. Zero framework-specific code. npm packages just work. |
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
