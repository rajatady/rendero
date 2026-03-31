# rendero-renderer -- Tile-Based CPU Rasterizer

## 1. Overview

`rendero-renderer` is a tile-based software renderer that converts a `DocumentTree` (the editing model) into raw RGBA pixel buffers. It is designed as a typed pipeline where each stage has well-defined input and output types -- if it compiles, the pipeline is structurally correct.

**Key design properties:**

- **Tile-based**: Only re-renders tiles that changed (dirty tracking). Each 64x64 tile fits in L1 cache (16 KB).
- **Cache-friendly**: Contiguous pixel memory within tiles for SIMD-ready operations.
- **Parallelizable**: Tiles are independent -- zero shared mutable state between tiles.
- **Separation of concerns**: The document tree (editing) and scene graph (rendering) are fully decoupled.

**Crate location:** `crates/renderer/`

---

## 2. Pipeline

The renderer uses a strict, type-enforced pipeline. Each stage's output type is the next stage's input type.

```
DocumentTree
    |
    v
build_scene(tree, root, viewport)          [scene.rs]
    |
    v
Vec<RenderItem>                            Flat, z-sorted list
    |
    v
render_items(items, viewport)              [pipeline.rs]
    |  - Creates TileGrid from viewport
    |  - For each item, finds affected tiles
    |  - Rasterizes item into each affected tile
    |  - Manages clip stack
    v
RenderOutput { tiles, grid, item_count }
    |
    v
to_pixels(width, height)                  [pipeline.rs]
    |  - Assembles HashMap<TileCoord, TileBuffer>
    |     into a single contiguous RGBA buffer
    v
Vec<u8>                                    Final RGBA pixels
```

### Entry Points

| Function | Signature | Purpose |
|---|---|---|
| `pipeline::render` | `(tree, root, viewport) -> RenderOutput` | Full pipeline: tree to pixels |
| `pipeline::render_items` | `(items, viewport) -> RenderOutput` | From pre-built scene items (camera transforms already applied) |
| `RenderOutput::to_pixels` | `(width, height) -> Vec<u8>` | Assemble tiles into a flat RGBA buffer |

### RenderOutput

| Field | Type | Description |
|---|---|---|
| `tiles` | `HashMap<TileCoord, TileBuffer>` | Sparse map of rendered tiles (only allocated tiles with content) |
| `grid` | `TileGrid` | Grid metadata (cols, rows, viewport) |
| `item_count` | `usize` | Number of render items processed |

---

## 3. Scene Graph

The scene graph (`scene.rs`) is a **flat, sorted list** of render-ready items derived from the hierarchical document tree. This separation means:

- Editing does not pay rendering costs.
- Rendering does not need to understand tree traversal.
- Each can be optimized independently.

### build_scene

```
build_scene(tree: &DocumentTree, root: &NodeId, viewport: &AABB) -> Vec<RenderItem>
```

Traverses the document tree depth-first, producing a flat `Vec<RenderItem>` sorted by z-order (paint order). For each node it:

1. Skips invisible nodes (`node.visible == false`).
2. Computes the world transform by composing with the parent transform (fast path for translation-only nodes avoids matrix multiply).
3. Computes the world-space AABB by transforming all four corners and taking the axis-aligned enclosure.
4. Culls nodes entirely outside the viewport (AABB intersection test).
5. Converts `NodeKind` to `RenderShape` (resolves editing abstractions like Components and Instances into concrete draw commands).
6. Records `descendant_count` (items.len delta after recursing children) for clip region management.

### RenderItem

| Field | Type | Description |
|---|---|---|
| `node_id` | `NodeId` | Source node identity |
| `world_transform` | `Transform` | Fully composed world-space affine transform |
| `world_bounds` | `AABB` | Axis-aligned bounding box in world space |
| `style` | `Style` | Visual style (fills, strokes, effects, opacity) |
| `shape` | `RenderShape` | What to draw (rect, ellipse, path, text, image) |
| `z_index` | `u32` | Paint order (lower = behind) |
| `clips` | `bool` | Whether this item clips its children |
| `descendant_count` | `usize` | Number of descendant items following this one in the list |
| `is_mask` | `bool` | Whether this item acts as a mask for subsequent siblings |

### RenderShape

| Variant | Fields | Source NodeKind |
|---|---|---|
| `Rect` | `width, height, corner_radii` | Frame, Rectangle, Component, Instance, Polygon (fallback) |
| `Ellipse` | `width, height, arc_start, arc_end, inner_radius_ratio` | Ellipse |
| `Line` | `length` | Line |
| `Path` | `commands: Vec<PathCommand>, fill_rule` | Vector, BooleanOp (computed result) |
| `Text` | `runs, width, height, align, vertical_align` | Text |
| `Image` | `width, height, data: Vec<u8>, image_width, image_height` | Image |

