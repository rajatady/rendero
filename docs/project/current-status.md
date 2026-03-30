# Current Status and Next Steps

Last updated: 2026-03-31

---

## What Renders Today

### TestApp (colored rectangles)
**Status: Pixel-perfect on both web and native.**

```jsx
<div style={{ width: '100%', height: '100vh', backgroundColor: '#ffffff', display: 'flex', flexDirection: 'column' }}>
    <div style={{ width: '100%', height: '100px', backgroundColor: '#ff0000' }} />
    <div style={{ width: '100%', height: '100px', backgroundColor: '#00ff00' }} />
    <div style={{ width: '100%', height: '100px', backgroundColor: '#0000ff' }} />
    <div style={{ width: '50%', height: '200px', backgroundColor: '#ff00ff' }} />
</div>
```

- Red, green, blue bars stacked vertically (Taffy flex column layout)
- Magenta bar at 50% width
- White background fills remaining space
- Identical on browser DOM and native Rust engine

### Apple Website
**Status: Partially renders on native. Text visible, layout incomplete.**

What shows:
- Navigation bar text (Store, Mac, iPad, iPhone, Watch...)
- "iPhone 16 Pro" heading
- "Hello, Apple Intelligence." subtitle
- "Learn more > Buy >" links
- "MacBook Pro" card title
- Dark backgrounds for hero/CTA sections

What's missing:
- Sections don't stack vertically (many heights are 0)
- Product cards overlap instead of forming a grid
- Feature icons don't show (gradient backgrounds not applied)
- Footer text doesn't flow

---

## The Blocking Issue: CSS Height Resolution

The single issue preventing the Apple page from rendering correctly:

**`style.js` sends `height: 0` for elements that don't have explicit `height` in pixels.**

Examples:
```
// These resolve to 0:
style={{ padding: '120px 0 60px' }}        // height from padding only
style={{ minHeight: '100vh' }}             // minHeight not mapped
style={{ height: '100vh' }}                // works NOW (after fix)
style={{ flex: 1 }}                        // flex-grow not mapped to engine
```

The fix is in one file: `docs/demos/dom-shim/shims/style.js`

What needs to change:
1. Map `padding` to Taffy padding (not just engine padding)
2. Map `minHeight` / `maxHeight` to Taffy min/max constraints
3. Map `flex: 1` / `flexGrow` to Taffy flex properties
4. Map `alignItems`, `justifyContent` to Taffy alignment
5. Map `flexWrap: 'wrap'` for the product grid

None of these require Rust changes. The engine and Taffy already support all of it. The shim just isn't translating all CSS properties to engine calls.

---

## Short-Term Runnable Path

These are ordered by impact. Each can be done independently.

### 1. Fix style.js CSS mapping (1-2 hours)
**Impact: Apple page renders with correct layout on both web and native.**

Add to `syncToEngine()` in `style.js`:
- `minHeight` / `maxHeight` via `engineSetProp('size', ...)` fallback
- `flexGrow` / `flexShrink` / `flexBasis` (currently ignored)
- `alignItems` / `justifyContent` via `engineSetProp('autoLayout', ...)`
- `flexWrap` flag
- `overflow: hidden` flag

### 2. Remove debug logging (30 min)
**Impact: Clean console, better performance.**

Remove `CREATED`, `SYNC`, `CHILD_INSERT`, `FLUSH`, `RENDER` console logs from element.js, style.js, engine-native.js. They were added for debugging the event loop issue.

### 3. Fix `__screenWidth` hardcoding (30 min)
**Impact: Window resizing works correctly.**

Currently `__screenWidth = 2048` is hardcoded in `engine_bridge.rs`. Should read from the actual window size and update on resize.

### 4. Vue on native (1 hour)
**Impact: Proves framework-agnostic claim.**

Create `entry-vue-macos.js`, add to build.mjs with the engine-native alias. Same approach as React entry.

### 5. Click events on native (2 hours)
**Impact: Interactive apps (buttons, links).**

Map winit `WindowEvent::MouseInput` to DOM shim `dispatchEvent`. Hit-test via engine's node bounds. The shim's event system already handles bubbling.

---

## Medium-Term Path

### 6. Font resolution (fontdb integration)
Replace embedded RobotoMono with system font matching. `fontdb::Database::load_system_fonts()`. Provider trait `FontResolver` ready but no implementation yet.

### 7. Text measurement (parley or improved heuristic)
Replace `width = len * fontSize * 0.65` with actual glyph metrics. Either integrate parley or use fontdue's `Font::metrics()` for per-character advance widths.

### 8. GPU rendering (wgpu backend)
Replace `softbuffer` (CPU blit) with `wgpu` (Metal/Vulkan/DX12). The `RenderItem` list is the same input -- only the output stage changes.

### 9. iOS/Android
Same Rust code. `winit` supports iOS and Android targets. Need to set up `cargo-ndk` (Android) and Xcode build target (iOS) for the native-shell binary.

---

## Key Bugs Found and Fixed

| Bug | Root Cause | Fix |
|-----|-----------|-----|
| React commit never fires (Swift) | AppKit Timer blocks DispatchQueue.main.async | Replaced Swift with pure Rust shell (QuickJS has synchronous job drain) |
| React commit never fires (QuickJS) | QuickJS has no setTimeout built-in | Polyfilled via `__pendingTimers` JS array, drained each frame |
| Nodes created but not in scene | `element.js` imported `engine.js` (WASM) instead of `engine-native.js` | esbuild alias plugin swaps `engine.js` -> `engine-native.js` for native bundle |
| Children at wrong parent | `setInsertParent` called with deferred IDs (-1,-1) from ops queue | Made tree structure ops synchronous (not queued) |
| Content half-size on Retina | Rendering at physical pixels (2048) but content authored at logical (1024) | Set `__screenWidth` to physical size, `cam_zoom` to 1.0 |
| `instanceof` error in JSC/QuickJS | `element instanceof window.HTMLIFrameElement` where window is shim object | Set `shimDoc.defaultView = globalThis` which has polyfilled constructors |
| WASM reentrant borrow panic | render_canvas2d borrows &mut self while microtask creates nodes | All engine ops queued, flushed sequentially before render |

---

## How to Swap Between Test and Apple Content

In `docs/demos/dom-shim/src/apple-react.jsx`, last line:

```jsx
// Test colored rectangles:
export default function App() { return <TestApp />; }

// Full Apple website:
// export default function App() { return <AppleApp />; }
```

Then rebuild: `cd docs/demos/dom-shim && node build.mjs`

For native: `cargo run -p rendero-native-shell`
For web: refresh browser at `http://localhost:5555/demos/dom-shim/`
