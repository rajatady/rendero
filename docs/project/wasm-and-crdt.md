# WASM Bindings, CRDT System, and Figma Import

Internal reference for the three supporting crates that sit around `rendero-core` and `rendero-renderer`.

---

## 1. WASM Bindings (`crates/wasm`)

**File:** `crates/wasm/src/lib.rs` (~6400 lines)

Thin `wasm_bindgen` wrapper over `rendero-core` (document model) and `rendero-renderer` (scene graph + drawing). All domain logic lives in those crates; the WASM crate only handles FFI marshalling, interaction state machines, undo/redo, and the scene cache.

### CanvasEngine struct

Exported to JavaScript via `#[wasm_bindgen]`. Holds:

- `document: Document` -- the full document model (pages, tree, nodes)
- `selected: Vec<NodeId>` -- current selection
- `mode: InteractionMode` -- state machine for drag, resize, rotate, marquee, vector edit, shape creation
- `cam_x, cam_y, cam_zoom` -- camera state
- `scene_cache` -- pre-built `Vec<RenderItem>` rebuilt only on structural changes
- `spatial_grid` -- cell-based spatial index for O(1) hit testing on large documents
- `pending_ops: Vec<Operation>` -- outbound CRDT operations for collaboration
- `undo_stack, redo_stack` -- local undo history
- `clipboard: Vec<Node>` -- internal copy/paste buffer
- `pen_anchors` -- pen tool state for vector drawing
- `webgl_state` -- cached GPU state (shaders, buffers, VAOs)
- `point_clouds` -- GPU-direct point cloud data that bypasses the document tree
- `comments`, `prototype_links`, `text_arc_params` -- annotation and prototyping features

Constructor: `CanvasEngine::new(name: &str, client_id: u32) -> Self`

### Method categories

#### Node Creation

| Method | Signature | Notes |
|--------|-----------|-------|
| `add_rectangle` | `(x, y, w, h, r, g, b, a) -> Vec<u32>` | Returns `[counter, client_id]` |
| `add_ellipse` | `(x, y, w, h, r, g, b, a) -> Vec<u32>` | |
| `add_text` | `(x, y, text, font_size, r, g, b, a) -> Vec<u32>` | |
| `add_frame` | `(x, y, w, h, r, g, b, a) -> Vec<u32>` | Container with clip_content |
| `add_line` | `(x1, y1, x2, y2, r, g, b, a, weight) -> Vec<u32>` | |
| `add_image` | `(x, y, w, h, image_key) -> Vec<u32>` | Image node |
| `add_image_fill` | `(x, y, w, h, image_key, ...) -> Vec<u32>` | Rectangle with image paint fill |
| `add_vector` | `(commands_json, ...) -> Vec<u32>` | Arbitrary vector path |
| `add_star` | `(x, y, outer_r, inner_r, points, r, g, b, a) -> Vec<u32>` | Star/polygon |
| `add_gradient_rectangle` | `(x, y, w, h, stops_json) -> Vec<u32>` | Rectangle with gradient fill |
| `add_rounded_rect` | `(x, y, w, h, r, g, b, a, tl, tr, br, bl) -> Vec<u32>` | Per-corner radii |

All `add_*` methods insert under `insert_parent` (or page root if unset), emit CRDT `InsertNode` ops, and push an undo entry.

#### Property Setters