### AABB

Axis-aligned bounding box with `min: Vec2` and `max: Vec2`.

| Method | Description |
|---|---|
| `from_size(x, y, w, h)` | Construct from position and size |
| `intersects(other)` | True if two AABBs overlap |
| `contains_point(p)` | Point-in-box test |
| `intersect(other)` | Intersection (overlap region) |
| `union(other)` | Union (enclosing region) |
| `width() / height()` | Dimension accessors |

### NodeKind to RenderShape Mapping

```
Frame { clip_content, corner_radii }  -->  Rect { corner_radii }, clips=clip_content
Rectangle { corner_radii }            -->  Rect { corner_radii }
Ellipse { arc_start, arc_end, ... }   -->  Ellipse { ... }
Line                                  -->  Line { length=node.width }
Vector { paths }                      -->  Path { commands (flattened), fill_rule }
BooleanOp { operation }               -->  Path { commands } via compute_boolean()
Component                             -->  Rect, clips=true
Instance                              -->  Rect, clips=true
Text { runs, align, ... }             -->  Text { runs, align, ... }
Image { data, ... }                   -->  Image { data, ... }
Polygon                               -->  Rect (TODO: convert to path)
```

---

## 4. Tile System

All rendering is tile-based (`tile.rs`). The viewport is divided into a grid of 64x64 pixel tiles. Only tiles that intersect with render items are allocated.

### Constants

| Name | Value | Notes |
|---|---|---|
| `TILE_SIZE` | 64 | Pixels per tile edge. 64x64 x 4 bytes = 16 KB, fits L1 cache |

### TileBuffer

A single tile's pixel storage. Format: RGBA, premultiplied alpha, `u8` per channel.

| Field | Type | Description |
|---|---|---|
| `pixels` | `Vec<u8>` | RGBA pixel data, length = width * height * 4 |
| `width` | `u32` | Tile width in pixels |
| `height` | `u32` | Tile height in pixels |

| Method | Description |
|---|---|
| `new(w, h)` | Allocate zeroed (transparent) tile |
| `clear()` | Zero all pixels |
| `set_pixel(x, y, r, g, b, a)` | Direct write with bounds check |
| `get_pixel(x, y) -> (u8,u8,u8,u8)` | Read pixel, returns (0,0,0,0) for out-of-bounds |
| `blend_pixel(x, y, sr, sg, sb, sa)` | Source-over composite onto existing pixel |

**blend_pixel formula (source-over):**
```
out = src + dst * (1 - src_alpha)
```

### TileCoord

| Field | Type | Description |
|---|---|---|
| `col` | `u32` | Column index in the tile grid |
| `row` | `u32` | Row index in the tile grid |

`bounds()` returns the world-space AABB for the tile: `[col*64, row*64] to [(col+1)*64, (row+1)*64]`.

### TileGrid

| Field | Type | Description |
|---|---|---|
| `cols` | `u32` | Number of tile columns covering the viewport |
| `rows` | `u32` | Number of tile rows covering the viewport |
| `viewport` | `AABB` | The viewport this grid covers |

| Method | Description |
|---|---|
| `new(viewport)` | Create grid sized to cover the viewport |
| `tiles_for_item(item)` | Return all TileCoords whose tiles intersect the item's world bounds |
| `total_tiles()` | cols * rows |
| `all_tiles()` | Iterator over every TileCoord |

**Tile allocation is sparse**: tiles are stored in a `HashMap<TileCoord, TileBuffer>`. A tile is only allocated when a render item first touches it. The `to_pixels()` method assembles the sparse tile map into a contiguous pixel buffer, copying each tile's rows into the correct position.

---

## 5. Rasterization

All rasterization happens in `rasterize.rs`. Functions are pure: tile buffer in, tile buffer out. No global state. Uses inverse affine transforms so all shapes support rotation, scale, and skew.

### Core Flow

```
rasterize_item(tile, tile_coord, shape, fills, opacity, world_transform)
    |
    |-- Text?  -->  text::rasterize_text() [separate module]
    |-- Image? -->  rasterize_image()
    |
    |-- Extract first fill as Paint
    |-- Build ColorSampler from Paint
    |-- Compute inverse transform (world -> local)
    |
    |-- Rect?    -->  rasterize_rect()
    |-- Ellipse? -->  rasterize_ellipse()
    |-- Line?    -->  rasterize_line()
    |-- Path?    -->  rasterize_path()
```

