# Rendero Native Shell & FFI

## 1. Overview

Rendero ships two crates for running React-rendered UI natively on desktop, without a browser or WebView:

| Crate | Purpose | Dependencies |
|---|---|---|
| `rendero-native-shell` | Pure Rust executable. Opens a window, embeds QuickJS, renders pixels, blits to the window surface. | winit 0.30, softbuffer 0.4, rquickjs 0.11, rendero-core, rendero-renderer |
| `rendero-native-ffi` | C-ABI dynamic/static library. Exposes 22 `extern "C"` functions for Swift, C++, or Kotlin callers. | rendero-core, rendero-renderer, rendero-crdt |

The native shell is cross-platform (macOS, Windows, Linux) because it uses only Rust-native windowing (winit) and software rendering (softbuffer). There is no GPU API, no Metal/Vulkan, and no WebView. The rendering pipeline is the same CPU rasterizer used in the WASM build (`rendero-renderer`), producing raw RGBA pixel buffers that are blitted directly to the OS window surface.

**Run it:**

```bash
cargo run -p rendero-native-shell
```

The binary name is `rendero` (defined in `Cargo.toml` as `[[bin]] name = "rendero"`).


## 2. Architecture

The system forms a pipeline with five stages, executed once per frame:

```
 [1] Poll OS events (winit)
      |
 [2] Drain JS callbacks (QuickJS pending timers + Promise jobs)
      |
 [3] Flush engine ops (__shimFlushAndRender)
      |
 [4] Render pixels (rendero-renderer CPU rasterizer)
      |
 [5] Blit to window surface (softbuffer)
```

### Data flow detail

```
React JSX
   |  (reconciler commits via DOM shim)
   v
DOM shim elements  -->  queued __rendero_* calls
   |
   v  (__shimFlushAndRender)
Engine::dispatch()  -->  Document tree mutations
   |
   v
build_scene()  -->  pipeline::render_items()  -->  RGBA Vec<u8>
   |
   v
softbuffer Surface::buffer_mut()  -->  OS window
```

React runs inside QuickJS. The DOM shim intercepts React DOM operations (createElement, appendChild, etc.) and translates them into `__rendero_*` function calls. These calls mutate the Rendero `Document` tree. Each frame, the renderer builds a scene from the tree, rasterizes it to an RGBA pixel buffer, and the shell copies those pixels into the OS window surface.


## 3. JSRuntime Trait

**File:** `crates/native-shell/src/providers.rs`

The JS runtime is abstracted behind a trait so QuickJS can be swapped for Hermes or another engine later.

```rust
pub trait JSRuntime {
    fn evaluate(&mut self, code: &str) -> Result<(), String>;
    fn drain_pending_jobs(&mut self);
    fn has_global(&mut self, name: &str) -> bool;
    fn call_global(&mut self, name: &str) -> Result<(), String>;
}
```

| Method | Purpose |
|---|---|
| `evaluate` | Evaluate a JS string. Returns `Err(message)` on failure. |
| `drain_pending_jobs` | Pump all pending async work: setTimeout callbacks, Promise microtasks, MessageChannel posts. Called once per frame. |
| `has_global` | Check if a named global function exists in the JS context. |
| `call_global` | Call a global JS function with no arguments. Used to invoke `__shimFlushAndRender`. |


## 4. QuickJS Runtime

**File:** `crates/native-shell/src/quickjs_runtime.rs`

### The problem

QuickJS does not have `setTimeout`, `setInterval`, or `queueMicrotask` built in. React's scheduler depends on all three (primarily `setTimeout` and `MessageChannel.postMessage`) to schedule reconciliation work.

### The solution: `__pendingTimers` array

On construction, `QuickJSRuntime::new()` evaluates a JS snippet that installs polyfills:

