# rendero-core

The core document model and scene graph crate for the Rendero engine.

**Design principle:** Make invalid states unrepresentable. If it compiles, the document is structurally valid.

---

## Overview

`rendero-core` defines the complete data model for a design document. It provides:

- **Scene graph** -- an arena-based tree (`DocumentTree`) with O(1) node access, ordered children (z-order), and cycle-proof structure
- **Document model** -- multi-page documents with serialization/snapshot support
- **Node types** -- exhaustive enum (`NodeKind`) covering Frame, Rectangle, Ellipse, Text, Vector, Image, BooleanOp, Component, Instance, Line, and Polygon
- **Visual properties** -- fills, strokes, effects, blend modes, gradients, transforms
- **Layout system** -- provider-based auto-layout with two engines (Taffy flexbox and a legacy single-pass fallback)
- **Hit testing** -- z-order-aware point-in-node queries with ellipse precision and clip-content culling
- **Boolean operations** -- Union, Subtract, Intersect, Exclude via Sutherland-Hodgman polygon clipping
- **ID system** -- Lamport-clock-based globally unique IDs designed for CRDT operations

The crate has zero rendering logic. It is consumed by renderers (WebGPU, Canvas2D) that read the tree and draw nodes.

---

## Key Types

### Structs

| Struct | File | Description |
|---|---|---|
| `Node` | `node.rs` | A complete node: identity, geometry, visual style, and kind |
| `TextRun` | `node.rs` | A run of text with uniform styling (font, size, weight, color, decoration) |
| `VectorPath` | `node.rs` | A sequence of path commands with a fill rule |
| `Override` | `node.rs` | A property override targeting a node within a component instance |
| `Document` | `document.rs` | Top-level document containing pages and a clock generator |
| `Page` | `document.rs` | A single page with its own `DocumentTree` |
| `DocumentSnapshot` | `document.rs` | Serializable snapshot of an entire document |
| `PageSnapshot` | `document.rs` | Serializable snapshot of a single page |
| `DocumentTree` | `tree.rs` | Arena-based tree: HashMap of nodes + parent/child relationships |
| `FlatTree` | `tree.rs` | Serializable DFS-ordered representation of a tree |
| `ChildList` | `tree.rs` | Ordered list of children for a parent node (z-order) |
| `Style` | `properties.rs` | Visual styling: opacity, blend mode, fills, strokes, effects |
| `Transform` | `properties.rs` | 2D affine transform (3x2 column-major matrix) |
| `Color` | `properties.rs` | RGBA color, clamped to 0.0..=1.0 at construction |
| `PremultColor` | `properties.rs` | Premultiplied-alpha color for GPU rendering |
| `GradientStop` | `properties.rs` | A position + color pair within a gradient |
| `AutoLayout` | `properties.rs` | Auto-layout configuration: direction, spacing, padding, sizing, alignment |
| `NodeId` | `id.rs` | Unique node identifier (wraps `LogicalClock`) |
| `OpId` | `id.rs` | Unique operation identifier for CRDT |
| `LogicalClock` | `id.rs` | Lamport-like timestamp: counter + client_id |
| `ClockGen` | `id.rs` | Monotonic clock generator for a single client |
| `HitResult` | `hit_test.rs` | Hit test result: node_id + depth |
| `BooleanResult` | `boolean.rs` | Result of a boolean path operation: commands + fill rule |
| `LegacyLayout` | `layout.rs` | Original single-pass layout engine (fallback provider) |
| `TaffyLayout<M>` | `taffy_layout.rs` | CSS flexbox layout engine powered by Taffy |
| `HeuristicTextMeasurer` | `providers.rs` | Simple fallback text measurer using character-count heuristics |

### Enums