Every shape rasterizer follows the same pattern for each pixel in the tile:

1. Convert world pixel coordinate to local space via inverse transform.
2. Test if the local-space point is inside the shape.
3. Sample the color from the `ColorSampler` at the local-space point.
4. Apply coverage (anti-aliasing) if near an edge.
5. Blend the result onto the tile using `blend_pixel` (source-over).

### ColorSampler

Resolves a `Paint` to a premultiplied color at a local-space coordinate. Gradient coordinates are normalized (0-1) and scaled to the node's pixel dimensions.

| Variant | Behavior |
|---|---|
| `Solid(PremultColor)` | Constant color, ignores position |
| `Linear { stops, start, dir_norm }` | Projects point onto gradient axis, samples at `t` |
| `Radial { stops, center, inv_radius }` | Distance from center normalized by radius, samples at `t` |

Gradient sampling (`sample_gradient`) does linear interpolation between stops in premultiplied color space.

Unsupported paint types (AngularGradient, DiamondGradient, Image fills) return `None` -- these are handled by the Canvas 2D renderer path, not the CPU rasterizer.

### Shape Rasterizers

**Rect** (`rasterize_rect`):
- Local-space bounds test: `0 <= x <= width, 0 <= y <= height`.
- Corner radii: computes signed distance from the circular arc at each corner. Anti-aliases with sub-pixel coverage.
- Supports per-corner radii (top-left, top-right, bottom-right, bottom-left).

**Ellipse** (`rasterize_ellipse`):
- Normalized distance test: `((x-cx)/rx)^2 + ((y-cy)/ry)^2 <= 1`.
- Anti-aliasing in the outer 5% band (`dist > 0.95`), with linear coverage falloff.

**Line** (`rasterize_line`):
- Hit test: `0 <= x <= length, |y| < 1.0`.
- Coverage falls off linearly with distance from y=0.

**Path** (`rasterize_path`):
- Flattens bezier curves to line segments (de Casteljau subdivision, tolerance 0.25 px, max depth 8).
- Winding number computed via horizontal ray casting.
- Fill rule: NonZero (winding != 0) or EvenOdd (winding & 1).
- Anti-aliasing: signed distance to nearest edge, coverage clamped to [0, 1].

**Image** (`rasterize_image`):
- Inverse-transforms pixel coordinates to local space.
- Bounds check against `[0, width) x [0, height)`.
- Nearest-neighbor sampling from source RGBA data.
- Opacity applied to alpha channel.

### Path Flattening

Bezier curves (cubic and quadratic) are recursively subdivided using de Casteljau's algorithm:

| Parameter | Value |
|---|---|
| `MAX_DEPTH` | 8 |
| `TOLERANCE` | 0.25 pixels |
| Flatness test | Sum of control point distances to chord < tolerance |

Quadratic beziers are first elevated to cubics before flattening.

### Winding Number

Computed by casting a horizontal ray from the test point toward +x and counting signed crossings:
- Upward crossing (y0 <= point.y < y1): winding += 1
- Downward crossing (y1 <= point.y < y0): winding -= 1

---

## 6. Stroke Rendering

Strokes are rendered by expanding the path into a filled outline polygon, then rasterizing via the normal path rasterizer (`stroke.rs`).

### expand_stroke

```
expand_stroke(commands, weight, align, cap, join) -> Vec<PathCommand>
```

1. Flattens the input path to points.
2. Computes per-vertex normals (average of adjacent edge directions, perpendicular).
3. Builds outer and inner offset curves by displacing points along normals.
4. Constructs a closed polygon: outer edge forward, end cap, inner edge backward, start cap.

### Stroke Alignment

| StrokeAlign | Outer Offset | Inner Offset |
|---|---|---|
| `Center` | weight / 2 | weight / 2 |
| `Inside` | 0 | weight |
| `Outside` | weight | 0 |

### Stroke Caps

| StrokeCap | End Treatment |
|---|---|
| `None` | Flat perpendicular cut |
| `Round` | Semicircle (8 segments) rotated around the endpoint |
| `Square` | Extends by `outer_w` beyond the endpoint |

### Shape to Path Conversion

Any `RenderShape` is converted to `PathCommand`s before stroke expansion:

- **Rect**: 4-point closed polygon.
- **Ellipse**: Approximated with 4 cubic bezier arcs (magic constant `k = 0.5522847498`).
- **Line**: Two-point open path.
- **Path**: Used directly.
- **Text / Image**: Not strokeable (returns empty).

---

## 7. Text Rendering

Text layout and rasterization is handled in `text.rs` using the `fontdue` crate for glyph rasterization.

### Embedded Font