```javascript
var __pendingTimers = [];
function setTimeout(fn, delay) {
    __pendingTimers.push(fn);
    return __pendingTimers.length;
}
function clearTimeout(id) {}
function setInterval(fn, ms) { return setTimeout(fn, ms); }
function clearInterval(id) {}
function queueMicrotask(fn) { __pendingTimers.push(fn); }
```

All timer/microtask callbacks are pushed into a single JS array. The `delay` parameter is ignored -- callbacks fire on the next drain cycle, which happens once per frame.

### Drain loop

`drain_pending_jobs()` runs up to 20 iterations. Each iteration:

1. **Drain QuickJS internal Promise jobs** -- calls `runtime.execute_pending_job()` in a loop until it returns `false`.
2. **Check `__pendingTimers.length > 0`** -- if empty, break early.
3. **Splice and execute all pending timers** -- `var __batch = __pendingTimers.splice(0)`, then iterate and call each callback inside a try/catch.
4. **Drain Promise jobs again** -- timer callbacks may have resolved Promises that enqueue more jobs.

The 20-iteration cap prevents infinite loops if callbacks keep scheduling more callbacks. In practice, React's scheduler typically settles within 2-3 iterations.

### Context access

```rust
pub fn with_context<F, R>(&self, f: F) -> R
where F: FnOnce(rquickjs::Ctx<'_>) -> R
```

Used by the engine bridge to register native functions in the QuickJS context.


## 5. Engine Bridge

**File:** `crates/native-shell/src/engine_bridge.rs`

### Engine struct

```rust
pub struct Engine {
    pub document: Document,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub cam_x: f32,
    pub cam_y: f32,
    pub cam_zoom: f32,
    pub insert_parent: Option<NodeId>,
    pub current_page: usize,
}
```

The `Engine` owns a `Document` (the Rendero scene graph) and camera state. It provides two dispatch methods and a pixel renderer.

### Dispatch pattern

Rather than registering many individual Rust closures (which causes rquickjs lifetime issues with multiple captured `Rc<RefCell<Engine>>`), the bridge registers exactly **two** Rust functions in QuickJS:

| Global function | Signature | Purpose |
|---|---|---|
| `__rendero_dispatch` | `(cmd: String, args: Vec<f64>) -> f64` | Numeric-only commands |
| `__rendero_dispatch_str` | `(cmd: String, args: Vec<f64>, text: String) -> f64` | Commands that include a string argument |

Individual `__rendero_*` JS wrapper functions are then defined in pure JS that call one of these two dispatchers. For example:

```javascript
function __rendero_add_frame(name, x, y, w, h, r, g, b, a) {
    return __rendero_dispatch_str('add_frame_named', [x, y, w, h, r, g, b, a], name);
}
function __rendero_set_node_position(c, ci, x, y) {
    __rendero_dispatch('set_node_position', [c, ci, x, y]);
}
```

### Supported dispatch commands

| Command | Args | Returns | Description |
|---|---|---|---|
| `set_viewport` | `[w, h]` | 0 | Set viewport dimensions |
| `set_camera` | `[x, y, zoom]` | 0 | Set camera position and zoom |
| `set_insert_parent` | `[counter, client_id]` | 0 | Set parent for next node creation |
| `clear_insert_parent` | `[]` | 0 | Reset parent to page root |
| `add_frame` | `[x, y, w, h, r, g, b, a]` | packed ID | Create a frame node |
| `add_text` | `[x, y, fontSize, r, g, b, a]` | packed ID | Create a text node |
| `set_node_position` | `[counter, client_id, x, y]` | 0 | Move a node |
| `set_node_size` | `[counter, client_id, w, h]` | 0 | Resize a node |
| `set_node_fill` | `[counter, client_id, r, g, b, a]` | 0 | Set fill color |
| `set_node_corner_radius` | `[counter, client_id, tl, tr, br, bl]` | 0 | Set corner radii |
| `set_node_opacity` | `[counter, client_id, opacity]` | 0 | Set opacity (0.0-1.0) |
| `set_node_font_size` | `[counter, client_id, size]` | 0 | Set text font size |
| `set_node_font_weight` | `[counter, client_id, weight]` | 0 | Set text font weight |
| `set_auto_layout` | `[counter, client_id, dir, spacing, pt, pr, pb, pl]` | 0 | Enable flexbox auto-layout |