| Method | Signature | Notes |
|--------|-----------|-------|
| `set_node_position` | `(counter, client_id, x, y) -> bool` | |
| `set_node_size` | `(counter, client_id, w, h) -> bool` | |
| `set_node_rotation` | `(counter, client_id, degrees) -> bool` | |
| `set_node_fill` | `(counter, client_id, r, g, b, a) -> bool` | Replaces all fills with one solid |
| `set_node_fills_json` | `(counter, client_id, fills_json) -> bool` | Full fills array from JSON |
| `set_node_linear_gradient` | `(counter, client_id, ..., stops_json) -> bool` | |
| `set_node_radial_gradient` | `(counter, client_id, ..., stops_json) -> bool` | |
| `set_node_angular_gradient` | `(counter, client_id, ..., stops_json) -> bool` | |
| `add_node_fill` | `(counter, client_id, r, g, b, a) -> bool` | Appends a solid fill |
| `add_node_linear_gradient` | `(counter, client_id, ...) -> bool` | Appends a gradient fill |
| `add_node_radial_gradient` | `(counter, client_id, ...) -> bool` | |
| `set_node_name` | `(counter, client_id, name) -> bool` | |
| `set_node_text` | `(counter, client_id, text) -> bool` | |
| `set_node_font_size` | `(counter, client_id, size) -> bool` | |
| `set_node_font_family` | `(counter, client_id, family) -> bool` | |
| `set_node_font_weight` | `(counter, client_id, weight) -> bool` | |
| `set_letter_spacing` | `(counter, client_id, spacing) -> bool` | |
| `set_line_height` | `(counter, client_id, height) -> bool` | |
| `set_text_decoration` | `(counter, client_id, decoration) -> bool` | "none", "underline", etc. |
| `set_text_vertical_align` | `(counter, client_id, align) -> bool` | |
| `set_text_align` | `(counter, client_id, align) -> bool` | |
| `set_text_gradient_fill` | `(counter, client_id, ...) -> bool` | Gradient on text runs |
| `set_text_arc` | `(counter, client_id, radius, start_angle, letter_spacing)` | Text along a circular arc |
| `set_node_stroke` | `(counter, client_id, r, g, b, a, weight) -> bool` | |
| `remove_node_stroke` | `(counter, client_id) -> bool` | |
| `set_stroke_align` | `(counter, client_id, align) -> bool` | "inside", "center", "outside" |
| `set_node_corner_radius` | `(counter, client_id, tl, tr, br, bl) -> bool` | |
| `set_node_blend_mode` | `(counter, client_id, mode) -> bool` | u32 enum index |
| `set_node_opacity` | `(counter, client_id, opacity) -> bool` | |
| `set_node_mask` | `(counter, client_id, is_mask) -> bool` | |
| `set_image_fill` | `(counter, client_id, image_key) -> bool` | |
| `add_drop_shadow` | `(counter, client_id, r, g, b, a, ox, oy, blur, spread) -> bool` | |
| `add_inner_shadow` | `(counter, client_id, r, g, b, a, ox, oy, blur, spread) -> bool` | |
| `add_blur` | `(counter, client_id, radius) -> bool` | Layer blur effect |
| `set_dash_pattern` | `(counter, client_id, dashes) -> bool` | Dashed strokes |
| `set_auto_layout` | `(counter, client_id, direction, gap, padding_h, padding_v, align, ...) -> bool` | Flexbox-style auto layout |
| `remove_auto_layout` | `(counter, client_id) -> bool` | |
| `set_node_constraints` | `(counter, client_id, h, v) -> bool` | Responsive constraints |

Property setters use incremental `patch_scene_*()` methods to avoid full scene cache rebuilds. The `mark_dirty()` method (full cache invalidation) is reserved for structural changes only -- on a 1.8M node document, a full rebuild takes 500ms-5s.

#### Rendering

| Method | Signature | Notes |
|--------|-----------|-------|
| `render_canvas2d` | `(ctx, width, height, dpr)` | Software render via Canvas2D context |
| `render_webgl` | `(gl, width, height, dpr)` | GPU render via WebGL2 |
| `render` | `(width, height) -> Vec<u8>` | CPU pixel export (RGBA) |
| `export_pixels` | `(width, height) -> Vec<u8>` | Export visible area as RGBA bytes |
| `export_svg` | `(width, height) -> String` | Export as SVG string |
| `needs_render` | `() -> bool` | Check if re-render needed |
| `drawn_count` | `() -> usize` | Items drawn in last frame |

#### Camera and Viewport

| Method | Signature | Notes |
|--------|-----------|-------|
| `set_viewport` | `(width, height)` | Set canvas size in pixels |
| `set_camera` | `(x, y, zoom)` | Set camera position and zoom |
| `get_camera` | `() -> Vec<f32>` | Returns `[cam_x, cam_y, cam_zoom]` |
| `zoom` | `(delta, screen_x, screen_y)` | Zoom toward/away from a screen point |
| `pan_start` | `(screen_x, screen_y)` | Begin panning |
| `pan_move` | `(screen_x, screen_y)` | Continue pan drag |
| `pan_end` | `()` | End panning |
| `zoom_to_fit` | `() -> bool` | Fit all content in viewport |
| `get_node_world_bounds` | `(counter, client_id) -> Vec<f32>` | Returns `[x, y, w, h]` in world coords |

#### Selection and Editing