- **Font**: Roboto Mono (`assets/RobotoMono.ttf`), compiled into the binary via `include_bytes!`.
- **Parsed once**: Stored in a `OnceLock<Font>` static for zero-cost access after initialization.

### rasterize_text

```
rasterize_text(tile, tile_coord, runs, width, height, align, vertical_align, world_transform, opacity)
```

Three-phase process:

**Phase 1 -- Glyph Layout:**
- Iterates over `TextRun`s, rasterizing each character via `fontdue::Font::rasterize(char, size)`.
- Positions glyphs using advance width + letter spacing.
- Handles newlines and word wrapping (wraps when `cursor_x + advance > text_width`).
- Tracks per-line width and height for alignment.

**Phase 2 -- Alignment:**
- Computes vertical offset based on `TextVerticalAlign` (Top / Center / Bottom).
- Computes horizontal offset per line based on `TextAlign` (Left / Center / Right / Justified).
- Applies offsets to all glyph positions.

**Phase 3 -- Rasterization:**
- For each pixel in the tile, inverse-transforms to local space.
- Tests against each placed glyph's bounding box.
- Samples the glyph bitmap (coverage mask from fontdue).
- Blends premultiplied color onto the tile, applying opacity.

### PlacedGlyph

| Field | Type | Description |
|---|---|---|
| `bitmap` | `Vec<u8>` | Alpha coverage mask from fontdue |
| `x, y` | `f32` | Position in local text space |
| `width, height` | `usize` | Glyph bitmap dimensions |
| `color` | `PremultColor` | Per-run text color (premultiplied) |

### Text Features

| Feature | Status |
|---|---|
| Multi-run styled text | Supported (per-run font size, color, letter spacing) |
| Word wrapping | Supported (wraps at text_width boundary) |
| Explicit newlines | Supported (`\n` character) |
| Horizontal alignment | Left, Center, Right, Justified (Left) |
| Vertical alignment | Top, Center, Bottom |
| Per-run line height | Supported (falls back to fontdue metrics) |
| Multiple fonts | Not supported (Roboto Mono only) |
| Font weight/style | Not supported |

---

## 8. Effects

### Drop Shadow

Shadows are rendered BEFORE the item itself so they appear behind it (`rasterize_drop_shadows` in `rasterize.rs`).

**Process:**

1. The pipeline expands the item's bounding box to include shadow extent: `max(|offset.x|, |offset.y|) + blur_radius + spread`.
2. A shadow transform is created by adding the offset to the item's world transform.
3. The shape dimensions are expanded by `spread * 2` on each axis.
4. For each pixel, a signed distance from the shape edge is computed.
5. Gaussian falloff is applied: `alpha = base_alpha * exp(-dist^2 / (2 * sigma^2))` where `sigma = blur_radius / 2`.
6. The shadow color is blended onto the tile.

Currently supported for Rect and Ellipse shapes only.

### Clipping

Frame nodes with `clip_content = true` clip their children. The pipeline manages this via a **clip stack** (`pipeline.rs`):

1. When a clipping item is encountered, its world bounds (intersected with any parent clip) are pushed onto the stack along with the end index (`i + 1 + descendant_count`).
2. For each subsequent item, the clip bounds are applied: the item's world bounds are intersected with the clip region.
3. After rasterization, pixels outside the clip bounds are restored from a saved copy of the tile.
4. When the item index exceeds the clip end index, the clip is popped.

### Opacity

- Per-item opacity is applied during rasterization by multiplying the paint color's alpha channel.
- For solid fills: `PremultColor { r*opacity, g*opacity, b*opacity, a*opacity }`.
- For images: `alpha = data_alpha * opacity`.
- For text: `alpha = glyph_coverage * opacity`.

### Compositing / Blend Modes

Blend mode compositing is in `composite.rs`. The `composite(dst, src, blend_mode)` function combines a source tile onto a destination tile.

**Fast path**: `BlendMode::Normal` uses source-over compositing inline (no per-channel blend function call).

| Blend Mode | Formula (per channel) |
|---|---|
| Normal | `src` (source-over) |
| Multiply | `src * dst` |
| Screen | `1 - (1-src)(1-dst)` |
| Overlay | `base<0.5 ? 2*base*blend : 1-2*(1-base)*(1-blend)` |
| Darken | `min(src, dst)` |
| Lighten | `max(src, dst)` |
| ColorDodge | `base / (1 - blend)` clamped |
| ColorBurn | `1 - (1-base)/blend` clamped |
| HardLight | Overlay with src/dst swapped |
| SoftLight | W3C formula with `d` factor |
| Difference | `|src - dst|` |
| Exclusion | `src + dst - 2*src*dst` |
| Hue, Saturation, ColorMode, Luminosity | TODO (currently fall through to Normal) |