String-taking commands (via `__rendero_dispatch_str`):

| Command | Text arg | Description |
|---|---|---|
| `add_frame_named` | frame name | Create frame with explicit name |
| `add_text_named` | `"name\0content"` | Create text with name and content (NUL-separated) |
| `set_node_text` | text content | Update text node content |

### Node ID packing

Node IDs are returned as `f64` to JS, packed as `(counter << 32) | client_id`. JS code unpacks with bitwise operations to pass `counter` and `client_id` back to subsequent calls.

### Browser polyfills

The constant `BROWSER_POLYFILLS` (injected before the React bundle) provides minimal stubs for browser globals that React and the DOM shim expect:

| Polyfill | What it provides |
|---|---|
| `window`, `self`, `global` | Aliases to `globalThis` |
| `navigator` | `{ userAgent: 'RenderoNative/1.0', platform: 'macOS' }` |
| `document` | Mock DOM: `createElement`, `createTextNode`, `createComment`, `body`, `documentElement`, `querySelector`, etc. |
| `HTMLElement`, `HTMLInputElement`, etc. | Empty constructor stubs |
| `Event`, `CustomEvent` | Minimal event with `type`, `preventDefault`, `stopPropagation` |
| `MutationObserver` | No-op observer |
| `requestAnimationFrame` | Routes through `setTimeout(cb, 16)` |
| `MessageChannel` | Two ports; `port2.postMessage` calls `port1.onmessage` via `setTimeout(0)` |
| `performance.now` | Returns `Date.now()` |
| `console.log/warn/error` | Routes to `__rendero_log` which prints to stdout |

### render_pixels

`Engine::render_pixels(width, height) -> Vec<u8>` performs the full render pipeline:

1. Compute Taffy layout on the document tree.
2. Build the scene (`rendero_renderer::scene::build_scene`).
3. Rasterize (`pipeline::render_items`).
4. Return RGBA pixel buffer (`output.to_pixels`).


## 6. Event Loop

**File:** `crates/native-shell/src/main.rs`

### App struct

```rust
struct App {
    window: Option<Window>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    window_rc: Option<Rc<Window>>,
    engine: Rc<RefCell<Engine>>,
    js: QuickJSRuntime,
    initialized: bool,
    frame_count: u64,
}
```

### Initialization sequence (App::new)

1. Create `Engine` with name "RenderoNative", client_id 1.
2. Create `QuickJSRuntime`.
3. Register engine dispatch functions in the JS context via `register_engine_functions`.
4. Evaluate `BROWSER_POLYFILLS`.
5. Find and load the JS bundle (`macos-bundle.js`) from several candidate paths.
6. **Critical:** Drain pending jobs immediately -- React's scheduler posts the initial commit via setTimeout/MessageChannel. Without draining, the commit never executes.
7. Call `__shimFlushAndRender` to flush queued engine operations.
8. Drain again (flushing may trigger more callbacks).

### ApplicationHandler implementation

| Event | Handler behavior |
|---|---|
| `resumed` | Create the winit window (1024x768 logical), create softbuffer context and surface. Runs once. |
| `WindowEvent::CloseRequested` | Exit the event loop. |
| `WindowEvent::Resized` | Resize the softbuffer surface. |
| `WindowEvent::RedrawRequested` | Call `render_frame()`. |
| `about_to_wait` | Request a redraw, creating a continuous render loop. |

### render_frame

Each frame:

1. `js.drain_pending_jobs()` -- pump React's scheduler.
2. `js.call_global("__shimFlushAndRender")` -- flush DOM shim operations to the engine.
3. Read physical window size. Update engine viewport if changed.
4. `engine.render_pixels(w, h)` -- get RGBA pixels.
5. Convert RGBA `[u8]` to `[u32]` in `0x00RRGGBB` format (softbuffer's expected format).
6. Copy to `surface.buffer_mut()` and call `buffer.present()`.

### Viewport scaling

The window opens at 1024x768 **logical** pixels. On Retina displays, the physical size is 2048x1536. The engine renders at physical pixel dimensions with `cam_zoom = 1.0`, meaning 1 engine unit = 1 physical pixel. Content authored at 1024 units wide would fill half a 2048px Retina surface.


## 7. Native FFI

**File:** `crates/native-ffi/src/lib.rs`
**Header:** `crates/native-ffi/rendero.h`

The native-ffi crate wraps the same Rendero engine (rendero-core + rendero-renderer) behind a C-ABI boundary. It builds as both `cdylib` (shared library) and `staticlib` (static archive).

### Build

```bash
cargo build --release -p rendero-native-ffi
# Produces: target/release/librendero_native_ffi.dylib (macOS)
#           target/release/librendero_native_ffi.a     (static)
```

### Exported functions (22 total)

| Category | Functions |
|---|---|
| **Lifecycle** | `rendero_create`, `rendero_destroy` |
| **Viewport/Camera** | `rendero_set_viewport`, `rendero_set_camera`, `rendero_get_camera` |
| **Insert Parent** | `rendero_set_insert_parent`, `rendero_clear_insert_parent` |
| **Node Creation** | `rendero_add_frame`, `rendero_add_text` |
| **Node Properties** | `rendero_set_node_position`, `rendero_set_node_size`, `rendero_set_node_fill`, `rendero_set_node_corner_radius`, `rendero_set_node_opacity`, `rendero_set_node_text`, `rendero_set_node_font_size`, `rendero_set_node_font_weight`, `rendero_set_auto_layout` |
| **Selection** | `rendero_select_node`, `rendero_delete_selected` |
| **Queries** | `rendero_get_node_bounds` |
| **Rendering** | `rendero_render_pixels` |

### NativeEngine vs Engine

The FFI crate has its own `NativeEngine` struct (not shared with native-shell's `Engine`). Key differences:

| Feature | native-shell `Engine` | native-ffi `NativeEngine` |
|---|---|---|
| Scene cache | None | `Option<Vec<RenderItem>>` with `invalidate_cache()` |
| Selection | Not tracked | `Vec<NodeId>` with select/delete operations |
| World position query | Not available | `node_world_pos()` walks parent chain |
| JS integration | Dispatch via QuickJS closures | No JS -- pure C-ABI, caller manages JS |

### ID packing convention

Both create functions (`rendero_add_frame`, `rendero_add_text`) return a `u64` packed as `(counter << 32) | client_id`. The caller unpacks this to pass `counter` and `client_id` separately to subsequent property-setting functions.

### rendero_render_pixels

```c
void rendero_render_pixels(void* engine, uint8_t* buffer, uint32_t width, uint32_t height);
```

The caller allocates the pixel buffer (`width * height * 4` bytes). The function:

1. Runs Taffy layout.
2. Builds the scene with current camera/viewport.
3. Rasterizes to RGBA.
4. Copies pixels into the caller's buffer.

### Header file

`rendero.h` provides the full C declaration of all 22 functions, organized by category. It includes `extern "C"` guards for C++ compatibility. Link with `-lrendero_native_ffi`.


## 8. Key Lesson: The JS Event Loop Problem

### The problem with Swift + JavaScriptCore

The original approach (see section 9) used Swift/AppKit with JavaScriptCore (JSC). JSC has no built-in `setTimeout` either, so the Swift shell polyfilled it by pushing callbacks to a `__pendingCallbacks` JS array and draining them from a 60fps `Timer`.

This worked partially, but had a fundamental issue: **React's scheduler chains callbacks 3+ levels deep.** A single drain pass was not enough. The Swift code resorted to running the drain loop 3 times back-to-back with separate splice/execute blocks, which was fragile and hard to debug.

Additionally, JavaScriptCore's `evaluateScript` is synchronous, but the interaction between Swift's `DispatchQueue.main.async` and `Timer` callbacks created subtle ordering bugs where `queueMicrotask` callbacks would not fire during render frames.

### How QuickJS + manual drain solves it

The pure-Rust QuickJS approach solves this cleanly:

1. **Single drain function with iteration.** `drain_pending_jobs()` loops up to 20 times, interleaving QuickJS's own job queue drain (`execute_pending_job`) with the timer array drain. Callbacks scheduling more callbacks are handled naturally.

2. **No async boundary.** Everything runs synchronously in one thread. There is no `DispatchQueue`, no run loop interaction, no Timer callback scheduling ambiguity. The call sequence is deterministic: drain -> flush -> render -> blit.

3. **QuickJS's own Promise queue.** QuickJS has a built-in pending job queue for Promises. `runtime.execute_pending_job()` drains it synchronously. This means Promise chains (which React uses heavily) resolve completely within the drain loop, without needing to wait for a "next tick."

4. **No FFI overhead per call.** The native-shell's dispatch pattern (one Rust function handling all commands) avoids the per-function `@convention(block)` overhead of JSC.

The result: React's full reconciliation cycle (setState -> schedule -> reconcile -> commit -> flush to engine) completes synchronously within a single `render_frame()` call.


## 9. Legacy Swift Shell

**File:** `native/macos-app.swift`

### What it is

A single-file macOS application (no Xcode project required) that uses:
- **AppKit** for windowing (NSWindow + NSView)
- **JavaScriptCore** for running the React bundle
- **rendero-native-ffi** (C-ABI) for the engine

### Build command

```bash
cargo build --release -p rendero-native-ffi
swiftc native/macos-app.swift \
    -L target/release -lrendero_native_ffi \
    -framework AppKit -framework JavaScriptCore \
    -o native/RenderoApp && native/RenderoApp
```

### Architecture

```
Swift (AppKit)
  |
  +-- NSWindow + NSView (RenderoView)
  +-- JSContext (JavaScriptCore)
  +-- 60fps Timer -> renderFrame()
       |
       +-- Drain __pendingCallbacks (3 passes)
       +-- Call __shimFlushAndRender
       +-- rendero_render_pixels (via C FFI)
       +-- Blit RGBA to CALayer via CGImage
```

### How it registers engine functions

Each of the 22 FFI functions is wrapped in a `@convention(block)` Swift closure and registered as a JSContext global. For example:

```swift
let setPos: @convention(block) (UInt32, UInt32, Float, Float) -> Void = { c, ci, x, y in
    rendero_set_node_position(eng, c, ci, x, y)
}
jsContext.setObject(setPos, forKeyedSubscript: "__rendero_set_node_position" as NSString)
```

### Why it was superseded

| Issue | Detail |
|---|---|
| **macOS-only** | AppKit and JavaScriptCore lock it to Apple platforms. |
| **Fragile callback drain** | Required 3 separate splice-and-execute passes hardcoded in `renderFrame()`. Adding more passes was trial-and-error. |
| **Timer-based rendering** | `Timer.scheduledTimer` at 1/60s is not synchronized with display refresh. |
| **Two-language build** | Requires both `cargo build` (for the FFI lib) and `swiftc` (for the app). The pure-Rust shell is a single `cargo run`. |
| **Debugging difficulty** | Swift's `print()` does not flush to piped stdout; the file had to use `fopen("/tmp/rendero-app.log", "w")` as a workaround. |
| **CGImage blit overhead** | Each frame allocates a `CGDataProvider` and `CGImage`. The softbuffer approach in the Rust shell copies directly to a memory-mapped window surface. |

The pure-Rust native-shell (`crates/native-shell`) replaces this with a single-binary, cross-platform solution that eliminates all of the above issues.