| Method | Signature | Notes |
|--------|-----------|-------|
| `select_node` | `(counter, client_id)` | Replace selection |
| `toggle_select_node` | `(counter, client_id)` | Add/remove from selection |
| `select_all` | `()` | Select all nodes on current page |
| `get_selected` | `() -> Vec<u32>` | Packed `[counter, client_id, ...]` pairs |
| `delete_selected` | `() -> bool` | Delete selected nodes |
| `copy_selected` | `() -> u32` | Copy to internal clipboard |
| `paste` | `() -> u32` | Paste from clipboard with offset |
| `duplicate_selected` | `() -> u32` | Copy + paste in one step |
| `group_selected` | `() -> bool` | Wrap selected in a group frame |
| `ungroup_selected` | `() -> bool` | Dissolve group, reparent children |
| `boolean_op` | `(op: u32) -> bool` | Union, subtract, intersect, exclude |
| `flatten_selected` | `() -> bool` | Flatten to single vector path |
| `bring_to_front` | `() -> bool` | Z-order |
| `send_to_back` | `() -> bool` | |
| `bring_forward` | `() -> bool` | |
| `send_backward` | `() -> bool` | |
| `align_selected` | `(direction: u32) -> bool` | Align left/center/right/top/middle/bottom |
| `distribute_selected` | `(direction: u32) -> bool` | Evenly distribute horizontally/vertically |
| `undo` | `() -> bool` | |
| `redo` | `() -> bool` | |
| `exit_group` | `()` | Leave entered group context |
| `get_entered_group` | `() -> Vec<i64>` | Currently entered group ID or empty |

#### Input Handling

| Method | Signature | Notes |
|--------|-----------|-------|
| `mouse_down` | `(sx, sy, shift) -> bool` | Handles hit test, selection, drag/resize/rotate start |
| `mouse_move` | `(sx, sy)` | Handles drag, resize, rotate, marquee, hover |
| `mouse_up` | `()` | Commits interaction, pushes undo |
| `handle_double_click` | `(sx, sy) -> bool` | Enter group or start vector editing |
| `is_rotation_zone` | `(sx, sy) -> bool` | Check if cursor is near rotation handle |

#### Vector / Pen Tool

| Method | Signature | Notes |
|--------|-----------|-------|
| `pen_start` | `()` | Activate pen tool |
| `pen_is_active` | `() -> bool` | |
| `pen_cancel` | `()` | Discard pen path |
| `pen_mouse_down` | `(sx, sy)` | Add anchor point |
| `pen_mouse_drag` | `(sx, sy)` | Drag handle for curves |
| `pen_mouse_up` | `()` | Commit anchor |
| `pen_mouse_move` | `(sx, sy)` | Update preview line |
| `pen_finish_open` | `()` | Commit as open path |
| `pen_finish_closed` | `()` | Commit as closed path |
| `pen_get_state` | `() -> String` | JSON: anchors, handles, preview |
| `is_vector_editing` | `() -> bool` | |
| `vector_edit_get_state` | `() -> String` | JSON: editable anchor points |
| `vector_edit_exit` | `()` | Leave vector editing mode |
| `get_vector_network` | `(counter, client_id) -> String` | JSON vector network data |

#### Shape Creation (Click-drag)

| Method | Signature | Notes |
|--------|-----------|-------|
| `start_creating` | `(shape_type: &str)` | "rectangle", "ellipse", "frame", "star", "text" |
| `is_creating` | `() -> bool` | |
| `get_creation_preview` | `() -> Vec<f32>` | `[x, y, w, h]` of shape being drawn |
| `cancel_creating` | `()` | |

#### Batch APIs

| Method | Signature | Notes |
|--------|-----------|-------|
| `add_ellipses_batch` | `(data: &[f32]) -> u32` | Flat array: `[x, y, w, h, r, g, b, a, ...]`. Bypasses per-node CRDT/undo/spatial-grid overhead. 100x faster than individual calls. |
| `add_point_cloud` | `(gl, data: &[f32]) -> u32` | GPU-direct instanced rendering. Data goes straight to GPU buffers, bypasses document tree entirely. |
| `clear_point_clouds` | `(gl)` | Free GPU buffers |
| `point_cloud_count` | `() -> usize` | |

#### Figma Import / Export

| Method | Signature | Notes |
|--------|-----------|-------|
| `import_fig_json` | `(json_str, image_base) -> String` | Import from pre-parsed JSON |
| `import_fig_binary` | `(bytes: &[u8]) -> String` | Import from raw .fig binary |
| `import_fig_page_json` | `(page_json, image_base) -> String` | Import a single page |
| `get_imported_image` | `(path) -> Vec<u8>` | Retrieve extracted image bytes |
| `export_document_json` | `() -> String` | Serialize full document |
| `import_document_json` | `(json) -> String` | Deserialize full document |
| `get_visible_image_fills` | `(width, height) -> String` | JSON of image fills visible in viewport |
| `get_all_image_keys` | `() -> String` | All image references in document |
| `find_nodes_with_image` | `(image_key) -> String` | Nodes using a specific image |

