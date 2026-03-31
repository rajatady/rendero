# Current Status and Next Steps

Last updated: 2026-03-31

---

## Layout Accuracy

**44.86%** — 192/428 properties match on the Apple demo page (107 elements,
browser oracle vs Rendero WASM path).

Native parity on the same Apple page: **7.94%** — 34/428 properties.

WASM vs native parity on the same Apple page: **10.42%** — 45/432 properties.

Synthetic layout corpus: **83.41%** — 377/452 properties.

Progress on the Apple page so far:

- 1.6% → 12.4% — element alignment fixes
- 12.9% — viewport-aware layout
- 19.2% — browser text measurement
- 49.3% — layered capture pipeline made the real gaps explicit
- 57.01% — `margin:auto` + parent-relative `%` sizing through the shared layout contract
- 60.98% — surface-coordinate normalization, fixed-position translation, and root/container height propagation fixes
- 44.86% / 7.94% / 10.42% — latest committed three-way browser/WASM/native checkpoint after text-layout refactors and native capture were added

Current high-signal mismatches:

- browser vs WASM top-level wrapper is now too tall in the latest committed run:
  browser `2966.8` vs WASM `3477`
- native top-level wrappers are closer on height (`2909`) but native still diverges
  heavily elsewhere, with browser-vs-native accuracy at only `7.94%`
- native navbar width is still dramatically wrong in the latest headless capture:
  browser `1440` vs native `843`
- constrained text and block widths are still wrong enough to cascade into card
  and section layout differences

### Comparison Baseline

For parity work, native should be compared against the same desktop baseline as
the browser oracle: **1440×900 at DPR 1**. Until that baseline matches, avoid
mixing viewport-size mismatches into layout debugging.

Follow-up after the 1440 desktop baseline matches:

- `%` and other containing-block-relative sizes must stay parent-relative until
  layout time
- viewport units like `vw` / `vh` must remain viewport-relative and be tested
  separately from `%`
- media queries and browser-environment resolution (viewport size, DPR,
  orientation, pointer/hover, color scheme, reduced motion) belong in the
  translation/style-resolution layer, not in the engine

---

## What Renders Today

### Apple Website (React)

**Status: Full page renders on browser Rendero and native Rendero, but parity is
still poor in the latest committed checkpoint.**

What is working:

- navigation bar
- hero section with gradient image block
- 2×2 product grid
- feature section
- CTA section
- footer

What is still off:

- native browser-equivalent layout is still far off despite the same shared
  translation contracts being present
- feature-card and other constrained paragraphs still do not wrap with the same
  effective containing width as the browser oracle
- one browser/WASM top-level wrapper is now too tall rather than too short
- text metrics and line wrapping are still weaker than browser truth

### TestApp / Synthetic Corpus

**Status: Core layout behavior is stable enough to hit 83.41% across the
synthetic corpus.**

This is the main proof that Taffy itself is not the primary blocker anymore;
the remaining misses are in the translation and measurement pipeline.

---

## Corpus

7 real websites with ground truth captured at 3 viewports (desktop 1440,
tablet 768, mobile 375):

| Site | Elements (desktop) | Purpose |
|------|-------------------:|---------|
| apple-macbook-pro | ~200 | Marketing, flex-heavy |
| fin | ~150 | Dashboard, data-heavy |
| github | ~300 | Complex nested layouts |
| gumroad | ~180 | E-commerce, grid |
| hacker-news | ~100 | Simple block flow, tables |
| linear | ~250 | SaaS app, modern CSS |
| tailadmin | ~350 | Admin dashboard, Tailwind |

Ground truth lives in `corpus/ground-truth/`. The broader corpus currently
drives coverage/oracle reporting; the Apple demo remains the active end-to-end
comparison target.

---

## Most Important Recent Changes

### Shared layout / translation contract

- `margin:auto` now survives the shim → bridge → node model → Taffy path
- parent-relative `%` sizing is now represented explicitly in the node model
  instead of being collapsed to viewport pixels or generic fill semantics
- native and WASM now share those same layout inputs
- flex shorthand now follows the CSS spec: `flex: 1` maps to `grow=1 shrink=1 basis=0%`
- Taffy no longer reads `primary_sizing` / `counter_sizing` to size containers
  themselves; container size comes from node width/height, item sizing stays
  independent

