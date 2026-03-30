# Rendero DOM Shim

## 1. Overview

The Rendero DOM Shim is a virtual DOM implementation that intercepts standard browser DOM API calls made by frameworks like React (via `react-dom`) and Vue and routes them to the Rendero rendering engine instead of the browser's native DOM. The shim implements enough of the DOM API surface -- `document.createElement`, `appendChild`, `style.*`, `addEventListener`, `getBoundingClientRect`, etc. -- that these frameworks operate without modification. They believe they are running in a browser; in reality, every node, style change, and event is handled by Rendero's 2D scene graph.

**Key properties:**

- **Framework-agnostic** -- works with React (react-dom) and Vue out of the box. Any framework that targets the standard DOM API can use it.
- **Platform-agnostic** -- runs in three modes: real browser DOM, browser canvas via WASM, and native macOS via JavaScriptCore + Rust FFI.
- **Zero framework patches** -- no forked react-dom, no custom reconciler. Stock `react-dom/client` and `vue` packages.

---

## 2. Architecture

The shim supports three rendering modes, all sharing the same application code (e.g., the same React component tree):

```
+---------------------+      +---------------------+      +---------------------+
|    Mode 1: Web      |      | Mode 2: Web Canvas  |      | Mode 3: Native      |
|  (Real Browser DOM) |      | (DOM Shim + WASM)   |      | (DOM Shim + Rust)   |
+---------------------+      +---------------------+      +---------------------+
|                     |      |                     |      |                     |
| react-dom/vue       |      | react-dom/vue       |      | react-dom/vue       |
|   |                 |      |   |                 |      |   |                 |
|   v                 |      |   v                 |      |   v                 |
| Browser DOM         |      | ShimDocument        |      | ShimDocument        |
| (real elements)     |      | ShimElement          |      | ShimElement          |
|                     |      | ShimTextNode        |      | ShimTextNode        |
|                     |      |   |                 |      |   |                 |
|                     |      |   v                 |      |   v                 |
|                     |      | engine.js           |      | engine-native.js    |
|                     |      | (ops queue + WASM)  |      | (ops queue + FFI)   |
|                     |      |   |                 |      |   |                 |
|                     |      |   v                 |      |   v                 |
|                     |      | CanvasEngine (WASM) |      | Rust engine (C FFI) |
|                     |      | renders to <canvas> |      | renders to MTKView  |
+---------------------+      +---------------------+      +---------------------+
```

| Mode | Entry Point | Engine Bridge | Runtime | Rendering Target |
|------|-------------|---------------|---------|------------------|
| Web | `entry-react-web.jsx` / `entry-vue-web.js` | None (real DOM) | Browser | Browser compositor |
| Web Canvas | `entry-react-native.jsx` / `entry-vue-native.js` | `engine.js` (WASM) | Browser | `<canvas>` element |
| Native macOS | `entry-macos.jsx` | `engine-native.js` (FFI) | JavaScriptCore | `MTKView` (Metal) |

---

## 3. File Inventory

### `shims/` Directory