#### Layer Panel / Inspection

| Method | Signature | Notes |
|--------|-----------|-------|
| `node_count` | `() -> usize` | Total nodes in current page |
| `layer_count` | `() -> u32` | Top-level layer count |
| `get_layers` | `() -> String` | JSON layer list |
| `get_layers_range` | `(start, count) -> String` | Paginated layer list |
| `get_tree_layers` | `(expanded_ids, start, count) -> Vec<u32>` | Virtualized tree view |
| `get_node_info` | `(counter, client_id) -> String` | Full JSON node properties |
| `get_node_name` | `(counter, client_id) -> String` | Node name |
| `find_nodes_by_name` | `(query) -> String` | Search nodes by name |

#### Pages

| Method | Signature | Notes |
|--------|-----------|-------|
| `page_count` | `() -> u32` | |
| `current_page_index` | `() -> u32` | |
| `get_pages` | `() -> String` | JSON array of `{name, index}` |
| `add_page` | `(name) -> u32` | |
| `switch_page` | `(index) -> bool` | |
| `rename_page` | `(index, name) -> bool` | |

#### Components and Instances

| Method | Signature | Notes |
|--------|-----------|-------|
| `create_component` | `() -> Vec<u32>` | Convert selected node to component |
| `create_instance` | `(comp_counter, comp_client_id) -> Vec<u32>` | Instantiate a component |
| `detach_instance` | `() -> bool` | Detach selected instance from its component |

#### Comments and Prototyping

| Method | Signature | Notes |
|--------|-----------|-------|
| `add_comment` | `(x, y, text, author) -> u32` | Pin a comment on canvas |
| `get_comments` | `() -> String` | JSON array of comments |
| `resolve_comment` | `(comment_id, resolved) -> bool` | |
| `delete_comment` | `(comment_id) -> bool` | |
| `add_prototype_link` | `(source_counter, source_client, target_counter, target_client, trigger, animation)` | |
| `get_prototype_links` | `() -> String` | |
| `remove_prototype_links` | `(counter, client_id) -> bool` | |

#### CRDT Collaboration

| Method | Signature | Notes |
|--------|-----------|-------|
| `get_pending_ops` | `() -> String` | Drain outbound CRDT ops as JSON |
| `apply_remote_ops` | `(json) -> u32` | Apply inbound CRDT ops from peers |

#### Utility

| Method | Signature | Notes |
|--------|-----------|-------|
| `set_snap_grid` | `(size: f32)` | 0 = disabled; typical values: 1, 4, 8, 16 |
| `get_snap_grid` | `() -> f32` | |
| `set_insert_parent` | `(counter, client_id)` | Route add_* calls into a specific parent |
| `clear_insert_parent` | `()` | Reset to page root |
| `get_text_arc` | `(counter, client_id) -> Vec<f32>` | `[radius, start_angle, letter_spacing]` |
| `get_marquee_rect` | `() -> Vec<f32>` | Current marquee selection rectangle |

---

## 2. CRDT System (`crates/crdt`)

**Purpose:** Conflict-free replicated data types for real-time collaborative editing. Every document mutation is expressed as an `Operation` that is commutative, idempotent, and serializable.

### Design Principles

- **Commutative:** `apply(a, apply(b, state)) == apply(b, apply(a, state))` -- order does not matter.
- **Idempotent:** `apply(a, apply(a, state)) == apply(a, state)` -- safe to replay.
- **Self-contained:** Each operation carries all data needed to apply it. No external state references. Operations are serializable (sent over the wire) and replayable (applied to any document state).
- **Type-level enforcement:** `OpKind` is an exhaustive enum -- the compiler forces every consumer to handle all variants.

### Files

| File | Purpose |
|------|---------|
| `operation.rs` | `Operation`, `OpKind` enum, `FractionalIndex`, `PropertyUpdate` |
| `apply.rs` | `apply()` function -- single entry point for all document mutations |
| `history.rs` | `History` struct -- undo/redo via operation inversion |

### Operation struct

```
Operation {
    id: OpId,       // Unique ID (client_id + counter). Provides total ordering.
    kind: OpKind,   // The actual mutation.
}
```

### OpKind variants