Files:

- `crates/core/src/node.rs`
- `crates/core/src/properties.rs`
- `crates/core/src/taffy_layout.rs`
- `crates/wasm/src/lib.rs`
- `crates/native-shell/src/engine_bridge.rs`
- `docs/demos/dom-shim/shims/style.js`
- `docs/demos/dom-shim/shims/layout-style.js`
- `docs/demos/dom-shim/shims/css-values.js`
- `docs/demos/dom-shim/shims/engine.js`
- `docs/demos/dom-shim/shims/engine-native.js`

### Layered capture pipeline

The benchmark pipeline now captures and compares all three runtime surfaces:

- browser oracle
- browser Rendero / WASM
- native headless

And within those surfaces we inspect more than final bounds:

- shim normalization
- engine command stream
- engine model
- engine layout
- final surface

Files:

- `scripts/capture-ground-truth.py`
- `scripts/capture-engine-truth.py`
- `scripts/capture-native-truth.py`
- `scripts/compare-runtime-triple.py`
- `scripts/run_accuracy_suite.sh`
- `docs/demos/dom-shim/shims/rendero-api.js`
- `docs/demos/dom-shim/shims/engine-runtime.js`

### Recent text/layout parity fixes

- block text elements stay frame-backed instead of collapsing into a single
  engine text node
- child text nodes inherit parent text styling
- text-size locking distinguishes explicit authored text sizes from measured
  layout output
- `BrowserTextMeasurer` is now the explicit WASM text measurer because browser
  sandboxes do not expose font files to Fontique/Parley
- browser-engine capture remains normalized into the same surface coordinate
  space as the browser oracle

Files:

- `crates/text/src/lib.rs`
- `crates/wasm/src/browser_text.rs`
- `crates/wasm/src/lib.rs`
- `scripts/capture-engine-truth.py`
- `docs/demos/dom-shim/shims/element.js`
- `docs/demos/dom-shim/shims/style.js`
- `docs/demos/dom-shim/shims/text-node.js`
- `crates/core/src/taffy_layout.rs`
- `crates/renderer/src/text.rs`

### Native parity baseline

- native window defaults to the same desktop baseline used by the oracle
- native scroll now routes through the shared window/shim path rather than
  mutating camera state directly in the host

Files:

- `crates/native-shell/src/main.rs`
- `docs/demos/dom-shim/shims/window.js`
- `docs/demos/dom-shim/src/entry-macos.jsx`
- `docs/demos/dom-shim/src/entry-macos-vue.js`

---

## Main Remaining Gaps

1. **Constrained text measurement**
   Wrapped paragraphs and block text still lose the correct containing width
   before measurement in some paths. That is still the first wrong layer.

2. **Top-level wrapper height drift**
   The latest committed WASM capture overshoots one top-level wrapper badly:
   browser `2966.8` vs WASM `3477`.

3. **Native parity**
   Native headless captures are now part of the suite, but native is still far
   from browser/WASM parity.

---

## Infrastructure Available

| Tool | What it does |
|------|--------------|
| `scripts/run_accuracy_suite.sh` | Full oracle loop: build, serve, capture browser/WASM/native, compare, corpus refresh |
| `scripts/capture-ground-truth.py` | Extract browser layout as JSON via Playwright |
| `scripts/capture-engine-truth.py` | Extract Rendero engine state/layout as JSON via Playwright |
| `scripts/capture-native-truth.py` | Extract native headless state/layout as JSON |
| `scripts/compare-layout.py` | Diff browser vs engine, report accuracy % |
| `scripts/compare-runtime-triple.py` | Build one browser/WASM/native comparison report |
| `corpus/dashboard.py` | Rebuild corpus dashboard from captured ground truth |
| `scripts/verify_rendero_runtime.sh` | Browser/native parity verification loop |
| `RENDERO_HEADLESS_DUMP` | Native deterministic framebuffer dump |

---

## Notes

- The browser remains the oracle.
- Taffy is good enough for the simple/synthetic cases; the remaining misses are
  almost entirely in translation and measurement.
- Temporary shortcuts and known debt are tracked in
  `docs/project/parity-shortcuts.md`.