| Enum | File | Description |
|---|---|---|
| `NodeKind` | `node.rs` | What a node is: Frame, Rectangle, Ellipse, Line, Polygon, Vector, Text, BooleanOp, Component, Instance, Image |
| `PathCommand` | `node.rs` | Vector path command: MoveTo, LineTo, CubicTo, QuadTo, Close |
| `TextAlign` | `node.rs` | Horizontal text alignment: Left, Center, Right, Justified |
| `TextVerticalAlign` | `node.rs` | Vertical text alignment: Top, Center, Bottom |
| `TextDecoration` | `node.rs` | Text decoration: None, Underline, Strikethrough |
| `TextResize` | `node.rs` | Text auto-resize behavior: None, Height, WidthAndHeight, Truncate |
| `CornerRadii` | `node.rs` | Corner radii: Uniform(f32) or PerCorner { top_left, top_right, bottom_right, bottom_left } |
| `BooleanOperation` | `node.rs` | Boolean op type: Union, Subtract, Intersect, Exclude |
| `OverrideValue` | `node.rs` | What can be overridden on an instance: Style, Text, Visible |
| `Paint` | `properties.rs` | Fill/stroke paint: Solid, LinearGradient, RadialGradient, AngularGradient, DiamondGradient, Image |
| `ImageScaleMode` | `properties.rs` | How an image fill scales: Fill, Fit, Tile, Stretch |
| `BlendMode` | `properties.rs` | All blend modes (Normal, Multiply, Screen, Overlay, etc.) |
| `StrokeAlign` | `properties.rs` | Stroke alignment: Inside, Center, Outside |
| `StrokeCap` | `properties.rs` | Stroke cap: None, Round, Square |
| `StrokeJoin` | `properties.rs` | Stroke join: Miter, Round, Bevel |
| `FillRule` | `properties.rs` | Path fill rule: NonZero, EvenOdd |
| `Effect` | `properties.rs` | Visual effects: DropShadow, InnerShadow, LayerBlur, BackgroundBlur |
| `LayoutAlign` | `properties.rs` | Cross-axis alignment: Start, Center, End, Stretch |
| `LayoutDirection` | `properties.rs` | Auto-layout direction: Horizontal, Vertical |
| `SizingMode` | `properties.rs` | Sizing mode: Fixed, Hug, Fill |
| `ConstraintType` | `properties.rs` | Resize constraint: Min, Max, MinMax, Center, Scale |
| `TreeError` | `tree.rs` | Tree operation errors: ParentNotFound, NotAContainer, CannotRemoveRoot, CycleDetected |

### Traits

| Trait | File | Description |
|---|---|---|
| `LayoutEngine` | `providers.rs` | Interface for layout computation: `fn compute(&mut self, tree, root, viewport)` |
| `TextMeasurer` | `providers.rs` | Interface for text measurement: `fn measure(&self, runs, max_width) -> (f32, f32)` |

---

## Layout System

The layout system uses a **provider-based architecture**. The engine never depends on a concrete layout implementation -- it programs against traits.

### Entry Points

```rust
// Default convenience path: Taffy flexbox with heuristic text measurement
pub fn compute_layout(tree: &mut DocumentTree, root: &NodeId);

// Custom engine: swap in any LayoutEngine implementation
pub fn compute_layout_with(
    engine: &mut dyn LayoutEngine,
    tree: &mut DocumentTree,
    root: &NodeId,
    viewport: (f32, f32),
);
```

**File:** `layout.rs`

### Traits

**`LayoutEngine`** (`providers.rs`) -- the core abstraction. Any struct that implements this trait can compute layout for a `DocumentTree`.

```rust
pub trait LayoutEngine {
    fn compute(&mut self, tree: &mut DocumentTree, root: &NodeId, viewport: (f32, f32));
}
```

**`TextMeasurer`** (`providers.rs`) -- called by layout engines to size text nodes. The layout engine does not know how text is shaped; it delegates to this trait.

```rust
pub trait TextMeasurer {
    fn measure(&self, runs: &[TextRun], max_width: f32) -> (f32, f32);
}
```

### Implementations

#### TaffyLayout (`taffy_layout.rs`)