| Variant | Fields | Description |
|---------|--------|-------------|
| `InsertNode` | `node, parent_id, position: FractionalIndex` | Insert a complete node into the tree |
| `DeleteNode` | `node_id` | Delete a node and all its descendants |
| `MoveNode` | `node_id, new_parent_id, position: FractionalIndex` | Reparent a node |
| `SetProperty` | `node_id, property: PropertyUpdate` | Update a single property on a node |
| `Reorder` | `node_id, position: FractionalIndex` | Reorder a node among its siblings |

### FractionalIndex

Solves the concurrent insertion problem. Instead of integer indices (which conflict when two clients insert at the same position), uses string-based keys that can always be bisected:

- `FractionalIndex::start()` -- returns `"A"`
- `FractionalIndex::end()` -- returns `"z"`
- `FractionalIndex::between(left, right)` -- generates a key between two existing keys (string space is dense, so this always succeeds)

Two inserts at different fractional positions always produce a deterministic order regardless of which is applied first.

### PropertyUpdate enum

Exhaustive enum of all mutable node properties. Each variant carries the new value:

| Category | Variants |
|----------|----------|
| Geometry | `Transform`, `Width`, `Height` |
| Visibility | `Opacity`, `BlendMode`, `Visible`, `Locked`, `Name` |
| Style arrays | `Fills(Vec<Paint>)`, `Strokes(Vec<Paint>)`, `Effects(Vec<Effect>)` |
| Stroke | `StrokeWeight`, `StrokeAlign`, `StrokeCap`, `StrokeJoin` |
| Kind-specific | `CornerRadii`, `ClipContent`, `AutoLayout`, `TextRuns`, `TextAlign` |

Style arrays use replace-entire-array semantics for simpler CRDT behavior (no per-element conflict resolution needed).

### apply() function

```rust
pub fn apply(tree: &mut DocumentTree, op: &Operation) -> ApplyResult
```

Single entry point for ALL document mutations. Returns:

| Result | Meaning |
|--------|---------|
| `Applied` | Operation applied successfully |
| `NoOp` | Operation was redundant (e.g., deleting already-deleted node) |
| `Deferred` | Dependencies not yet present; queue and retry later |

Conflict resolution rules:
- **FractionalIndex** for sibling ordering (no index conflicts)
- **OpId** for tie-breaking (deterministic winner via total ordering)
- **Last-writer-wins** for property updates (OpId determines "last")
- **Delete wins over concurrent move** (prevents orphan nodes)

### History (undo/redo)

```rust
pub struct History {
    entries: Vec<HistoryEntry>,
    undo_stack: Vec<usize>,
    redo_stack: Vec<HistoryEntry>,
}
```

Undo/redo works by storing operations paired with their computed inverse. Undo = apply the inverse. Redo = re-apply the original. This avoids snapshotting full document state.

| Method | Purpose |
|--------|---------|
| `push(op, inverse, is_local)` | Record an operation. Only local ops go on undo stack. |
| `pop_undo() -> Option<Operation>` | Get the inverse of the last local op |
| `pop_redo() -> Option<Operation>` | Re-apply the last undone op |
| `ops_after(after: Option<OpId>)` | Get all operations since a given point (for sync) |

The `compute_inverse()` function calculates an operation's inverse by reading the current tree state BEFORE the operation is applied. For example, the inverse of `InsertNode` is `DeleteNode`; the inverse of `SetProperty` captures the old property value.

---

## 3. Figma Import (`crates/fig-import`)