---

## 9. SVG Export

`svg.rs` provides `export_svg(tree, root, viewport) -> String`, which traverses the document tree and emits SVG markup.

### Supported Node Types

| NodeKind | SVG Element |
|---|---|
| Rectangle | `<rect>` |
| Ellipse (equal axes) | `<circle>` |
| Ellipse (unequal axes) | `<ellipse>` |
| Text | `<text>` (joins all runs) |
| Frame | `<rect>` (background) + recurse children |
| Vector | `<path d="...">` |

### Limitations

- Only the first solid fill is used (gradients not exported).
- Opacity is emitted as an attribute when < 1.0.
- Strokes, effects (shadows), and blend modes are not exported.
- Boolean operations, Images, Components, Instances, Lines, and Polygons are skipped.
- Transforms are simplified to translation only (`world.tx`, `world.ty`).
- XML special characters are escaped (`&`, `<`, `>`, `"`).

---

## 10. Verification and Testing

`verify.rs` provides deterministic render testing without visual inspection.

### Approach

1. Render a known scene to pixels.
2. Hash the output buffer (`DefaultHasher`).
3. Compare to expected hash.
4. Optionally assert specific pixel values at known coordinates.
5. Optionally assert that pixels have alpha > 0 (opaque checks for font-dependent content).

### RenderTest

| Field | Type | Description |
|---|---|---|
| `name` | `&'static str` | Test identifier |
| `width, height` | `u32` | Viewport dimensions |
| `setup` | `fn(&mut DocumentTree, &mut ClockGen)` | Scene construction function |
| `expected_hash` | `Option<u64>` | Expected pixel hash (None = first run) |
| `pixel_checks` | `Vec<(u32,u32, u8,u8,u8,u8)>` | Exact pixel assertions (x, y, r, g, b, a) with +/-1 tolerance |
| `opaque_checks` | `Vec<(u32,u32)>` | Coordinates that must have alpha > 0 |

### Built-in Test Scenes

| Test Name | What It Validates |
|---|---|
| `empty_canvas` | Transparent output for empty tree |
| `single_red_rect` | Basic rectangle fill and positioning |
| `overlapping_rects_alpha` | Source-over alpha compositing |
| `ellipse_center` | Ellipse rasterization |
| `triangle_path` | Path fill with winding rule |
| `rotated_rect` | Affine transform (rotation) |
| `linear_gradient_rect` | Linear gradient sampling |
| `radial_gradient_rect` | Radial gradient sampling |
| `boolean_subtract` | Boolean path subtraction |
| `boolean_union` | Boolean path union |
| `text_renders_pixels` | Text rasterization (opaque checks) |
| `image_node_pixel_sampling` | Image node with nearest-neighbor sampling |
| `rounded_rect_corners` | Corner radius anti-aliasing |
| `frame_clips_children` | Clip content on frames |

### Binaries

| Binary | Command | Purpose |
|---|---|---|
| `render_verify` | `cargo run --bin render_verify` | Run all tests, exit 1 on failure |
| `bench` | `cargo run --bin bench` | Benchmark N-object rendering (100 to 80,000) |
| `snapshot_export` | `cargo run --bin snapshot_export [--dump]` | Export test pixel data and hashes; `--dump` writes raw RGBA to `.snapshots/` |

---

## 11. Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `rendero-core` | workspace (path dep) | Node types, DocumentTree, Transform, Style, Paint, Color, properties, boolean ops |
| `glam` | workspace | Vec2 math (positions, normals, distances) |
| `bytemuck` | workspace | Safe transmutation for pixel buffers |
| `fontdue` | workspace | TTF font parsing and glyph rasterization |
| `fontdb` | workspace | Font database (declared dependency, not directly used in renderer source) |

### Relationship to rendero-core

The renderer depends on these core types:

- `NodeId`, `ClockGen` -- node identity
- `Node`, `NodeKind`, `CornerRadii`, `PathCommand`, `TextRun`, `TextAlign`, `TextVerticalAlign`, `VectorPath`, `BooleanOperation` -- node model
- `DocumentTree` -- tree structure with parent/child traversal
- `Transform` -- 2D affine transform with `apply()`, `inverse()`, `then()`, `rotate()`, `translate()`
- `Style`, `Paint`, `Color`, `PremultColor`, `GradientStop`, `FillRule`, `Effect`, `BlendMode` -- visual properties
- `StrokeAlign`, `StrokeCap`, `StrokeJoin` -- stroke properties
- `boolean::compute_boolean` -- boolean path operations
