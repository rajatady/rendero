## Rendero

A portable rendering engine that lets unmodified React/Vue apps run natively on macOS, Windows, Linux, iOS, and Android — without a WebView, without Electron, without rewriting your app. On web production builds: zero overhead (it's just a normal web app). For parity/debugging, the same app can also run through the DOM shim + WASM engine in the browser. On native: the DOM shim routes framework calls to a Rust rendering engine.

### Commands

```sh
# Build
cargo build -p rendero-native-shell              # native app
cargo build -p rendero-css                        # CSS parser crate
cargo build -p rendero-text                       # text measurement crate
wasm-pack build crates/wasm --target web --out-dir ../../pkg --out-name rendero  # WASM
cd docs/demos/dom-shim && node build.mjs          # JS bundles (React/Vue, web/native)

# Run
cargo run -p rendero-native-shell                 # windowed app (1440x900 baseline)
RENDERO_HEADLESS_DUMP=/tmp/out.ppm RENDERO_HEADLESS_WIDTH=1440 RENDERO_HEADLESS_HEIGHT=3400 cargo run -p rendero-native-shell  # headless render
serve -l tcp://127.0.0.1:5555 docs -C --no-etag -c ./serve.rendero.json  # no-cache web dev server → http://127.0.0.1:5555/demos/dom-shim/

# Test
cargo test -p rendero-core                        # 12 layout tests
cargo test -p rendero-css                         # 25 CSS parser tests
cargo test -p rendero-text                        # 7 text measurement tests

# Accuracy (oracle loop — browser is ground truth)
python3 scripts/capture-ground-truth.py http://localhost:5555/demos/dom-shim/ accuracy/apple-web.json
python3 scripts/capture-engine-truth.py http://localhost:5555/demos/dom-shim/ accuracy/apple-engine.json
python3 scripts/capture-native-truth.py http://localhost:5555/demos/dom-shim/ accuracy/apple-native.json
python3 scripts/compare-layout.py accuracy/apple-web.json accuracy/apple-engine.json accuracy/apple-comparison.json
python3 scripts/compare-runtime-triple.py accuracy/apple-web.json accuracy/apple-engine.json accuracy/apple-native.json accuracy/apple-runtime-comparison.json
scripts/run_accuracy_suite.sh
```

### Architecture

```
YOUR APP (React, Vue, Svelte — unmodified)
    |
react-dom / vue runtime (UNMODIFIED)
    |
DOM Shim (JS, ~2000 LOC)                 <-- used on browser Rendero + native
    |                                         real browser DOM remains the oracle path
Engine Bridge (engine.js / engine-native.js)
    |
Rendero Engine (Rust)
    |-- rendero-core: nodes, layout (Taffy), properties, provider traits
    |-- rendero-renderer: tiles, rasterization, text
    |-- rendero-css: CSS parsing (Lightning CSS) — trait: CssParser
    |-- rendero-text: text measurement (Parley) — trait: TextMeasurer
    |
Platform Backend (swappable)
    |-- Web: Canvas2D / WebGL2 / raster (WASM)
    |-- macOS/Win/Linux: softbuffer (CPU) / wgpu (GPU, future)
```

### Important files

- `crates/core/src/providers.rs` — all swappable traits: LayoutEngine, TextMeasurer, CssParser, FontResolver, GlyphRasterizer
- `crates/core/src/taffy_layout.rs` — Taffy flexbox layout, `node_to_taffy_style()` mapping
- `crates/core/src/properties.rs` — AutoLayout, LayoutJustify, LayoutWrap, LayoutMargin, LayoutPosition, LayoutSizeConstraints
- `crates/core/src/node.rs` — Node struct with margin, layout_position, size_constraints fields
- `crates/native-shell/src/engine_bridge.rs` — Rust engine dispatch, JS function registration, browser polyfills
- `crates/native-shell/src/main.rs` — winit window, event loop, headless mode, scroll handling
- `crates/css/src/lib.rs` — LightningCssParser, maps lightningcss Property → CssValue
- `crates/text/src/lib.rs` — ParleyTextMeasurer, FontiqueResolver
- `docs/demos/dom-shim/shims/style.js` — CSS property → engine command mapping
- `docs/demos/dom-shim/shims/engine-native.js` — native bridge (QuickJS → Rust dispatch)
- `docs/demos/dom-shim/shims/engine.js` — WASM bridge (browser → Rust dispatch)
- `docs/demos/dom-shim/shims/layout-style.js` — autoLayout builder from CSS values
- `docs/demos/dom-shim/shims/window.js` — window shim with scroll support
- `docs/demos/dom-shim/shims/css-values.js` — JS CSS parser (color, length, gradient, shorthand)
- `docs/demos/dom-shim/src/apple-react.jsx` — Apple website demo
- `crates/renderer/src/text.rs` — text rasterization with fontdb system font resolution + fontdue
- `scripts/capture-ground-truth.py` — extract browser layout as JSON (Playwright)
- `scripts/capture-engine-truth.py` — extract engine layout as JSON (Playwright)
- `scripts/capture-native-truth.py` — extract native headless layout/render state as JSON
- `scripts/compare-layout.py` — diff browser vs engine, report accuracy %
- `scripts/compare-runtime-triple.py` — compare browser, WASM, and native in one report
- `scripts/capture-layout-corpus.py` — run the synthetic layout corpus page against the Rendero WASM shim
- `scripts/run_accuracy_suite.sh` — rebuild bundles, refresh corpus dashboard, run the Apple oracle loop end to end
- `corpus/capture-site.py` — capture real website ground truth at multiple viewports
- `corpus/dashboard.py` — rebuild corpus dashboard from ground truth JSON
- `docs/demos/dom-shim/accuracy/` — browser-based accuracy test page + corpus

### Current accuracy

**44.86%** — 192/428 properties match (Apple demo page, 107 elements, browser oracle vs Rendero WASM path).

Native parity on the same Apple page: **7.94%** — 34/428 properties match.

WASM vs native parity on the same Apple page: **10.42%** — 45/432 properties match.

Synthetic layout corpus: **83.41%** — 377/452 properties.

Recent checkpoints: 1.6% → 12.4% (element alignment fix) → 12.9% (viewport-aware layout) → 19.2% (browser text measurement) → 49.3% (layered capture + translation fixes) → 57.01% (`margin:auto` + parent-relative `%` sizing) → 60.98% (surface-coordinate normalization, fixed-position translation, root/container height propagation fixes) → 44.86% / 7.94% / 10.42% (latest committed three-way runtime capture + text-layout fixes).

Run the full accuracy loop:
```sh
python3 scripts/capture-ground-truth.py http://localhost:5555/demos/dom-shim/ accuracy/apple-web.json   # once
python3 scripts/capture-engine-truth.py http://localhost:5555/demos/dom-shim/ accuracy/apple-engine.json
python3 scripts/compare-layout.py accuracy/apple-web.json accuracy/apple-engine.json accuracy/apple-comparison.json
scripts/run_accuracy_suite.sh
```

### Corpus

7 real sites with ground truth at 3 viewports each (desktop 1440, tablet 768, mobile 375):
- `corpus/sites/` — saved .mhtml pages
- `corpus/ground-truth/` — extracted element bounds + computed styles (JSON)
- `corpus/dashboard.json` — status dashboard
- `corpus/capture-site.py` — capture script

Sites: apple-macbook-pro, fin, github, gumroad, hacker-news, linear, tailadmin.

### The 20-item fix list

Every item is a DOM/CSS behavior the shim must handle correctly. Fix in priority order. Each fix must pass ALL test pages, not just the Apple demo. Run the accuracy comparison after every fix.

**Fix first (layout breaks without these):**

1. `margin: auto` — centering. Basic auto-margin support is now wired through the shared layout contract. Keep validating against more pages and edge cases.
2. `flex` shorthand — `flex: '1'` → `flex-grow: 1; flex-shrink: 1; flex-basis: 0%`. `flex: 'none'` → `0 0 auto`. `flex: '0 0 auto'` → three values. Getting this wrong breaks most layouts.
3. `padding`/`margin` shorthand — 1-value, 2-value, 3-value, 4-value expansion. Already partially handled in `css-values.js expandShorthand()` but verify all cases.
4. `position: fixed` — navbar. Taffy doesn't have fixed. Must attach to root Taffy node regardless of DOM position and be translated relative to the viewport, not the Rendero surface. Browser capture normalization for fixed nodes is now in place; remaining work is broader stacking/root-flow correctness.
5. Percentage `width`/`height` — basic parent-relative `%` sizing is now carried into the node model. Extend this to more cases (`height`, min/max, flex-basis, positioned elements) without collapsing back to viewport pixels.
6. `getComputedStyle()` / `offsetWidth` / `offsetHeight` — libraries read computed values. Must return values from Taffy's computed layout, not the raw set values. `width: '100%'` read back should return `'1280px'`.
7. `line-height` parsing — `lineHeight: '1.5'` (unitless multiplier) vs `lineHeight: '24px'` (pixels). Affects text spacing.
8. `overflow: hidden` — card clipping. Already wired but verify with border-radius (clip path must follow radius, not rectangular).

**Fix second (looks wrong without these):**

9. `linear-gradient()` parsing — card backgrounds. Parse angle, parse color stops with positions. Engine supports gradients, parsing is the gap.
10. `rgba()` / `hsla()` — semi-transparent backgrounds. Already partially handled.
11. `border` shorthand — `border: '1px solid #ddd'` → width + style + color. Each side can differ.
12. Named colors — 147 standard CSS colors. Lookup table in css-values.js, expand coverage.
13. `text-align: center` in flex — centered text within flex containers.
14. `z-index` — fixed navbar over scrolled content. Need to sort render items by z-index within stacking contexts.

**Fix third (edge cases):**

15. `min-width`/`max-width`/`min-height`/`max-height` — already wired via `size_constraints`, verify Taffy mapping.
16. `document.createDocumentFragment()` — React uses fragments. Support as lightweight containers that don't create engine nodes.
17. `font-weight` visual difference — different weights need different fonts or metrics. System font resolution is wired (fontdb) but CSS font inheritance is missing — most elements use default font.
18. Sub-pixel rounding — browsers anti-alias at fractional pixels. Round consistently (always floor, or always round) to avoid 1px gaps.
19. Group opacity compositing — `opacity: 0.5` on parent should composite children together, then apply opacity to the group. Not per-child.
20. `white-space: nowrap` — prevents text wrapping. Used in navbars, buttons, tags.

### Design principles

- **The browser is the oracle.** Never guess what CSS should do. Measure in the browser, compare with engine output. Every fix is validated against the ground truth.
- **No hardcoding for specific pages.** A fix that works for apple.com but breaks news.ycombinator.com is not a fix. Run accuracy checks against multiple pages.
- **Trait-based, swappable.** Every subsystem is behind a trait: LayoutEngine, TextMeasurer, CssParser, FontResolver, GlyphRasterizer. Swap implementations without touching callers.
- **The shim IS the browser.** It doesn't care if it's React, Vue, Svelte, Angular, or vanilla JS. They all call the same DOM APIs. Fix the DOM, all frameworks work.
- **Accuracy is a number.** Currently 44.86% on the Apple page, 7.94% native-vs-browser, 10.42% WASM-vs-native, and 83.41% on the synthetic corpus. Track it. Every commit either improves it or doesn't. No subjective "looks better."

### Key quirks to know

- **Inline layout doesn't exist.** Taffy does block/flex/grid, not inline text flow. `<p>Hello <strong>world</strong></p>` can't flow as one paragraph with mixed styles. Each element is a separate block. React Native has the same limitation. Acceptable for v1.
- **Text measurement has two high-accuracy paths plus a fallback.** On WASM/web: `BrowserTextMeasurer` uses the browser canvas as a deliberate temporary oracle during layout because browser sandboxes do not expose real font files to Fontique/Parley. On native: `ParleyTextMeasurer` is the intended path. `HeuristicTextMeasurer` still exists as a fallback/default core helper and for environments where the higher-fidelity measurers are unavailable. Browser Rendero also still carries a temporary JS-side eager text-size sync for some constrained text cases; that shortcut is tracked in `docs/project/parity-shortcuts.md`.
- **System font resolution is wired in.** The renderer resolves font families via fontdb (system fonts), falls back to embedded RobotoMono. But most elements don't have explicit `fontFamily` in inline style — CSS inheritance is not implemented in the shim, so fonts default to the constructor value "Inter" which falls back to RobotoMono.
- **`inf` from Taffy.** Unconstrained auto-sized containers return `f32::INFINITY`. If Taffy returns inf, the node size is left unchanged (not clamped to viewport). `compute_layout()` reads viewport from root node dimensions.
- **Frames without autoLayout default to `FlexDirection::Column`** in `node_to_taffy_style()`. This mimics CSS block flow (vertical stacking). Correct for 90% of cases. The 10% that break: inline elements, floats.

### Crate map

| Crate | Type | Purpose |
|-------|------|---------|
| `rendero-core` | lib | Scene graph, nodes, layout (Taffy), provider traits |
| `rendero-renderer` | lib | Tile rasterizer, text (fontdue + fontdb system fonts), SVG |
| `rendero-css` | lib | CSS parsing (Lightning CSS), CssParser trait impl |
| `rendero-text` | lib | Text measurement (Parley), TextMeasurer trait impl |
| `rendero-crdt` | lib | CRDT operations for collaboration |
| `rendero-wasm` | cdylib | WASM bindings for browser |
| `rendero-native-ffi` | cdylib+staticlib | C-ABI for Swift/C++ |
| `rendero-native-shell` | bin | Pure Rust native app (winit + QuickJS + softbuffer) |
| `rendero-fig-import` | lib | Figma .fig file parser |