| File | Size | Purpose | Key Exports |
|------|------|---------|-------------|
| `index.js` | 6.9 KB | Barrel export and initialization orchestrator. Provides `installShim()` for WASM mode and `installShimNative()` for native mode. Wires canvas events to shim event dispatch. | `installShim(canvas)`, `installShimNative()`, re-exports of `ShimDocument`, `ShimElement`, `ShimTextNode`, `ShimEvent`, `ShimMouseEvent` |
| `node.js` | 5.3 KB | Base class for all shim nodes. Implements the DOM `Node` interface: parent/child relationships, tree mutation methods, sibling traversal. | `ShimNode`, `ELEMENT_NODE`, `TEXT_NODE`, `COMMENT_NODE`, `DOCUMENT_NODE`, `DOCUMENT_FRAGMENT_NODE` |
| `element.js` | 12.8 KB | Core DOM `Element` shim. Maps HTML tag names to engine node types (Frame or Text). Handles attributes, classList, query selectors, geometry, engine lifecycle. | `ShimElement` |
| `text-node.js` | 2.0 KB | DOM `Text` node. Updates engine text content when `textContent`/`nodeValue`/`data` are set. | `ShimTextNode` |
| `document.js` | 5.4 KB | DOM `Document` shim. Provides `createElement`, `createTextNode`, `createDocumentFragment`, `createTreeWalker`, query methods. Maintains `html > body` structure. | `ShimDocument` |
| `window.js` | 3.8 KB | Window-level API shim. Provides `getComputedStyle`, `matchMedia`, `scrollTo`, `getSelection`, viewport dimensions, `navigator`, `location`, `history`. | `createWindowShim(shimDocument)` |
| `style.js` | 8.5 KB | `CSSStyleDeclaration` Proxy. Intercepts every `element.style.*` write and translates it to engine property calls. Handles shorthand expansion and CSS text parsing. | `createStyleProxy(element)` |
| `css-values.js` | 12.3 KB | CSS value parsers. Converts CSS color strings, length units, font weights, box shadows, and linear gradients into engine-compatible numeric values. | `parseColor`, `parseLength`, `parseFontWeight`, `parseBoxShadow`, `parseLinearGradient`, `expandShorthand`, `setViewport` |
| `engine.js` | 9.2 KB | WASM engine bridge. Manages the ops queue, node registry (shimId -> counter/clientId), render loop via `requestAnimationFrame`. All engine mutations are serialized through the queue. | `initEngine`, `allocId`, `markDirty`, `engineCreateFrame`, `engineCreateText`, `engineDeleteNode`, `engineSetProp`, `engineGetBounds`, `hitTest` |
| `engine-native.js` | 8.4 KB | Native engine bridge (drop-in replacement for `engine.js`). Same API, but calls `__rendero_*` global functions registered by Swift instead of WASM methods. Tree ops (create/insert) are synchronous; property changes are queued. | `initEngine`, `allocId`, `markDirty`, `engineCreateFrame`, `engineCreateText`, `engineDeleteNode`, `engineSetProp`, `engineGetBounds`, `hitTest`, `flushAndRender` |
| `events.js` | 5.9 KB | DOM Event system. Implements `Event`, `MouseEvent`, `KeyboardEvent`, `FocusEvent`, `InputEvent` classes and the `EventTarget` mixin with full capture/target/bubble phase dispatching. | `ShimEvent`, `ShimMouseEvent`, `ShimKeyboardEvent`, `ShimFocusEvent`, `ShimInputEvent`, `EventTargetMixin` |

### `src/` Directory

| File | Size | Purpose |
|------|------|---------|
| `entry-react-web.jsx` | 0.7 KB | React + real browser DOM entry (Mode 1) |
| `entry-react-native.jsx` | 1.4 KB | React + DOM shim + WASM canvas entry (Mode 2) |
| `entry-vue-web.js` | 0.6 KB | Vue + real browser DOM entry (Mode 1) |
| `entry-vue-native.js` | 1.0 KB | Vue + DOM shim + WASM canvas entry (Mode 2) |
| `entry-macos.jsx` | 3.0 KB | React + DOM shim + native Rust engine entry (Mode 3) |
| `apple-react.jsx` | 15.2 KB | Demo application (React version) |
| `apple-vue.js` | 12.3 KB | Demo application (Vue version) |

---

## 4. Node System

### Class Hierarchy

```
ShimNode (node.js)
  |-- ShimElement (element.js)        nodeType = 1 (ELEMENT_NODE)
  |-- ShimTextNode (text-node.js)     nodeType = 3 (TEXT_NODE)
  |-- ShimDocumentFragment            nodeType = 11 (DOCUMENT_FRAGMENT_NODE)
  |-- ShimDocument (document.js)      nodeType = 9 (DOCUMENT_NODE)
```

### ShimNode (Base Class)

Every node gets a unique `_engineId` via `allocId()` at construction time. The `EventTargetMixin` is mixed into `ShimNode.prototype`, giving all nodes `addEventListener`, `removeEventListener`, and `dispatchEvent`.

**Tree mutation methods:**

| Method | Behavior |
|--------|----------|
| `appendChild(child)` | Removes child from current parent (if any), appends to `childNodes`, calls `_onChildInserted(child)` |
| `removeChild(child)` | Splices from `childNodes`, nullifies parent refs, calls `_onChildRemoved(child)` |
| `insertBefore(newChild, refChild)` | Inserts before reference child in `childNodes` array, calls `_onChildInserted(newChild, refChild)` |
| `replaceChild(newChild, oldChild)` | Calls `insertBefore` then `removeChild` |

