# Rendero -- Vision and Architecture

A portable browser engine optimized for apps. Run React, Vue, or Svelte on
every platform with zero overhead on the web, a browser Rendero parity/debug
path via WASM, and a custom Rust rendering engine on native.

---

## Table of Contents

1. [Motivation](#motivation)
2. [The Insight](#the-insight)
3. [Competitive Landscape](#competitive-landscape)
4. [Architecture](#architecture)
5. [Provider Pattern](#provider-pattern)
6. [Platform Matrix](#platform-matrix)
7. [Performance Model](#performance-model)
8. [Escape Hatches](#escape-hatches)
9. [What Works Today](#what-works-today)
10. [Roadmap](#roadmap)
11. [Key Decisions Made](#key-decisions-made)

---

## 1. Motivation

Every cross-platform framework asks you to accept a trade-off:

- **Flutter** gives you a custom engine everywhere -- but on the web it ships
  a full rendering pipeline that competes with the browser. Your web build is
  slow, inaccessible, and SEO-hostile.
- **React Native** gives you native views on iOS/Android -- but on the web
  you use a completely different stack (react-dom or react-native-web), and the
  two stories share almost nothing.
- **Tauri** gives you web tech on desktop -- but it ships a WebView, not a
  native engine. You inherit every browser bug, you cannot control the GPU
  pipeline, and you are limited to whatever the OS WebView supports.
- **Electron** ships an entire Chromium. 200 MB minimum.
- **Capacitor / Cordova** wrap a WebView on mobile. Same limitations as Tauri,
  plus worse performance on low-end Android.

The root problem: **no framework gives you zero overhead on the web AND a
custom, controllable engine on native.** You always pay a tax on at least one
side.

Rendero exists to eliminate that trade-off entirely.

---

## 2. The Insight

The web already has a world-class app engine: the browser. Billions of dollars
of optimization, accessibility infrastructure, GPU compositing, text shaping,
and devtools. On the web, the right answer is to use ALL of it -- zero
abstraction tax.

On native, there is no browser. But the API contract that frameworks like
React and Vue target -- the DOM -- is a remarkably small surface. A `div` is a
rectangle with styles. `appendChild` inserts a child. `addEventListener`
registers a callback. That is the entire contract.

The insight:

```
On web production:  Framework --> real browser DOM --> browser engine --> pixels
On browser Rendero: Framework --> DOM SHIM --> WASM engine --> canvas pixels
On native:          Framework --> DOM SHIM --> Rust engine --> native pixels
```

The DOM shim is the main new thing. It is approximately 2,300 lines of
JavaScript. It implements `document.createElement`, `element.style`,
`appendChild`, `removeChild`, `addEventListener`, `dispatchEvent`, and the
other DOM APIs that React and Vue actually call. Under the hood, each DOM
mutation becomes a call into the Rendero pipeline: normalize styles, emit
engine commands, update the engine model, recompute layout, and re-render.

React, Vue, and Svelte run **completely unmodified**. They import `react-dom`
and call `createRoot`. They never know they are not in a browser.

---

## 3. Competitive Landscape

| Framework       | Web Story                       | Native Story                    | Language          | Bundle Size    | Ecosystem Compatibility               |
|-----------------|---------------------------------|---------------------------------|-------------------|----------------|---------------------------------------|
| **Rendero**     | Real browser, zero overhead     | Rust engine via DOM shim        | JS/TS + Rust      | ~600 KB (WASM) | Full npm / react-dom / Vue            |
| React Native    | Separate (react-native-web)     | Native views (iOS/Android)      | JS/TS             | ~2 MB          | RN-specific packages only             |
| Flutter         | Custom engine (CanvasKit/WASM)  | Custom engine (Skia/Impeller)   | Dart              | ~2 MB (web)    | Dart-only, no npm                     |
| Tauri           | WebView (system)                | WebView (system)                | JS/TS + Rust      | ~3 MB          | Full npm, limited by WebView          |
| Capacitor       | WebView (system)                | WebView (system)                | JS/TS             | ~5 MB          | Full npm, limited by WebView          |
| KMP / Compose   | Compose for Web (experimental)  | Compose Multiplatform           | Kotlin            | ~5 MB          | Kotlin-only, no npm                   |
| Electron        | Full Chromium                   | Full Chromium                   | JS/TS             | ~200 MB        | Full npm, full browser API            |
| Dioxus          | WASM or WebView                 | Custom renderer (Taffy)         | Rust               | ~1 MB          | Rust-only, no npm                     |

Key differentiators for Rendero:

- **Web**: Zero tax. No WASM download, no custom renderer, no accessibility
  shim. The browser does what it already does.
- **Native**: Full npm ecosystem. Use any React component library. The shim
  handles the translation.
- **Language**: App code is JS/TS. Engine code is Rust. Each language is used
  where it is strongest.

---

## 4. Architecture

Rendero has a four-layer architecture. Each layer is independently replaceable.

```
+------------------------------------------------------------------+
|  Layer 1: Application + Framework                                |
|  React / Vue / Svelte / Solid / vanilla JS                       |
|  npm packages, component libraries, state management             |
|  COMPLETELY UNMODIFIED                                           |
+------------------------------------------------------------------+
        |                                    |
        | (on web: real DOM)                 | (on browser Rendero + native: DOM shim)
        v                                    v
+-------------------------+    +-------------------------------+
|  Browser Engine         |    |  Layer 2: DOM Shim            |
|  (Chrome, Safari, etc.) |    |  ~2,300 LOC JavaScript        |
|  Real DOM oracle path   |    |  document, element, style,    |
|  for production + tests |    |  events, CSS value parsing    |
+-------------------------+    +-------------------------------+
                                         |
                                         | engine calls
                                         v
                               +-------------------------------+
                               |  Layer 3: Rendero Engine      |
                               |  ~17,700 LOC Rust             |
                               |  Document tree, layout (Taffy)|
                               |  text measurement, hit testing|
                               |  property system, CRDT sync   |
                               +-------------------------------+
                                         |
                                         | render commands
                                         v
                               +-------------------------------+
                               |  Layer 4: Platform Backend    |
                               |  Swappable per target:        |
                               |  - Canvas2D (web demos)       |
                               |  - softbuffer (native today)  |
                               |  - wgpu (native future)       |
                               |  - Metal / Vulkan / DX12      |
                               |  - CPU tile rasterizer        |
                               |  - PDF / SVG export           |
                               |  - Terminal (TUI)             |
                               +-------------------------------+
```

### Layer 1: Application + Framework

Your app. Written in React, Vue, Svelte, or plain JS. Uses npm packages
normally. Imports `react-dom` or `vue` and calls the standard mounting APIs.
No special imports, no framework forks, no compatibility layers.

### Layer 2: DOM Shim

A lightweight JavaScript module (~2,300 LOC across 11 files) that implements
the subset of the DOM API that frameworks actually use:

| File              | Purpose                                              |
|-------------------|------------------------------------------------------|
| `element.js`      | `ShimElement` -- tag name, attributes, children, style |
| `document.js`     | `ShimDocument` -- createElement, getElementById, body  |
| `node.js`         | Base node class -- parentNode, childNodes, tree ops    |
| `text-node.js`    | `ShimTextNode` -- text content nodes                   |
| `style.js`        | CSSStyleDeclaration proxy -- maps CSS to engine props  |
| `css-values.js`   | CSS value parsing (colors, lengths, units)             |
| `events.js`       | ShimEvent, ShimMouseEvent, ShimKeyboardEvent           |
| `window.js`       | Window shim for non-browser environments               |
| `engine.js`       | Bridge to WASM engine (web mode)                       |
| `engine-native.js`| Bridge to native engine (QuickJS mode)                 |
| `index.js`        | Barrel export, `installShim()`, event wiring           |

The shim has two Rendero modes:

- **Web mode** (`installShim(canvas)`): Intercepts DOM calls, routes to
  the WASM engine, renders to a canvas. Used for parity work, browser
  benchmarking, and deterministic layer capture.
- **Native mode** (`installShimNative()`): Same DOM API, but routes to
  pre-registered native functions (`__rendero_*`) exposed by the Rust shell
  via QuickJS. No WASM, no canvas, no browser.

### Layer 3: Rendero Engine

A Rust engine organized into crates:

| Crate              | Purpose                                            |
|---------------------|----------------------------------------------------|
| `rendero-core`      | Document tree, node model, properties, hit testing, layout traits, Taffy layout |
| `rendero-renderer`  | CPU rasterizer, font loading (fontdb), glyph rendering (fontdue) |
| `rendero-crdt`      | Conflict-free replicated data types for collaborative editing |
| `rendero-wasm`      | WASM bindings (`wasm-bindgen`), `CanvasEngine` API |
| `rendero-native-ffi`| FFI bindings for native (C ABI)                    |
| `rendero-native-shell` | Full native app: winit window + QuickJS + render loop |
| `rendero-fig-import`| Figma file import                                  |

### Layer 4: Platform Backend

The rendering surface is swappable. Today:

- **Web**: The WASM engine produces an RGBA pixel buffer. JavaScript blits it
  to a `<canvas>` via `putImageData`.
- **Native**: The Rust shell produces an RGBA pixel buffer. `softbuffer` blits
  it to the OS window surface.

Future backends include wgpu (GPU-accelerated), Metal, Vulkan, PDF export,
SVG export, and terminal rendering.

---

## 5. Provider Pattern

Every subsystem in the engine is behind a trait. Nothing depends on a concrete
implementation. This allows swapping, fallback, and testing.

```
trait LayoutEngine {
    fn compute(&mut self, tree: &mut DocumentTree, root: &NodeId, viewport: (f32, f32));
}

trait TextMeasurer {
    fn measure(&self, runs: &[TextRun], max_width: f32) -> (f32, f32);
}

trait JSRuntime {
    fn evaluate(&mut self, code: &str) -> Result<(), String>;
    fn drain_pending_jobs(&mut self);
    fn has_global(&mut self, name: &str) -> bool;
    fn call_global(&mut self, name: &str) -> Result<(), String>;
}
```

Current implementations:

| Trait            | Implementation            | Notes                                |
|------------------|---------------------------|--------------------------------------|
| `LayoutEngine`   | `TaffyLayout`             | Full CSS flexbox via Taffy           |
| `LayoutEngine`   | `LegacyLayout`            | Original basic auto-layout           |
| `TextMeasurer`   | `HeuristicTextMeasurer`   | Core fallback/default helper for environments without a higher-fidelity measurer |
| `TextMeasurer`   | `ParleyTextMeasurer`      | Native high-fidelity path via `rendero-text` |
| `TextMeasurer`   | `BrowserTextMeasurer`     | WASM-only deliberate oracle path using browser canvas until shared font loading lands |
| `JSRuntime`      | `QuickJSRuntime`          | Via `rquickjs` crate                 |
| `JSRuntime`      | (planned) `HermesRuntime` | Meta's Hermes engine                 |

Planned additional traits:

| Trait              | Purpose                                          |
|--------------------|--------------------------------------------------|
| `FontResolver`     | Find font files by family/weight/style           |
| `GlyphRasterizer`  | Rasterize shaped glyphs to bitmaps               |
| `GPUBackend`       | Abstract over wgpu / Metal / Vulkan              |

---

## 6. Platform Matrix

| Platform      | Window System | GPU / Render      | JS Engine    | Status        |
|---------------|---------------|-------------------|--------------|---------------|
| **Web**       | Browser       | Browser (Canvas2D)| Browser V8/JSC | Working     |
| **macOS**     | winit         | softbuffer (CPU)  | QuickJS      | Working       |
| **Windows**   | winit         | softbuffer (CPU)  | QuickJS      | Builds, untested |
| **Linux**     | winit         | softbuffer (CPU)  | QuickJS      | Builds, untested |
| **iOS**       | winit         | wgpu (Metal)      | QuickJS      | Planned       |
| **Android**   | winit         | wgpu (Vulkan)     | QuickJS      | Planned       |
| **Vision Pro**| winit (?)     | wgpu (Metal)      | QuickJS      | Planned       |
| **PDF**       | N/A           | CPU rasterizer    | N/A          | Planned       |
| **SVG**       | N/A           | Vector export     | N/A          | Planned       |
| **Terminal**   | crossterm     | CPU tiles         | N/A          | Experimental  |

The pure Rust stack (winit + softbuffer/wgpu + QuickJS) compiles on every
platform with `cargo build`. The legacy Swift shell remains as a compatibility
path, but parity work now targets the pure Rust shell plus the browser Rendero
WASM path.

---

## 7. Performance Model

Where does time go when rendering a frame? This table breaks down each stage,
what Rendero does, and how it compares to alternatives.

| Stage              | Rendero (native)                  | Flutter              | React Native          | Browser               |
|--------------------|-----------------------------------|----------------------|-----------------------|-----------------------|
| **JS Execution**   | QuickJS (~5x slower than V8)      | N/A (Dart VM)        | Hermes/JSC            | V8/JSC (fastest)      |
| **DOM Shim**       | ~0.1 ms per frame (tree diffing)  | N/A                  | N/A (native bridge)   | N/A (native DOM)      |
| **CSS Parsing**    | Subset parser in style.js         | N/A (Dart widgets)   | N/A (StyleSheet)      | Full CSS engine       |
| **Layout**         | Taffy (Rust, fast flexbox)        | Dart layout          | Yoga (C++)            | Blink layout          |
| **Text Shaping**   | Parley on native, browser-canvas oracle on WASM, heuristic fallback | libTxt/Skia | Platform text | HarfBuzz (accurate) |
| **Rasterization**  | CPU tiles (fontdue)               | Skia/Impeller (GPU)  | Platform views        | GPU compositing       |
| **Compositing**    | Single buffer blit                | GPU compositor       | Platform compositor   | GPU compositor        |

**Where the overhead is today:**

1. **QuickJS** is the largest bottleneck. It is ~5x slower than V8 for CPU-bound
   JS. For typical UI code (event handlers, state updates), this is not
   noticeable. For heavy computation, use Rust plugins (escape hatch Level 1).

2. **CPU rasterization** via softbuffer is fine for simple UIs but will not
   scale to complex scenes with many elements, gradients, or blur effects.
   The path to fixing this is wgpu (GPU-accelerated rendering).

3. **Text measurement still has one deliberate platform split.** Native uses
   Parley for real shaping. WASM temporarily uses the browser canvas as the
   layout oracle because browser sandboxes do not expose font bytes to
   Fontique/Parley. This is the only intentional measurement divergence in the
   current architecture and should disappear once browser font loading is wired
   into the shared Rust text path.

**Where there is zero overhead:**

- On web, Rendero adds nothing. The browser does everything.
- Layout via Taffy is competitive with Yoga (same algorithm, similar perf).
- The DOM shim tree operations are O(1) amortized.

---

## 8. Escape Hatches

Rendero provides three levels of escape hatch, from comfortable to maximum
performance.

### Level 3: DOM Shim (Comfortable)

Write standard React/Vue code. Use `div`, `span`, `onClick`, `style`. The DOM
shim translates everything. This is the default and covers 90% of use cases.

```jsx
function Card({ title }) {
    return (
        <div style={{ padding: 16, backgroundColor: '#fff', borderRadius: 8 }}>
            <span style={{ fontSize: 18, fontWeight: 'bold' }}>{title}</span>
        </div>
    );
}
```

### Level 2: Direct Engine API (Fast)

Bypass the DOM shim and call the Rendero engine directly. Useful for custom
drawing, particle systems, or performance-critical code. Available from JS
via the WASM or native FFI bindings.

```js
import { engine } from 'rendero';

const nodeId = engine.create_rectangle('myRect');
engine.set_x(nodeId, 100);
engine.set_y(nodeId, 200);
engine.set_width(nodeId, 300);
engine.set_fill(nodeId, 0xFF0000FF);
```

### Level 1: Rust Plugin (Maximum Performance)

Write a Rust crate that links directly into the engine. Zero FFI overhead,
zero JS overhead. Use this for computationally intensive features like
physics, pathfinding, image processing, or custom renderers.

```rust
use rendero_core::tree::DocumentTree;

pub fn apply_physics(tree: &mut DocumentTree, dt: f32) {
    // Direct access to the document tree, no serialization,
    // no JS boundary crossing.
}
```

---

## 9. What Works Today

- [x] DOM shim: createElement, appendChild, removeChild, insertBefore, style
- [x] CSS subset: width, height, backgroundColor, color, fontSize, fontWeight,
      borderRadius, display (flex), flexDirection, gap, alignItems,
      justifyContent, padding (partial), margin (partial), position, overflow
- [x] Event system: click, mouseenter, mouseleave, mouseover, mouseout, touch
- [x] React 18 with react-dom: createRoot, useState, useEffect, useRef, lists, keys
- [x] Taffy flexbox layout (row, column, gap, align, justify, wrap)
- [x] CPU rasterization: rectangles, rounded corners, text, colors
- [x] Font loading via fontdb + glyph rasterization via fontdue
- [x] Hit testing (click on rendered elements, get shim element back)
- [x] Native macOS shell: winit window + QuickJS + softbuffer
- [x] WASM build: runs in browser, renders to canvas
- [x] esbuild bundling: JSX, React, DOM shim into single file
- [x] Multiple demos: design tool, genome browser, earthquake explorer,
      neural net visualizer, 3D splat viewer, React-on-Rendero
- [ ] CSS: padding, margin (all sides), border, overflow clipping
- [ ] Text: wrapping, multi-line, real shaping
- [ ] Scroll containers
- [ ] Input elements (TextInput, select, checkbox)
- [ ] GPU rendering (wgpu)
- [ ] Images / media
- [ ] Accessibility tree
- [ ] Animation (requestAnimationFrame bridge)

---

## 10. Roadmap

### Phase 1: CSS Fidelity (Current)

Expand the CSS property coverage to handle real-world layouts:

- Padding and margin on all four sides
- Border (width, color, style)
- Overflow: hidden (clip children)
- Min/max width/height
- Percentage-based sizing
- z-index and stacking contexts

### Phase 2: Text Shaping

Replace the heuristic text measurer with real text shaping:

- Integrate fontdb for font resolution (already in renderer)
- Integrate Parley (or rustybuzz directly) for text shaping
- Support multi-line text, word wrapping, line height
- Bidirectional text (LTR/RTL)

### Phase 3: GPU Rendering

Move from CPU rasterization (softbuffer) to GPU rendering:

- wgpu as the primary GPU backend
- Instanced quad rendering for rectangles
- Signed distance field (SDF) text rendering
- GPU-accelerated blur, shadows, gradients
- Metal on macOS/iOS, Vulkan on Android/Linux, DX12 on Windows

### Phase 4: More Platforms

- iOS: winit + wgpu (Metal) + QuickJS
- Android: winit + wgpu (Vulkan) + QuickJS
- Windows / Linux: testing and polish
- PDF / SVG export backends

### Phase 5: Production Readiness

- Accessibility tree generation from DOM shim
- Input elements (TextInput, forms)
- Scroll containers with inertia
- Animation bridge (requestAnimationFrame, CSS transitions)
- DevTools protocol support
- Hot reload

---

## 11. Key Decisions Made

### Pure Rust (not Swift)

The initial native shell was a Swift + AppKit application using
JavaScriptCore. This was abandoned because:

- Swift's AppKit run loop fights with the JS event loop (Timer vs
  DispatchQueue). Getting React's scheduler to commit synchronously required
  fighting the platform.
- Two languages means two build systems (Xcode + Cargo).
- Swift is macOS-only. The whole point is cross-platform.

The replacement is a pure Rust binary: winit for windowing, softbuffer for
pixel blitting, QuickJS for JavaScript. `cargo run` works on macOS, Windows,
and Linux from the same codebase.

### QuickJS (not JavaScriptCore or V8)

QuickJS was chosen because:

- Pure C, compiles everywhere (including WASM).
- Small binary size (~500 KB).
- Deterministic execution -- no JIT, no GC pauses.
- Excellent Rust bindings via `rquickjs`.
- Good enough for UI code (event handlers, state updates).
- The bottleneck is rasterization, not JS execution.

The trade-off is ~5x slower raw JS execution compared to V8. This is
acceptable because UI code is not CPU-bound, and heavy computation should
use Rust plugins (escape hatch Level 1).

### Provider Pattern (traits everywhere)

Every subsystem is behind a trait:

- `LayoutEngine` -- swap Taffy for a future incremental layout engine.
- `TextMeasurer` -- swap the heuristic for real shaping.
- `JSRuntime` -- swap QuickJS for Hermes.
- Future: `FontResolver`, `GlyphRasterizer`, `GPUBackend`.

This adds minimal complexity but provides maximum flexibility. It also
enables testing (mock implementations) and graceful fallback (if Parley
fails to shape a string, fall back to the heuristic measurer).

### Taffy for Layout

Taffy is a pure Rust implementation of CSS flexbox (and grid). It was chosen
over Yoga because:

- Pure Rust, no C++ FFI.
- Actively maintained.
- Supports flexbox and CSS grid.
- Same algorithm and spec compliance as Yoga.

### softbuffer for Now, wgpu Later

The current renderer produces an RGBA pixel buffer on the CPU and blits it
to the window via softbuffer. This is intentionally simple:

- No GPU driver dependencies.
- Works everywhere, including SSH and VMs.
- Easy to debug (dump the pixel buffer to a PNG).
- Fast enough for simple UIs (~60 FPS for typical app layouts).

wgpu is the planned upgrade path for GPU-accelerated rendering, but CPU
rasterization will remain available as a fallback for headless, testing,
and server-side rendering scenarios.

---

*Last updated: 2026-03-30*
