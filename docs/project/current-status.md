# Current Status and Next Steps

Last updated: 2026-03-31

---

## Layout Accuracy

**19.2%** — 82/428 properties match (Apple demo page, 107 elements, WASM web path).

Progress: 1.6% → 12.4% (element alignment) → 12.9% (viewport-aware layout) → 19.2% (browser text measurement).

Remaining mismatches by property:
- y position: 100 mismatches, avg 74px off
- x position: 91 mismatches, avg 136px off
- width: 84 mismatches, avg 75px off
- height: 71 mismatches, avg 49px off

Root causes: text width differences (canvas.measureText font string), container height cascading from text size errors, CSS properties not yet mapped.

---

## What Renders Today

### Apple Website (React)
**Status: Full page renders on both web (WASM canvas) and native (Rust softbuffer).**

What shows:
- Navigation bar with all links
- Hero section: "iPhone 16 Pro", subtitle, links, gradient image
- Product grid: MacBook Pro, MacBook Air, iPad Pro, iPad Air in 2×2 wrap layout
- Feature section: "Why Apple." with Apple Intelligence, Privacy cards
- CTA section: "Get the new iPhone." with pricing and blue Buy button
- Footer: 4-column layout with links, copyright

What's off:
- Text widths differ from browser (monospace heuristic on native, canvas.measureText on WASM)
- Heights cascade errors from text size differences
- No CSS font inheritance — most elements use default font
- Scroll partially working (camera movement wired, content height issue)

### TestApp (colored rectangles)
**Status: Pixel-perfect on both web and native.**

---

## Corpus

7 real websites with ground truth captured at 3 viewports (desktop 1440, tablet 768, mobile 375):

| Site | Elements (desktop) | Purpose |
|------|-------------------|---------|
| apple-macbook-pro | ~200 | Marketing, flex-heavy |
| fin | ~150 | Dashboard, data-heavy |
| github | ~300 | Complex nested layouts |
| gumroad | ~180 | E-commerce, grid |
| hacker-news | ~100 | Simple block flow, tables |
| linear | ~250 | SaaS app, modern CSS |
| tailadmin | ~350 | Admin dashboard, Tailwind |

Ground truth stored in `corpus/ground-truth/`. Not yet plugged into automated comparison — Apple demo is the active test target.

---

## The 20-Item Fix List

See CLAUDE.md (or AGENTS.md) for the full prioritized list. Summary:

**Fix first (layout breaks):** margin:auto, flex shorthand, padding/margin shorthand, position:fixed, percentage width/height, getComputedStyle, line-height, overflow:hidden.

**Fix second (looks wrong):** linear-gradient, rgba/hsla, border shorthand, named colors, text-align:center, z-index.

**Fix third (edge cases):** min/max constraints, createDocumentFragment, font-weight visual, sub-pixel rounding, group opacity, white-space:nowrap.

---

## Crates Built This Session

| Crate | Tests | Status |
|-------|-------|--------|
| rendero-css (Lightning CSS) | 25 pass | Built, trait-based, not used for visual output yet |
| rendero-text (Parley) | 7 pass | Built, trait-based, not wired into renderer yet |

Provider traits added to `crates/core/src/providers.rs`: CssParser, FontResolver, GlyphRasterizer (joining existing LayoutEngine, TextMeasurer).

---

## Infrastructure Built This Session

| Tool | What it does |
|------|-------------|
| `scripts/capture-ground-truth.py` | Extract browser layout as JSON via Playwright |
| `scripts/capture-engine-truth.py` | Extract engine layout as JSON via Playwright |
| `scripts/compare-layout.py` | Diff browser vs engine, report accuracy % |
| `corpus/capture-site.py` | Capture real website at multiple viewports |
| Headless rendering | `RENDERO_HEADLESS_DUMP` env var, PPM output |
| System font resolution | fontdb in renderer, per-run font lookup |
| Browser text measurement | canvas.measureText() on WASM path |

---

## Key Changes Since Last Session

### Rust

- `crates/core/src/properties.rs` — Added LayoutJustify, LayoutWrap, LayoutMargin, LayoutPosition, LayoutSizeConstraints
- `crates/core/src/node.rs` — Added margin, layout_position, size_constraints fields to Node
- `crates/core/src/taffy_layout.rs` — justify-content, flex-wrap, margin, absolute position, min/max mapping. Column default for frames. inf handling.
- `crates/core/src/providers.rs` — CssParser trait, FontResolver trait, GlyphRasterizer trait, CssValue enum
- `crates/core/src/layout.rs` — compute_layout reads viewport from root node
- `crates/renderer/src/text.rs` — System font resolution via fontdb, per-run font lookup
- `crates/css/` — NEW: rendero-css crate with LightningCssParser
- `crates/text/` — NEW: rendero-text crate with ParleyTextMeasurer, FontiqueResolver
- `crates/native-shell/src/engine_bridge.rs` — Many new dispatch commands (margin, position, size_constraints, gradients, stroke, shadow)
- `crates/native-shell/src/main.rs` — Headless mode, scroll handling, RENDERO_DEMO env var

### JS Shims

- `shims/style.js` — Browser text measurement, size constraints, margin, autoLayout with align/justify/wrap
- `shims/engine-native.js` — Full dispatch bridge with gradient, stroke, shadow, font family, text align
- `shims/engine.js` — Renderer switching (canvas2d/raster), Rendero namespace
- `shims/window.js` — Scroll shim with camera sync
- `shims/layout-style.js` — NEW: autoLayout builder
- `shims/engine-runtime.js` — NEW: bridge abstraction + measureTextBrowser
- `shims/rendero-api.js` — NEW: Rendero namespace with engine/native APIs

### WASM

- Updated set_auto_layout with align/justify/wrap params
- 6 new API methods: set_node_layout_position, set_node_size_constraints, set_node_sizing, set_node_clip_content, set_node_margin
- Pre-built WASM binary in docs/pkg/

---

## Key Bugs Found and Fixed This Session

| Bug | Root Cause | Fix |
|-----|-----------|-----|
| Sections overlap at (0,0) | Frames without autoLayout had no flex direction | Default to FlexDirection::Column (CSS block flow) |
| Root height = infinity | Taffy returns inf for unconstrained nodes | Leave node size unchanged on inf (don't clamp to viewport) |
| Width = 8401 on root div | compute_layout used (0,0) viewport | Read viewport from root node dimensions |
| Text too wide on WASM | Rust heuristic (len*0.65*fontSize) | Browser canvas.measureText() on WASM path |
| Text nodes size=0 after constructor change | Node::text() set 0x0, heuristic removed | Taffy measurer handles 0x0 text nodes via else branch |
| WASM build fails with getrandom | rendero-css pulled in rayon→getrandom | Removed rendero-css dep from rendero-wasm |
| Scroll not working on native | installWindowScrollShim not called in native init | Added to installShimNative() |