**Document fragment handling:** Both `appendChild` and `insertBefore` detect `DOCUMENT_FRAGMENT_NODE` children and iterate their contents, appending/inserting each child individually (matching browser behavior).

### ShimElement

Maps HTML tag names to engine node types:

| Tag Category | Tags | Engine Node Type |
|-------------|------|-----------------|
| Layout containers (frames) | `div`, `section`, `header`, `footer`, `nav`, `main`, `article`, `aside`, `ul`, `ol`, `form`, `fieldset`, `figure`, `table`, `tr`, `td`, etc. | Frame |
| Text containers | `span`, `p`, `h1`-`h6`, `a`, `label`, `strong`, `em`, `b`, `i`, `small`, `code`, `pre`, `li` | Text (if no element children) or Frame with text child |
| Images | `img` | Frame with image fill |
| Buttons | `button` | Frame (clickable) |
| Inputs | `input`, `textarea` | TextInput pattern |

**Engine lifecycle:**

1. `_createInEngine(parentEngineId)` -- Called when the element is inserted into a tree rooted at an engine-connected node. Sets the insert parent via `setInsertParent()`, then calls `engineCreateFrame()` or `engineCreateText()` depending on tag classification. After creation, calls `style._syncNow()` to apply initial styles, then recursively creates children.
2. `_destroyInEngine()` -- Called on removal. Recursively destroys children first, then calls `engineDeleteNode()`.
3. `_onChildInserted(child)` -- If this element is already engine-created, creates the child in the engine.
4. `_onChildRemoved(child)` -- Destroys the child in the engine.

### ShimTextNode

Represents DOM text nodes. When inserted into an engine-connected parent, creates an engine Text node. When `textContent` / `nodeValue` / `data` is set on an already-created node, calls `engineSetProp(id, 'text', value)` to update the engine. Whitespace-only text nodes are skipped (not created in the engine).

---

## 5. Style System

### CSSStyleDeclaration Proxy (`style.js`)

`createStyleProxy(element)` returns a `Proxy` object that intercepts all property access on `element.style`. This is the core mechanism for translating CSS into engine calls.

**Write path:**

```
element.style.backgroundColor = '#ff0000'
  --> Proxy set trap
    --> expandShorthand('backgroundColor', '#ff0000')  // returns null, not a shorthand
    --> _values.backgroundColor = '#ff0000'
    --> scheduleSync()
      --> syncToEngine(element)
        --> parseColor('#ff0000') --> [1, 0, 0, 1]
        --> engineSetProp(id, 'fill', {r:1, g:0, b:0, a:1})
```

**CSS-to-engine property mapping:**

| CSS Property | Engine Call | Notes |
|-------------|-----------|-------|
| `width`, `height`, `minWidth`, `minHeight` | `engineSetProp(id, 'size', {w, h})` | Resolved via `parseLength`; capped by `maxWidth`/`maxHeight` |
| `left`, `top` (with `position: absolute/fixed`) | `engineSetProp(id, 'position', {x, y})` | Only applied for absolute/fixed positioning |
| `backgroundColor` | `engineSetProp(id, 'fill', {r, g, b, a})` | Color parsed to 0-1 RGBA |
| `backgroundImage` (linear-gradient) | `engineSetProp(id, 'linearGradient', ...)` | Gradient stops with positions and colors |
| `borderRadius` | `engineSetProp(id, 'cornerRadius', {tl, tr, br, bl})` | Individual corners supported |
| `opacity` | `engineSetProp(id, 'opacity', float)` | Direct passthrough |
| `display: flex`, `flexDirection`, `gap`, `padding*` | `engineSetProp(id, 'autoLayout', {...})` | Direction: 0=row, 1=column |
| `fontSize` | `engineSetProp(id, 'fontSize', px)` | Text elements only |
| `fontWeight` | `engineSetProp(id, 'fontWeight', numeric)` | Parsed via `parseFontWeight` |
| `fontFamily` | `engineSetProp(id, 'fontFamily', name)` | Quotes stripped |
| `textAlign` | `engineSetProp(id, 'textAlign', value)` | Text elements only |
| `color` (on text elements) | `engineSetProp(id, 'fill', {r, g, b, a})` | Text fill color |
| `boxShadow` | `engineSetProp(id, 'shadow', {...})` | Offset, blur, spread, color |
| `border` / `borderWidth` / `borderColor` | `engineSetProp(id, 'stroke', {r, g, b, a, weight})` | Simplified: single stroke |
| `transform: rotate(Xdeg)` | `engineSetProp(id, 'rotation', degrees)` | Only rotation extracted |
| `overflow: hidden` | `engineSetProp(id, 'clipContent', true)` | Clip children |

