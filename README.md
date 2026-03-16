# Rendero

A Rust/WASM 2D rendering engine with a built-in scene graph, spatial indexing, and CRDT collaboration — designed for building infinite canvas applications in the browser.

**[Live Demo](https://rajatady.github.io/Rendero/)** · [Earthquake Explorer](https://rajatady.github.io/Rendero/demos/earthquake-explorer/) · [Design Tool](https://rajatady.github.io/Rendero/demos/design-tool/) · [Gaussian Splats](https://rajatady.github.io/Rendero/demos/splat-viewer/) · [Genome Browser](https://rajatady.github.io/Rendero/demos/genome-browser/) · [Neural Net](https://rajatady.github.io/Rendero/demos/neural-net/)

## Performance

| Scene | FPS | Render Path |
|-------|-----|-------------|
| 2M interactive nodes | 127 | Canvas2D scene graph |
| 134M points ([Neural Net demo](https://rajatady.github.io/Rendero/demos/neural-net/)) | 120 | WebGL2 GPU point cloud |

Every node in the scene graph is selectable, draggable, and undo-able. GPU point clouds are a separate view-only path for massive datasets.

Spatial hash grid for O(1) viewport culling. Hierarchical LOD. Sub-pixel auto-cull.

## Features

- **Dual rendering** — Canvas2D for crisp text/vectors, WebGL2 for millions of points. Both run simultaneously with synced cameras.
- **Scene graph** — Arena-based tree with hit testing, selection, grouping, undo/redo.
- **Batch APIs** — Bulk insertion that bypasses per-node CRDT/undo overhead. 500K nodes in under a second.
- **GPU point clouds** — Separate render path for view-only massive datasets. Spatial chunking with automatic LOD (full → 1:16 → 1:256).
- **CRDT** — Lamport timestamp-based conflict resolution for real-time collaboration.
- **Figma import** — Native `.fig` file parsing.
- **Zero JS dependencies** — One WASM binary, vanilla JS on top.

## Quick Start

```javascript
import init, { CanvasEngine } from 'rendero-wasm';

await init();

const engine = new CanvasEngine("MyApp", 1);
engine.set_viewport(window.innerWidth, window.innerHeight);

// Add shapes
engine.add_rectangle("rect1", 100, 100, 200, 150, 0.2, 0.5, 1.0, 1.0);
engine.add_ellipse("circle1", 400, 200, 80, 80, 1.0, 0.4, 0.3, 1.0);
engine.add_text("label", "Hello", 100, 50, 24, 1.0, 1.0, 1.0, 1.0);

// Render
const ctx = canvas.getContext('2d');
engine.render_canvas2d(ctx, canvas.width, canvas.height);

// Or use WebGL2 for large scenes
const gl = canvas.getContext('webgl2');
engine.render_webgl(gl, canvas.width, canvas.height);
```

### Bulk insertion (100x faster)

```javascript
// 8 floats per node: x, y, w, h, r, g, b, a
const data = new Float32Array(500_000 * 8);
// ... fill data ...
engine.add_ellipses_batch(data);
```

### GPU point clouds (millions of points)

```javascript
const points = new Float32Array(1_000_000 * 8);
// ... fill points ...
engine.add_point_cloud(gl, points);
```

## Camera

```javascript
engine.pan_start(x, y);
engine.pan_move(x, y);
engine.pan_end();
engine.zoom(+1, mouseX, mouseY);  // zoom in toward cursor
engine.zoom_to_fit();
engine.set_zoom_bounds(0.01, 100);

const [x, y, z] = engine.get_camera();
```

## Building from source

```bash
# Requires Rust and wasm-pack
wasm-pack build crates/wasm --target web --out-dir ../../pkg --out-name rendero
```

## Architecture

```
crates/
  core/       — Scene graph, spatial index, node types
  renderer/   — Canvas2D rendering backend
  crdt/       — Conflict-free replicated data types
  fig-import/ — .fig file parser
  wasm/       — WebGL2 backend, WASM bindings, batch APIs, point clouds
```

## License

MIT
