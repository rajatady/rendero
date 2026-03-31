# Current Status and Next Steps

Last updated: 2026-03-31

---

## Layout Accuracy

**57.01%** — 244/428 properties match on the Apple demo page (107 elements,
browser oracle vs Rendero WASM path).

Synthetic layout corpus: **83.85%** — 379/452 properties.

Progress on the Apple page so far:

- 1.6% → 12.4% — element alignment fixes
- 12.9% — viewport-aware layout
- 19.2% — browser text measurement
- 49.3% — layered capture pipeline made the real gaps explicit
- 57.01% — `margin:auto` + parent-relative `%` sizing through the shared layout contract

Current high-signal mismatches:

- root/page height collapse: browser `2966.8` vs engine `1617`
- fixed navbar offset: browser `y=-44` vs engine `y=0`
- feature-card text still measured too wide/short in constrained layouts
- lower-section heights still drift from text/layout cascades

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

**Status: Full page renders on browser Rendero and native Rendero.**

What is working:

- navigation bar
- hero section with gradient image block
- 2×2 product grid
- feature section
- CTA section
- footer

What is still off:

- native live scroll/input parity is still being validated against the same
  shared contracts as WASM
- feature-card paragraphs do not yet wrap like the browser oracle, which
  inflates some card widths/heights
- root/document height is still too short
- text metrics and line wrapping are still weaker than browser truth

### TestApp / Synthetic Corpus

**Status: Core layout behavior is stable enough to hit 83.85% across the
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

Files:

- `crates/core/src/node.rs`
- `crates/core/src/properties.rs`
- `crates/core/src/taffy_layout.rs`
- `crates/wasm/src/lib.rs`
- `crates/native-shell/src/engine_bridge.rs`
- `docs/demos/dom-shim/shims/style.js`
- `docs/demos/dom-shim/shims/layout-style.js`
- `docs/demos/dom-shim/shims/engine.js`
- `docs/demos/dom-shim/shims/engine-native.js`

### Layered capture pipeline

The benchmark pipeline now captures more than final bounds. We now inspect:

- browser oracle
- shim normalization
- engine command stream
- engine model
- engine layout
- final surface

Files:

- `scripts/capture-ground-truth.py`
- `scripts/capture-engine-truth.py`
- `scripts/run_accuracy_suite.sh`
- `docs/demos/dom-shim/shims/rendero-api.js`
- `docs/demos/dom-shim/shims/engine-runtime.js`

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

1. **Root/document height propagation**
   Browser page height is still far larger than engine page height.

2. **Fixed/root offset behavior**
   The navbar is structurally correct, but the page/root offset is still off by
   `44px`.

3. **Constrained text measurement**
   Feature-card paragraphs are still measured as if they are unconstrained
   single-line text too early in the browser path. That is the next
   translation-layer target.

4. **Live native screenshot capture**
   Still blocked by Codex runtime macOS permissions, so true live-window
   screenshots remain separate from deterministic headless captures.

---

## Infrastructure Available

| Tool | What it does |
|------|--------------|
| `scripts/run_accuracy_suite.sh` | Full oracle loop: build, serve, capture, compare, corpus refresh |
| `scripts/capture-ground-truth.py` | Extract browser layout as JSON via Playwright |
| `scripts/capture-engine-truth.py` | Extract Rendero engine state/layout as JSON via Playwright |
| `scripts/compare-layout.py` | Diff browser vs engine, report accuracy % |
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