**Shorthand expansion:** The `set` trap calls `expandShorthand(prop, value)` before storing. Supported shorthands:

| Shorthand | Expansion |
|-----------|-----------|
| `margin` | `marginTop`, `marginRight`, `marginBottom`, `marginLeft` |
| `padding` | `paddingTop`, `paddingRight`, `paddingBottom`, `paddingLeft` |
| `borderRadius` | Individual corner radii |
| `gap` | `rowGap`, `columnGap` |
| `flex` | `flexGrow`, `flexShrink`, `flexBasis` |
| `background` | `backgroundColor` or `backgroundImage` (gradient detection) |

**Sync timing:** Style sync is immediate (not microtask-batched). This was a deliberate design decision -- microtask batching caused first-frame 0x0 nodes. Since `engineSetProp` already queues operations for batch execution before render, additional batching is unnecessary.

---

## 6. CSS Parsing (`css-values.js`)

### `parseColor(str) --> [r, g, b, a] | null`

Parses CSS color values to an RGBA array with components in the 0-1 range.

**Supported formats:**

| Format | Example |
|--------|---------|
| Named colors | `'red'`, `'cornflowerblue'`, `'transparent'` (90+ named colors) |
| Hex (3-digit) | `'#f0f'` |
| Hex (6-digit) | `'#ff00ff'` |
| Hex (8-digit, with alpha) | `'#ff00ff80'` |
| `rgb()` / `rgba()` comma syntax | `'rgba(255, 0, 255, 0.5)'` |
| `rgb()` modern space syntax | `'rgb(255 0 255 / 0.5)'` |

Returns `null` for `'none'`, `'inherit'`, `'initial'`, `'unset'`. Falls back to `[0, 0, 0, 1]` (black) for unrecognized values.

### `parseLength(value, base?) --> number`

Converts CSS length values to pixels.

| Unit | Conversion |
|------|------------|
| `px` | Direct value |
| `rem` | Value * 16 |
| `em` | Value * base (default 16) |
| `vh` | Value * viewportHeight / 100 |
| `vw` | Value * viewportWidth / 100 |
| `%` | Value / 100 * base (default: viewport width) |
| `auto`, `none` | 0 |
| Bare number | Direct value |

Viewport dimensions are set via `setViewport(w, h)` (defaults: 1280x800).

### `parseFontWeight(value) --> number`

Maps CSS font-weight keywords to numeric values:

| Keyword(s) | Value |
|-----------|-------|
| `thin`, `hairline` | 100 |
| `extralight`, `ultralight` | 200 |
| `light` | 300 |
| `normal`, `regular` | 400 |
| `medium` | 500 |
| `semibold`, `demibold` | 600 |
| `bold` | 700 |
| `extrabold`, `ultrabold` | 800 |
| `black`, `heavy` | 900 |

### `parseBoxShadow(value) --> {ox, oy, blur, spread, r, g, b, a} | null`

Parses CSS box-shadow strings. Expects format: `"Xpx Ypx Bpx [Spx] color"`. Returns `null` for `'none'`.

### `parseLinearGradient(value) --> {startX, startY, endX, endY, positions, colors} | null`

Parses `linear-gradient(...)` values. Supports angle (`180deg`) and direction (`to right`) syntax. Returns gradient with:
- Start/end points as 0-1 coordinates derived from the angle
- `positions` as a `Float32Array` of stop positions (0-1)
- `colors` as a `Float32Array` of flattened RGBA values

### `expandShorthand(prop, value) --> {key: value} | null`

Expands CSS shorthand properties into their individual longhand properties. Returns `null` if the property is not a shorthand. Handles 1/2/3/4-value box model expansion for `margin` and `padding`.

---

## 7. Engine Bridges