**Purpose:** Parse `.fig` binary files (Figma's native format) and convert them into a JSON document tree compatible with `rendero-core`.

### Pipeline

```
.fig bytes
  |
  v
1. ZIP extraction (if .fig is a ZIP container)
   -> canvas.fig bytes + extracted images
  |
  v
2. Header validation (magic: "fig-kiwi" or "fig-jam.")
   + chunk extraction + Deflate decompression
  |
  v
3. Kiwi schema decode -> serde_json::Value
   (binary schema in chunk 0, data in chunk 1)
  |
  v
4. Flat nodeChanges array -> hierarchical tree
  |
  v
5. Blob substitution
   (commandsBlob -> commands, vectorNetworkBlob -> vectorNetwork)
  |
  v
6. Essential transforms
   (color -> hex, matrix -> CSS, image hash -> filename)
  |
  v
FigImportResult { document: JSON, images: Vec<(path, bytes)>, version: u32 }
```

### Files

| File | Purpose |
|------|---------|
| `lib.rs` | Public API: `convert_fig(bytes) -> Result<FigImportResult>` |
| `container.rs` | ZIP extraction, header validation (`fig-kiwi`/`fig-jam.` magic), chunk parsing |
| `decode.rs` | Kiwi binary schema decoding to JSON via `kiwi_schema` crate |
| `blobs.rs` | Base64 encoding of binary blobs, blob reference substitution in tree |
| `tree.rs` | Flat `nodeChanges` array to hierarchical parent-child tree |
| `transform.rs` | Color/matrix/image-hash transforms to rendero-compatible formats |
| `error.rs` | `FigError` enum and `Result` type |

### Public API

```rust
pub fn convert_fig(bytes: &[u8]) -> Result<FigImportResult>
```

```rust
pub struct FigImportResult {
    pub document: serde_json::Value,  // JSON tree ready for WASM fig_import.rs conversion
    pub images: Vec<(String, Vec<u8>)>,  // (path, bytes) for extracted images
    pub version: u32,  // .fig format version
}
```

The WASM crate calls this from two entry points:
- `import_fig_binary(bytes)` -- calls `convert_fig()` directly
- `import_fig_json(json_str, image_base)` -- for pre-parsed JSON (skips binary decoding)

### Container format

`.fig` files can be either:
- **Raw fig-kiwi binary** -- starts with `fig-kiwi` magic (8 bytes)
- **ZIP archive** -- starts with `PK` magic, contains `canvas.fig` + `images/` directory

The container module detects the format automatically and extracts accordingly.

### Kiwi decoding

The binary format uses [Kiwi](https://github.com/nickhash/kiwi-schema), a schema-based binary encoding. The `.fig` file contains two chunks:
- **Chunk 0:** Kiwi schema definition
- **Chunk 1:** Encoded data (root type is `Message` with `nodeChanges` and `blobs` fields)

Decoding produces a `serde_json::Value` tree that matches Figma's internal representation.

---

## 4. How WASM Connects to the DOM Shim

The DOM shim (`docs/demos/dom-shim/`) lets React, Vue, and React Native apps render through the Rendero engine instead of a real browser DOM.

### Architecture

```
React / Vue / React Native
        |
        v
  DOM Shim (document.js, element.js, style.js, ...)
        |
        v
  Engine Bridge (engine.js or engine-native.js)
        |
        v
  WASM CanvasEngine  (web)   OR   NativeEngine (iOS/macOS via C FFI)
        |                              |
        v                              v
  rendero-core + rendero-renderer  (same Rust code)
```

**Web path (`shims/engine.js`):**
- Imports `CanvasEngine` from `pkg/rendero.js` (wasm-pack output)
- Holds a single shared engine instance
- Maps shim element IDs to engine `(counter, client_id)` pairs via `_nodeRegistry`
- Queues all engine mutations in `_ops[]` and flushes them serially before each render frame (WASM uses `&mut self` borrows, so concurrent calls are forbidden)

**Native path (`shims/engine-native.js`):**
- Drop-in replacement for `engine.js` with the same exports
- Calls global C functions injected by Swift via `JSContext`: `__rendero_create()`, `__rendero_add_frame()`, `__rendero_set_node_fill()`, `__rendero_render_pixels()`, etc.
- The Swift host holds the engine pointer and wraps each Rust FFI call

The key insight: **CanvasEngine (WASM) and NativeEngine (C FFI) wrap the same `rendero-core` and `rendero-renderer` crates.** The DOM shim's engine bridge is the only file that differs between platforms. All other shim files (element.js, style.js, document.js, events.js) are platform-agnostic.

---

## 5. Build

### Prerequisites

- Rust toolchain (stable)
- `wasm-pack` (install via `cargo install wasm-pack` or `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`)

### WASM build command

```bash
wasm-pack build crates/wasm --target web --out-dir ../../pkg --out-name rendero
```

This produces:
- `pkg/rendero.js` -- JS glue (ES module)
- `pkg/rendero_bg.wasm` -- compiled WebAssembly binary (~600KB gzipped, ~1.6MB raw)
- `pkg/rendero.d.ts` -- TypeScript type declarations

The `--target web` flag generates ES module output suitable for direct `<script type="module">` import or bundler consumption. The `--out-dir ../../pkg` places output at the repo root's `pkg/` directory, where demos reference it.

### Native build (iOS/macOS)

The native build uses `cargo build --release --target aarch64-apple-darwin` (or the iOS target triple) via the `crates/native-ffi/` crate, producing a static library that Swift links against. See `native/` for the macOS app wrapper.
