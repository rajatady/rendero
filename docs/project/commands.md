# Commands Reference

All commands run from the repo root.

---

## Build Commands

### WASM (for web demos)
```bash
wasm-pack build crates/wasm --target web --out-dir ../../pkg --out-name rendero
```
Output: `pkg/rendero.js` + `pkg/rendero_bg.wasm`

### Native Shell (pure Rust app)
```bash
cargo build -p rendero-native-shell
# or release:
cargo build --release -p rendero-native-shell
```
Output: `target/debug/rendero` (or `target/release/rendero`)

### Native FFI (C-ABI dylib for Swift/C++)
```bash
cargo build --release -p rendero-native-ffi
```
Output: `target/release/librendero_native_ffi.dylib` + `target/release/librendero_native_ffi.a`

### JS Bundles (DOM shim + React/Vue apps)
```bash
cd docs/demos/dom-shim
node build.mjs
```
Output: `dist/react-web.js`, `dist/react-native.js`, `dist/vue-web.js`, `dist/vue-native.js`, `dist/macos-react-bundle.js`, `dist/macos-vue-bundle.js`, `dist/macos-bundle.js`

### Full Rebuild (everything)
```bash
# 1. WASM
wasm-pack build crates/wasm --target web --out-dir ../../pkg --out-name rendero

# 2. JS bundles
cd docs/demos/dom-shim && node build.mjs && cd ../../..

# 3. Native shell
cargo build -p rendero-native-shell
```

---

## Run Commands

### Native App
```bash
cargo run -p rendero-native-shell
```
Opens a macOS window with React rendering through the Rust engine. Close window to exit.

### Web Dev Server
```bash
# Kill old server first
lsof -ti:5555 | xargs kill 2>/dev/null

# Start a no-cache dev server from repo root
serve -l tcp://127.0.0.1:5555 docs -C --no-etag -c ./serve.rendero.json
```
Then open: `http://127.0.0.1:5555/demos/dom-shim/`

### Swift Shell (legacy, superseded by Rust shell)
```bash
swiftc native/macos-app.swift -L target/release -lrendero_native_ffi -framework AppKit -framework JavaScriptCore -o native/RenderoApp
DYLD_LIBRARY_PATH=target/release native/RenderoApp
```

---

## Test Commands

### Rust Unit Tests
```bash
# All core tests (includes Taffy layout tests)
cargo test -p rendero-core

# Specific test
cargo test -p rendero-core test_vertical_stack

# All tests across workspace
cargo test
```

### Test with Verbose Output
```bash
cargo test -p rendero-core -- --nocapture
```

### Accuracy Suite
```bash
scripts/run_accuracy_suite.sh
```
Builds the DOM-shim bundles and WASM output, refreshes `corpus/dashboard.json`,
starts a no-cache server, captures browser oracle + Rendero engine layout for the
Apple demo, runs the synthetic layout corpus benchmark, and writes the outputs
into `accuracy/`.

### Native App with Log Capture
```bash
cargo run -p rendero-native-shell > /tmp/rendero.log 2>&1 &
APP_PID=$!
sleep 5
kill $APP_PID
cat /tmp/rendero.log
```

---

## Screenshot / Debug Commands

### Peekaboo (native app screenshots)
```bash
# Full screen (Retina)
peekaboo image --mode screen --retina --path /tmp/screenshot.png

# Specific app window
peekaboo image --mode window --app "rendero" --retina --path /tmp/rendero.png

# Then view:
# Use Read tool on the image file, or open /tmp/rendero.png
```

### macOS screencapture
```bash
# Full screen, no sound
screencapture -x /tmp/screen.png

# As JPEG (smaller file)
screencapture -x -t jpg /tmp/screen.jpg
```

### Automated Test Cycle (launch, screenshot, kill)
```bash
cargo run -p rendero-native-shell > /tmp/rendero.log 2>&1 &
APP_PID=$!
sleep 6
screencapture -x -t jpg /tmp/rendero-test.jpg
kill $APP_PID
wait $APP_PID 2>/dev/null
cat /tmp/rendero.log | tail -20
```

---

## Development Workflow

### Swap Test Component
In `docs/demos/dom-shim/src/apple-react.jsx`, change the last line:
```jsx
// Simple colored rectangles (for testing layout/rendering):
export default function App() { return <TestApp />; }

// Full Apple website (for real content):
// export default function App() { return <AppleApp />; }
```

### Iterate on Shim (JS changes)
```bash
# 1. Edit shims/*.js or src/*.jsx
# 2. Rebuild bundles
cd docs/demos/dom-shim && node build.mjs

# 3. Test on native (no cache issues)
cargo run -p rendero-native-shell

# 4. Test on web (may need hard reload Cmd+Shift+R)
# Open http://localhost:5555/demos/dom-shim/
```

### Iterate on Engine (Rust changes)
```bash
# 1. Edit crates/core/ or crates/renderer/
# 2. Run tests
cargo test -p rendero-core

# 3. Rebuild native shell (picks up changes automatically)
cargo run -p rendero-native-shell

# 4. For web, also rebuild WASM
wasm-pack build crates/wasm --target web --out-dir ../../pkg --out-name rendero
```

### Cache Busting (web only)
The recommended dev server already serves without ETags and with explicit cache control.
The page also appends `?v=<timestamp>` to bundle imports on mode switches.
If you still need to force a refresh:
1. Hard reload: `Cmd+Shift+R` in browser
2. Bump `?v=N` on the page URL: `http://127.0.0.1:5555/demos/dom-shim/?v=42`

Native app has no cache issues.

---

## Key Directories

| Path | What |
|------|------|
| `crates/core/` | Scene graph, nodes, layout (Taffy), providers |
| `crates/renderer/` | Tile-based CPU rasterizer, text, SVG export |
| `crates/wasm/` | WASM bindings (wasm-bindgen) |
| `crates/native-shell/` | Pure Rust native app (winit + QuickJS) |
| `crates/native-ffi/` | C-ABI FFI for Swift/C++ |
| `crates/crdt/` | CRDT operations for collaboration |
| `crates/fig-import/` | Figma .fig file parser |
| `docs/demos/dom-shim/` | DOM shim + React/Vue demo apps |
| `docs/demos/dom-shim/shims/` | The DOM shim layer (10 JS files) |
| `docs/demos/dom-shim/src/` | Entry points + Apple page components |
| `docs/demos/dom-shim/dist/` | Built JS bundles (gitignored) |
| `pkg/` | Built WASM output |
| `native/` | Legacy Swift shell (superseded) |
| `docs/project/` | This documentation |

---

## Environment Requirements

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | stable (2021 edition) | Engine, native shell |
| wasm-pack | latest | WASM build |
| Node.js | 18+ | JS bundling (esbuild) |
| Python 3 | any | Dev server |
| peekaboo | 3.0+ | Native app screenshots |
| Xcode CLT | latest | macOS compilation (clang linker) |