Both bridges expose an identical API surface. The build system swaps between them using an esbuild alias (see Section 10).

### Shared API

```js
initEngine(engine?, canvas?)   // Initialize the engine and start render loop
getEngine()                    // Get the engine instance (null on native)
getCanvas()                    // Get the canvas element (null on native)
allocId()                      // Monotonic ID allocator for shim nodes
markDirty()                    // Flag the scene as needing re-render

// Node registry
registerNode(engineId, counter, clientId)
unregisterNode(engineId)
getNodeIds(engineId)           // --> {counter, clientId} | null

// Tree operations
setInsertParent(engineId)      // Set parent context before creating child
clearInsertParent()            // Clear parent context

// Node CRUD
engineCreateFrame(engineId, name)   // --> {counter, clientId}
engineCreateText(engineId, name, text)  // --> {counter, clientId}
engineDeleteNode(engineId)

// Properties
engineSetProp(engineId, prop, value)

// Queries
engineGetBounds(engineId)      // --> {x, y, width, height}  (flushes ops first)
hitTest(screenX, screenY)      // --> engineId | null  (flushes ops first)
```

### `engine.js` -- WASM Bridge

**Ops queue pattern:** All engine mutations are pushed as closures to an `_ops` array. The queue is flushed once per frame in the render loop, right before `render_canvas2d()`. This serialization is critical because the WASM engine uses `&mut self` borrows -- concurrent calls would panic.

```
[style change] --> _enqueue(fn)
[appendChild]  --> _enqueue(fn)
[setAttribute] --> _enqueue(fn)
                       |
                       v
            requestAnimationFrame loop:
              1. _flushOps()   // execute all queued closures
              2. render_canvas2d()  // paint to canvas
```

**Node registry:** Maps `shimNode._engineId` (a monotonic integer) to `{counter, clientId}` -- the two-part identifier the engine uses internally. The registry is populated when `engineCreateFrame`/`engineCreateText` ops execute.

**Render loop:** `_startRenderLoop()` uses `requestAnimationFrame`. Each frame:
1. Flushes all pending ops
2. If `_dirty` is true, calls `engine.render_canvas2d(ctx, width, height, dpr)`

**Bounds queries:** `engineGetBounds` and `hitTest` call `_flushOps()` synchronously before querying, ensuring the engine state is up-to-date.

### `engine-native.js` -- Native FFI Bridge

Drop-in replacement for `engine.js`. Instead of calling methods on a WASM `CanvasEngine` instance, calls global `__rendero_*` functions that Swift registers in the JSContext before loading the JS bundle.

**Key differences from WASM bridge:**

| Aspect | WASM (`engine.js`) | Native (`engine-native.js`) |
|--------|-------------------|---------------------------|
| Engine instance | `CanvasEngine` JS object | No JS object; global `__rendero_*` functions |
| Tree creation | Queued (deferred) | **Synchronous** -- parent must be set before child is created |
| Property changes | Queued | Queued (same pattern) |
| Render trigger | `requestAnimationFrame` loop | `flushAndRender()` called by Swift's display link |
| Canvas | Real `<canvas>` element | None; Swift renders to `MTKView` |
| ID encoding | Array return `[counter, clientId]` | Packed integer: `counter * 0x100000000 + clientId` |

**Native FFI functions expected (registered by Swift):**

```
__rendero_set_viewport(w, h)
__rendero_set_camera(x, y, zoom)
__rendero_add_frame(name, x, y, w, h, r, g, b, a) --> packedId
__rendero_add_text(name, text, x, y, fontSize, r, g, b, a) --> packedId
__rendero_set_insert_parent(counter, clientId)
__rendero_clear_insert_parent()
__rendero_select_node(counter, clientId)
__rendero_delete_selected()
__rendero_set_node_position(counter, clientId, x, y)
__rendero_set_node_size(counter, clientId, w, h)
__rendero_set_node_fill(counter, clientId, r, g, b, a)
__rendero_set_node_corner_radius(counter, clientId, tl, tr, br, bl)
__rendero_set_node_opacity(counter, clientId, value)
__rendero_set_node_text(counter, clientId, text)
__rendero_set_node_font_size(counter, clientId, size)
__rendero_set_node_font_weight(counter, clientId, weight)
__rendero_set_auto_layout(counter, clientId, dir, spacing, pt, pr, pb, pl)
__rendero_get_node_bounds(counter, clientId) --> {x, y, w, h}
__rendero_get_camera() --> {x, y, zoom}
__rendero_request_render()
__rendero_log(message)
```