Full CSS flexbox layout powered by the [Taffy](https://github.com/DioxusLabs/taffy) crate. Generic over any `TextMeasurer`.

**Flow:**

1. Traverse the `DocumentTree` in DFS order
2. Build a parallel `TaffyTree` bottom-up (children created before parents)
3. Convert each Rendero node to a `taffy::Style` via `node_to_taffy_style()`
4. For text nodes, measure via the `TextMeasurer` and set explicit dimensions
5. Call `taffy.compute_layout()` with the viewport size
6. Apply computed positions and sizes back to the `DocumentTree` (skipping root)

Key mapping details:

| Rendero concept | Taffy mapping |
|---|---|
| `AutoLayout` on Frame | `Display::Flex` with direction, gap, padding, align_items |
| `LayoutDirection::Horizontal` | `FlexDirection::Row` |
| `LayoutDirection::Vertical` | `FlexDirection::Column` |
| `SizingMode::Hug` | `Dimension::auto()` |
| `SizingMode::Fill` (primary axis) | `flex_grow: 1.0, flex_basis: 0` |
| `SizingMode::Fill` (counter axis) | `AlignSelf::Stretch` |
| `SizingMode::Fixed` | `Dimension::length(value)` |
| Leaf nodes (no auto-layout) | `Display::Flex` with explicit width/height |
| Invisible nodes | `Display::None` |

#### LegacyLayout (`layout.rs`)

The original single-pass layout engine, kept as a fallback. Traverses bottom-up (reverse DFS), positions children along the primary axis with spacing, and handles hug/fill sizing modes.

It does not support CSS grid, flex-wrap, or percentage-based sizing.

#### HeuristicTextMeasurer (`providers.rs`)

Zero-dependency text measurer. Always available. Used as the default.

```
width  = sum(run.text.len() * run.font_size * 0.65)
height = max(run.font_size * 1.5)
```

Fast but inaccurate. It remains useful as the always-available fallback and as
the default convenience path inside `rendero-core`, but the higher-fidelity
platform paths now override it:

- native shells wire `ParleyTextMeasurer`
- WASM uses a browser-backed `BrowserTextMeasurer` adapter in `rendero-wasm`

The important architectural point is that `rendero-core` only depends on the
`TextMeasurer` trait, not on any concrete shaping library.

---

## Node Types

All node types are variants of the `NodeKind` enum in `node.rs`. The enum is exhaustive -- every renderer, CRDT operation, and export must handle all variants. The compiler enforces this via match exhaustiveness.

### Frame

```rust
Frame {
    clip_content: bool,
    auto_layout: Option<AutoLayout>,
    corner_radii: CornerRadii,
}
```

Container node. Can hold children, optionally has auto-layout (flexbox). `clip_content` controls whether children are clipped to the frame bounds. Containers: Frame, Component, Instance, and BooleanOp.

### Rectangle

```rust
Rectangle {
    corner_radii: CornerRadii,
}
```

Rectangle primitive with optional per-corner radii.

### Ellipse

```rust
Ellipse {
    arc_start: f32,     // radians, full ellipse = 0
    arc_end: f32,       // radians, full ellipse = TAU (2*PI)
    inner_radius_ratio: f32,  // 0.0 = solid, 0.5 = half-hollow donut
}
```

Supports arcs and donut shapes via the inner radius ratio.

### Text

```rust
Text {
    runs: Vec<TextRun>,
    align: TextAlign,
    vertical_align: TextVerticalAlign,
    resize: TextResize,
}
```

Rich text node with multiple styled runs. Each `TextRun` carries font family, size, weight, italic flag, color, letter spacing, line height, decoration, and an optional fill override for gradient text.

### Vector

```rust
Vector {
    paths: Vec<VectorPath>,
}
```

Arbitrary vector shape defined by path commands (MoveTo, LineTo, CubicTo, QuadTo, Close).

### Line

```rust
Line
```

A simple line from (0, 0) to (width, 0).

### Polygon

```rust
Polygon {
    point_count: u32,
    inner_radius_ratio: f32,  // 0.0 = regular polygon, 0.5 = star
}
```

Regular polygons and stars. `inner_radius_ratio > 0` creates star shapes with alternating inner/outer vertices.

### Image

```rust
Image {
    data: Vec<u8>,        // raw RGBA pixel data
    image_width: u32,     // source width in pixels
    image_height: u32,    // source height in pixels
}
```

Raster image node. Pixel data is stored inline.

### BooleanOp

```rust
BooleanOp {
    operation: BooleanOperation,  // Union, Subtract, Intersect, Exclude
}
```

Combines child shapes using boolean path operations. Implementation in `boolean.rs` uses Sutherland-Hodgman polygon clipping for intersections.

### Component

```rust
Component
```

A reusable component definition. Acts as a container (can hold children).

### Instance

```rust
Instance {
    component_id: NodeId,
    overrides: Vec<Override>,
}
```

An instance of a component. Only changed properties are stored as `Override` values (Style, Text, or Visible).

---

## Document Structure

The document hierarchy is:

```
Document
  |-- name: String
  |-- clock: ClockGen           (generates unique IDs)
  |-- pages: Vec<Page>
        |-- id: NodeId
        |-- name: String
        |-- tree: DocumentTree
              |-- root_id: NodeId
              |-- nodes: HashMap<NodeId, Node>     (O(1) access)
              |-- children: HashMap<NodeId, ChildList>  (ordered children)
              |-- parents: HashMap<NodeId, NodeId>      (parent lookup)
```

### DocumentTree internals

- **Arena-based**, not pointer-based. Nodes are stored in a `HashMap<NodeId, Node>`. Parent-child relationships are stored separately in `children` and `parents` maps.
- **Root node** is always present, created automatically with `NodeId::ROOT` (counter=0, client_id=0). It has infinite width/height.
- **Guarantees:**
  - Every node has exactly one parent (except root)
  - No cycles possible (checked by `is_descendant_of` before moves)
  - Children are ordered (ChildList maintains insertion order for z-ordering)
  - O(1) access by NodeId

### Key operations

```rust
// Create a tree
let tree = DocumentTree::new();

// Insert a node
tree.insert(node, parent_id, child_index)?;

// Remove a node and all descendants (returns removed nodes for undo)
let removed: Vec<Node> = tree.remove(&node_id)?;

// Move a node (cycle-safe)
tree.move_node(node_id, new_parent_id, index)?;

// Traversal
let ids: Vec<NodeId> = tree.traverse_depth_first(&root_id);

// Serialization round-trip
let flat: FlatTree = tree.to_flat();
let restored: DocumentTree = DocumentTree::from_flat(flat);
```

### Document-level API

```rust
let mut doc = Document::new("My Design", client_id);
let page_id = doc.add_page("Page 2");
let node_id = doc.next_id();
doc.add_node(page_index, node, parent_id, child_index)?;

// Snapshot round-trip (serializable via serde)
let snap: DocumentSnapshot = doc.to_snapshot();
let restored: Document = Document::from_snapshot(snap);
```

### Node construction helpers

`Node` provides factory methods for common node types:

```rust
Node::frame(id, "Container", 400.0, 300.0)
Node::rectangle(id, "Rect", 100.0, 50.0)
Node::ellipse(id, "Circle", 80.0, 80.0)
Node::text(id, "Label", "Hello", 16.0, Color::BLACK)
Node::component(id, "Button", 200.0, 48.0)
Node::instance(id, "Button Instance", component_id, 200.0, 48.0)
Node::image(id, "Photo", 300.0, 200.0, 1200, 800, rgba_bytes)
```

All factory methods set sensible defaults: visible=true, locked=false, identity transform, default style, Fixed sizing, Min constraints.

---

## Hit Testing

**File:** `hit_test.rs`

Point-in-node queries with z-order awareness. Returns nodes topmost-first.

```rust
// All nodes at a point, topmost first
let results: Vec<HitResult> = hit_test(&tree, &root, point);

// Just the topmost node
let top: Option<NodeId> = hit_test_top(&tree, &root, point);
```

Key behaviors:

- Invisible nodes are skipped
- Ellipses use precise `(dx/rx)^2 + (dy/ry)^2 <= 1` test (not bounding box)
- Frames with `clip_content=true` cull their children when the point is outside the frame bounds -- this turns O(n * children) into O(n) for large artboard counts
- Transparent containers (no fills, no strokes) are excluded from results
- Transforms are composed recursively (world-to-local via inverse transform)

---

## Boolean Operations

**File:** `boolean.rs`

Combines child shapes via path operations. All operations first flatten curves to polylines, operate on polygons, then emit `PathCommand` sequences.

| Operation | Strategy | Fill Rule |
|---|---|---|
| Union | Merge all paths | NonZero |
| Subtract | Keep first path, reverse winding of all others | NonZero |
| Intersect | Sutherland-Hodgman polygon clipping, iteratively | NonZero |
| Exclude | Merge all paths | EvenOdd |

The `node_to_path_commands()` function converts any node shape to local-space path commands (Rectangle to 4 LineTos, Ellipse to 4 CubicTos using the Kappa constant, Polygon/Star to computed vertices, Vector to its stored paths).

---

## Dependencies

| Crate | Purpose |
|---|---|
| `glam` | 2D math (`Vec2`) for transforms, hit testing, and path operations |
| `serde` | Serialization/deserialization for all types (Serialize + Deserialize derives) |
| `taffy` | CSS flexbox layout computation (used by `TaffyLayout`) |

---

## Tests

### `taffy_layout.rs` tests

| Test | What it verifies |
|---|---|
| `test_vertical_stack_positions` | Three 400x100 children in a vertical container with gap=10 are positioned at y=0, y=110, y=220 |
| `test_horizontal_row_positions` | Three children in a horizontal row with gap=20 are positioned at x=0, x=120, x=290 |
| `test_padding` | A child inside a padded container (20px top/bottom, 30px left/right) is offset to (30, 20) |
| `test_text_measurement_used` | Text node "Hello World" at 16px is measured by `HeuristicTextMeasurer` and gets width > 100, height > 20 |
| `test_nested_layout` | Outer vertical container with an inner horizontal row: inner row at y=0, sibling at y=110; inner children at x=0 and x=105 |
| `test_taffy_matches_expected_vertical` | Two children (80px and 60px tall) in a vertical container with gap=10: positioned at y=0 and y=90 |
| `test_custom_text_measurer` | A `MockTextMeasurer` returning fixed 200x40 dimensions is correctly used by `TaffyLayout`, overriding heuristic measurement |

### `boolean.rs` tests

| Test | What it verifies |
|---|---|
| `test_polygon_path_triangle` | `polygon_path(3, ...)` generates exactly 4 commands (MoveTo + 2 LineTo + Close) |
| `test_polygon_path_star` | `polygon_path(5, 0.5, ...)` generates 11 commands (10-vertex star) |
| `test_sutherland_hodgman_overlap` | Two overlapping squares produce an intersection polygon with all points within the expected 5x5 region |
| `test_sutherland_hodgman_no_overlap` | Two non-overlapping squares produce fewer than 3 points (empty intersection) |
| `test_flatten_rect_identity` | Flattening a rectangle path with identity transform produces exactly 4 polygon points |

---

## File Index

| File | Lines | Purpose |
|---|---|---|
| `lib.rs` | 16 | Module declarations |
| `node.rs` | 422 | Node types, NodeKind enum, factory methods |
| `properties.rs` | 374 | Visual properties (Color, Transform, Style, Paint, AutoLayout, etc.) |
| `tree.rs` | 357 | DocumentTree (arena-based), ChildList, FlatTree, TreeError |
| `document.rs` | 123 | Document and Page wrappers, snapshot serialization |
| `layout.rs` | 264 | Layout entry points and LegacyLayout engine |
| `taffy_layout.rs` | 466 | TaffyLayout engine with tests |
| `providers.rs` | 49 | LayoutEngine and TextMeasurer traits, HeuristicTextMeasurer |
| `id.rs` | 89 | NodeId, OpId, LogicalClock, ClockGen |
| `hit_test.rs` | 88 | Point-in-node hit testing with z-order |
| `boolean.rs` | 458 | Boolean path operations (Union, Subtract, Intersect, Exclude) |