---

## 8. Event System (`events.js`)

### Event Classes

| Class | Extends | Additional Properties |
|-------|---------|----------------------|
| `ShimEvent` | -- | `type`, `bubbles`, `cancelable`, `target`, `currentTarget`, `defaultPrevented`, `eventPhase`, `timeStamp` |
| `ShimMouseEvent` | `ShimEvent` | `clientX/Y`, `pageX/Y`, `screenX/Y`, `button`, `buttons`, `ctrlKey`, `shiftKey`, `altKey`, `metaKey` |
| `ShimKeyboardEvent` | `ShimEvent` | `key`, `code`, `ctrlKey`, `shiftKey`, `altKey`, `metaKey`, `repeat` |
| `ShimFocusEvent` | `ShimEvent` | `relatedTarget` |
| `ShimInputEvent` | `ShimEvent` | `data`, `inputType` |

### EventTargetMixin

Mixed into `ShimNode.prototype` via `Object.assign`. Gives every node:

- `addEventListener(type, handler, options)` -- Supports `capture`, `once`, `passive` options. Deduplicates by handler+capture pair.
- `removeEventListener(type, handler, options)` -- Matches by handler+capture.
- `dispatchEvent(event)` -- Full W3C-compliant three-phase dispatch:
  1. **Capture phase** (root to target) -- fires listeners with `capture: true`
  2. **Target phase** -- fires both capture and bubble listeners on the target
  3. **Bubble phase** (target to root) -- fires listeners with `capture: false`, only if `event.bubbles` is true

The `handleEvent` pattern (passing an object with `handleEvent` method instead of a function) is supported.

### Canvas Event Wiring (`index.js`)

In WASM mode, `_wireCanvasEvents(canvas, shimDoc)` translates real browser events on the `<canvas>` element into shim events:

| Browser Event | Shim Behavior |
|--------------|---------------|
| `mousedown` on canvas | `hitTest(x, y)` to find engine node, walk shim tree to find element, dispatch `ShimMouseEvent('click')` |
| `mousemove` on canvas | Track `lastHover`, dispatch `mouseout`/`mouseleave` on previous, `mouseover`/`mouseenter` on new |
| `touchstart` / `touchmove` / `touchend` | Detect taps (not scrolls), dispatch `click` on touch end |
| `wheel` on canvas | Prevents default, marks dirty (per-element scroll not yet implemented) |

---

## 9. Entry Points

### Mode 1: Web (Real DOM)

**React** (`entry-react-web.jsx`):
```js
import { createRoot } from 'react-dom/client';
const root = createRoot(document.getElementById('root'));
root.render(<AppleApp />);
```

**Vue** (`entry-vue-web.js`):
```js
import { createApp } from 'vue';
const app = createApp(AppleApp);
app.mount('#root');
```

No shim involved. Standard framework usage against the real browser DOM.

### Mode 2: Web Canvas (DOM Shim + WASM)

**React** (`entry-react-native.jsx`):
```js
const shim = await installShim(canvas);
const container = shim.getContainer();
const root = createRoot(container);  // ShimElement, not a real DOM node
root.render(<AppleApp />);
```

**Vue** (`entry-vue-native.js`):
```js
const shim = await installShim(canvas);
const container = shim.getContainer();
const app = createApp(AppleApp);
app.mount(container);  // ShimElement, not a real DOM node
```

`installShim(canvas)` initializes the WASM engine, creates a `ShimDocument`, wires canvas events, and returns `{document, window, engine, getContainer()}`. The `getContainer()` helper creates a `<div id="root">` in the shim body.

### Mode 3: Native macOS (DOM Shim + Rust FFI)

**React** (`entry-macos.jsx`):

This entry point is built as an IIFE (not ESM) for JavaScriptCore. It:

1. Sets viewport from `__screenWidth` / `__screenHeight` globals (provided by Swift)
2. Calls `initEngine()` from `engine-native.js`
3. Creates `ShimDocument` and connects to engine
4. Sets `shimDoc.defaultView = globalThis` (critical for react-dom's `instanceof` checks)
5. Creates root container, mounts React
6. Exports `__shimFlushAndRender` to `self`/`global` for Swift's display link to call

---

## 10. Build System (`build.mjs`)

Uses esbuild to produce 5 bundles from the entry points.

### Web Builds (ESM)

```js
const common = {
    bundle: true,
    format: 'esm',
    target: 'es2020',
    minify: false,
    sourcemap: true,
    plugins: [renderoPlugin],
};
```

The `renderoPlugin` marks the WASM module import (`rendero.js`) as external, rewriting the path to `../../../pkg/rendero.js` relative to the output `dist/` directory.

### Native Build (IIFE)

```js
const nativeCommon = {
    bundle: true,
    format: 'iife',       // Self-contained, no import/export
    target: 'es2020',
    minify: false,
    sourcemap: false,
    plugins: [nativeEnginePlugin],
};
```

**The native engine alias** is the key mechanism that makes Mode 3 work. The `nativeEnginePlugin` intercepts any import of `engine.js` from within the `shims/` directory and rewrites it to `engine-native.js`:

```js
build.onResolve({ filter: /engine\.js$/ }, (args) => {
    if (args.importer.includes('shims/')) {
        return { path: args.importer.replace(/[^/\\]+$/, 'engine-native.js') };
    }
});
```

This means `node.js`, `element.js`, `style.js`, `text-node.js`, and `index.js` all import from `engine.js` in source, but when building the macOS bundle, esbuild silently redirects those imports to `engine-native.js`. No source changes needed.

### Output Bundles

| Bundle | Entry | Format | Engine |
|--------|-------|--------|--------|
| `dist/react-web.js` | `entry-react-web.jsx` | ESM | None (real DOM) |
| `dist/react-native.js` | `entry-react-native.jsx` | ESM | WASM |
| `dist/vue-web.js` | `entry-vue-web.js` | ESM | None (real DOM) |
| `dist/vue-native.js` | `entry-vue-native.js` | ESM | WASM |
| `dist/macos-bundle.js` | `entry-macos.jsx` | IIFE | Native FFI |

---

## 11. Known Issues

### Elements with height 0 from unresolved CSS

Elements may render with zero height when:

- Their size depends on CSS properties that the shim does not resolve (e.g., percentage heights without an explicit parent height).
- `parseLength` returns 0 for `'auto'` and `'none'`, which means content-sized elements get no explicit size set in the engine. The engine's auto-layout handles sizing for flex children, but non-flex children with no explicit size will collapse.
- Microtask-based style batching was previously attempted but caused first-frame 0x0 nodes. The current approach uses immediate sync, but the underlying issue is that the engine needs explicit dimensions when auto-layout is not in use.

### `instanceof` errors in JSC/QuickJS

React-dom performs checks like `element instanceof HTMLIFrameElement` internally (e.g., in `getActiveElementDeep`). In JavaScriptCore and QuickJS:

- There is no `HTMLIFrameElement` or `HTMLElement` constructor on `globalThis`.
- The `entry-macos.jsx` entry point works around this by setting `shimDoc.defaultView = globalThis` and relying on Swift to polyfill the required constructors (e.g., `globalThis.HTMLIFrameElement = function(){}`) before loading the JS bundle.
- If these polyfills are missing, react-dom will throw `ReferenceError: Can't find variable: HTMLIFrameElement`.

### Additional limitations

- **No CSS inheritance/cascade** -- `getComputedStyle` returns directly-set values only, not inherited or cascading styles.
- **No CSS class resolution** -- Setting `className` or `classList` does not resolve styles from stylesheets. Only inline styles are processed.
- **No full HTML parsing** -- `innerHTML` handles plain text only; HTML tags in `innerHTML` are ignored.
- **Scroll containers not implemented** -- `wheel` events mark the scene dirty but do not perform per-element scrolling.
- **Hit testing is linear** -- `hitTest` iterates all registered nodes. No spatial index. Acceptable for typical UI node counts but would degrade on very large scenes.
- **SVG elements** -- `createElementNS` creates Frame nodes regardless of namespace. SVG rendering is not supported.
- **TreeWalker** -- Minimal implementation for react-dom compatibility. Does not implement `whatToShow` filtering or `NodeFilter`.
