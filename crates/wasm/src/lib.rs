//! WASM bindings — expose the engine and renderer to JavaScript.
//! Thin wrapper. All logic in engine/renderer/crdt crates.

mod bench;
mod canvas2d;
mod fig_import;
mod webgl;

use wasm_bindgen::prelude::*;

use rendero_core::document::Document;
use rendero_core::hit_test;
use rendero_core::id::NodeId;
use rendero_core::node::{BooleanOperation, Node, NodeKind, PathCommand, TextRun, VectorPath};
use rendero_core::properties::*;
use rendero_crdt::apply;
use rendero_crdt::operation::{FractionalIndex, OpKind, Operation};
use rendero_renderer::pipeline;
use rendero_renderer::scene::{AABB, RenderItem};
use glam::Vec2;
use web_sys::CanvasRenderingContext2d;

/// Which part of a vector anchor is being edited.
#[derive(Clone, Copy, PartialEq)]
enum HandleType { In, Out }

/// Interaction mode state machine.
#[derive(Clone, PartialEq)]
enum InteractionMode {
    Idle,
    /// Dragging one or more selected nodes. `origins` stores (node_id, orig_tx, orig_ty) for each.
    Dragging { origins: Vec<(NodeId, f32, f32)>, start_x: f32, start_y: f32 },
    Resizing { node_id: NodeId, handle: ResizeHandle, start_x: f32, start_y: f32, orig_w: f32, orig_h: f32, orig_tx: f32, orig_ty: f32 },
    /// Rotating a selected node around its center.
    Rotating { node_id: NodeId, center_x: f32, center_y: f32, start_angle: f32, orig_transform: Transform },
    /// Marquee (lasso) selection: drag on empty space to select all nodes within the rectangle.
    MarqueeSelect { start_wx: f32, start_wy: f32, current_wx: f32, current_wy: f32 },
    EditingVector {
        vector_id: NodeId,
        point_index: usize,
        handle_type: Option<HandleType>, // None = anchor itself, Some = handle
        start_x: f32,
        start_y: f32,
        orig_x: f32,
        orig_y: f32,
    },
    /// Click-drag shape creation: user drags to define position and size.
    CreatingShape { shape_type: ShapeCreationType, start_wx: f32, start_wy: f32 },
}

#[derive(Clone, Copy, PartialEq)]
enum ShapeCreationType {
    Rectangle,
    Ellipse,
    Frame,
    Star,
    Text,
}

#[derive(Clone, Copy, PartialEq)]
enum ResizeHandle {
    TopLeft, TopRight, BottomLeft, BottomRight,
    Top, Right, Bottom, Left,
}

/// A reversible action for undo/redo.
/// Each variant stores enough to go BOTH directions.
#[derive(Clone)]
enum UndoAction {
    /// A node was added. Undo = remove it. Redo = re-add it.
    AddNode { node: Node, parent_id: NodeId },
    /// A node was removed. Undo = re-add it. Redo = remove it.
    RemoveNode { node: Node, parent_id: NodeId },
    /// A node was moved. Stores the position to restore.
    MoveNode { node_id: NodeId, tx: f32, ty: f32 },
    /// A node was resized. Stores the state to restore.
    ResizeNode { node_id: NodeId, tx: f32, ty: f32, w: f32, h: f32 },
    /// Fill color changed. Stores the fills to restore.
    ChangeFill { node_id: NodeId, fills: Vec<Paint> },
    /// Name changed. Stores the name to restore.
    ChangeName { node_id: NodeId, name: String },
    /// Text content changed. Stores the old runs to restore.
    ChangeText { node_id: NodeId, runs: Vec<TextRun>, width: f32, height: f32 },
    /// Vector path edited. Stores old paths + dimensions to restore.
    EditVector { node_id: NodeId, paths: Vec<VectorPath>, width: f32, height: f32, tx: f32, ty: f32 },
    /// Node was rotated. Stores the full transform to restore.
    RotateNode { node_id: NodeId, transform: Transform },
}

/// An anchor point extracted from a committed vector path for editing.
#[derive(Clone)]
struct EditAnchor {
    pos: Vec2,
    handle_in: Option<Vec2>,  // relative to pos
    handle_out: Option<Vec2>, // relative to pos
}

/// An anchor point in the pen tool path.
#[derive(Clone)]
struct PenAnchor {
    pos: Vec2,
    /// Control point for the curve arriving at this point (relative to pos).
    handle_in: Option<Vec2>,
    /// Control point for the curve leaving this point (relative to pos).
    handle_out: Option<Vec2>,
}

/// Extract anchor points from committed PathCommand sequences.
/// Returns (anchors, is_closed).
fn extract_anchors(paths: &[VectorPath]) -> (Vec<EditAnchor>, bool) {
    let mut anchors = Vec::new();
    let mut closed = false;

    for path in paths {
        let cmds = &path.commands;
        let mut i = 0;
        while i < cmds.len() {
            match &cmds[i] {
                PathCommand::MoveTo(p) => {
                    anchors.push(EditAnchor {
                        pos: *p,
                        handle_in: None,
                        handle_out: None,
                    });
                }
                PathCommand::LineTo(p) => {
                    anchors.push(EditAnchor {
                        pos: *p,
                        handle_in: None,
                        handle_out: None,
                    });
                }
                PathCommand::CubicTo { control1, control2, to } => {
                    // control1 is the outgoing handle of the PREVIOUS anchor
                    if let Some(prev) = anchors.last_mut() {
                        prev.handle_out = Some(*control1 - prev.pos);
                    }
                    // control2 is the incoming handle of THIS anchor
                    anchors.push(EditAnchor {
                        pos: *to,
                        handle_in: Some(*control2 - *to),
                        handle_out: None,
                    });
                }
                PathCommand::QuadTo { control, to } => {
                    // Approximate: treat quad control as both in/out handle
                    if let Some(prev) = anchors.last_mut() {
                        prev.handle_out = Some(*control - prev.pos);
                    }
                    anchors.push(EditAnchor {
                        pos: *to,
                        handle_in: Some(*control - *to),
                        handle_out: None,
                    });
                }
                PathCommand::Close => {
                    closed = true;
                    // For close: if there's a cubic arriving at first anchor, its handle_in
                    // was already set when that CubicTo was processed
                }
            }
            i += 1;
        }
    }
    (anchors, closed)
}

/// Rebuild PathCommand sequence from edited anchors.
fn rebuild_commands(anchors: &[EditAnchor], closed: bool) -> Vec<PathCommand> {
    if anchors.is_empty() {
        return Vec::new();
    }

    let mut cmds = Vec::new();
    cmds.push(PathCommand::MoveTo(anchors[0].pos));

    for i in 1..anchors.len() {
        let prev = &anchors[i - 1];
        let curr = &anchors[i];

        let has_handles = prev.handle_out.is_some() || curr.handle_in.is_some();
        if has_handles {
            let c1 = prev.pos + prev.handle_out.unwrap_or(Vec2::ZERO);
            let c2 = curr.pos + curr.handle_in.unwrap_or(Vec2::ZERO);
            cmds.push(PathCommand::CubicTo {
                control1: c1,
                control2: c2,
                to: curr.pos,
            });
        } else {
            cmds.push(PathCommand::LineTo(curr.pos));
        }
    }

    if closed && anchors.len() > 1 {
        let last = anchors.last().unwrap();
        let first = &anchors[0];
        let has_handles = last.handle_out.is_some() || first.handle_in.is_some();
        if has_handles {
            let c1 = last.pos + last.handle_out.unwrap_or(Vec2::ZERO);
            let c2 = first.pos + first.handle_in.unwrap_or(Vec2::ZERO);
            cmds.push(PathCommand::CubicTo {
                control1: c1,
                control2: c2,
                to: first.pos,
            });
        } else {
            cmds.push(PathCommand::LineTo(first.pos));
        }
        cmds.push(PathCommand::Close);
    }

    cmds
}

/// Compute bounding box from anchors (including handles for bezier overshoot).
fn anchors_bbox(anchors: &[EditAnchor]) -> (Vec2, Vec2) {
    let mut min = Vec2::new(f32::MAX, f32::MAX);
    let mut max = Vec2::new(f32::MIN, f32::MIN);
    for a in anchors {
        min = min.min(a.pos);
        max = max.max(a.pos);
        if let Some(h) = a.handle_out {
            let p = a.pos + h;
            min = min.min(p);
            max = max.max(p);
        }
        if let Some(h) = a.handle_in {
            let p = a.pos + h;
            min = min.min(p);
            max = max.max(p);
        }
    }
    (min, max)
}

/// A prototype interaction link between two nodes.
struct PrototypeLink {
    source_id: NodeId,
    target_id: NodeId,
    trigger: String, // "click", "hover", "drag"
    animation: String, // "instant", "dissolve", "slide"
}

/// A comment pin on the canvas.
struct Comment {
    id: u32,
    x: f32,
    y: f32,
    text: String,
    author: String,
    timestamp: f64,
    resolved: bool,
}

#[wasm_bindgen]
pub struct CanvasEngine {
    document: Document,
    selected: Vec<NodeId>,
    mode: InteractionMode,
    viewport_width: u32,
    viewport_height: u32,
    needs_render: bool,
    /// Pending CRDT operations to send to server.
    pending_ops: Vec<Operation>,
    /// Undo/redo stacks.
    undo_stack: Vec<UndoAction>,
    redo_stack: Vec<UndoAction>,
    /// Camera: pan offset in world units.
    cam_x: f32,
    cam_y: f32,
    /// Camera: zoom level (1.0 = 100%).
    cam_zoom: f32,
    /// Panning mode (space+drag or middle-click).
    panning: bool,
    pan_start_x: f32,
    pan_start_y: f32,
    pan_orig_cam_x: f32,
    pan_orig_cam_y: f32,
    /// Cached scene items (rebuilt only when tree changes, not on camera move).
    scene_cache: Option<Vec<rendero_renderer::scene::RenderItem>>,
    scene_cache_viewport: Option<rendero_renderer::scene::AABB>,
    /// NodeId → scene cache index for O(1) lookups in patch_scene_*.
    scene_node_index: std::collections::HashMap<NodeId, usize>,
    /// Spatial grid index for top-level artboards.
    /// Maps grid cell (col, row) → list of (scene_index, end_index) for artboards in that cell.
    /// Grid cell size chosen to give O(1) viewport lookups instead of O(100K) iteration.
    spatial_grid: std::collections::HashMap<(i32, i32), Vec<(usize, usize)>>,
    spatial_grid_cell_size: f32,
    /// Number of items drawn in last render (for diagnostics).
    last_drawn_count: usize,
    /// Current page index.
    current_page: usize,
    /// Pen tool state.
    pen_active: bool,
    pen_anchors: Vec<PenAnchor>,
    /// While dragging from an anchor, the current handle position (world coords).
    pen_dragging_handle: Option<Vec2>,
    /// Current mouse position in world coords (for preview line).
    pen_cursor: Vec2,
    /// Internal clipboard for copy/paste. Stores cloned nodes with offsets.
    clipboard: Vec<Node>,
    /// Snap-to-grid size. 0 = disabled.
    snap_grid: f32,
    /// Override parent for add_* calls. None = use page root.
    insert_parent: Option<NodeId>,
    /// Whether any imported node has Paint::Image fills (skip image overlay scan when false).
    has_image_fills: bool,
    /// Image bytes extracted from .fig ZIP, keyed by path (e.g. "images/abc123...").
    imported_images: std::collections::HashMap<String, Vec<u8>>,
    /// Currently entered group — when Some, clicks select children inside the group.
    entered_group: Option<NodeId>,
    /// For double-click detection: timestamp of last mouse_down.
    last_click_time: f64,
    /// For double-click detection: node_id of last click target.
    last_click_node: Option<NodeId>,
    /// Vector editing: which vector node is being point-edited.
    editing_vector: Option<NodeId>,
    /// Vector editing: selected anchor point index.
    vector_selected_point: Option<usize>,
    /// Vector editing: cached anchor extraction (avoids recomputing each frame).
    vector_edit_anchors: Vec<EditAnchor>,
    /// Vector editing: whether the path is closed.
    vector_edit_closed: bool,
    /// Vector editing: snapshot of paths before edit started (for undo).
    vector_edit_orig_paths: Vec<VectorPath>,
    vector_edit_orig_w: f32,
    vector_edit_orig_h: f32,
    vector_edit_orig_tx: f32,
    vector_edit_orig_ty: f32,
    /// Comments — annotation pins on the canvas.
    comments: Vec<Comment>,
    comment_counter: u32,
    /// Prototype interactions — links between nodes for click-through prototypes.
    prototype_links: Vec<PrototypeLink>,
    /// Pending shape creation type — set by start_creating(), consumed by next mousedown.
    pending_creation: Option<ShapeCreationType>,
    /// Text-on-arc params: NodeId → (radius, start_angle, letter_spacing).
    /// If a text node has an entry here, it renders along a circular arc instead of flat.
    text_arc_params: std::collections::HashMap<NodeId, (f32, f32, f32)>,
    /// Cached WebGL2 GPU state (shaders, buffers, VAOs). Initialized on first render_webgl call.
    webgl_state: Option<webgl::WebGlState>,
    /// Point clouds: GPU-direct rendering that bypasses the document tree.
    point_clouds: Vec<webgl::PointCloud>,
}

#[wasm_bindgen]
impl CanvasEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, client_id: u32) -> Self {
        Self {
            document: Document::new(name, client_id),
            selected: Vec::new(),
            mode: InteractionMode::Idle,
            viewport_width: 800,
            viewport_height: 600,
            needs_render: true,
            pending_ops: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            cam_x: 0.0,
            cam_y: 0.0,
            cam_zoom: 1.0,
            panning: false,
            pan_start_x: 0.0,
            pan_start_y: 0.0,
            pan_orig_cam_x: 0.0,
            pan_orig_cam_y: 0.0,
            scene_cache: None,
            scene_cache_viewport: None,
            scene_node_index: std::collections::HashMap::new(),
            spatial_grid: std::collections::HashMap::new(),
            spatial_grid_cell_size: 4000.0,
            last_drawn_count: 0,
            current_page: 0,
            pen_active: false,
            pen_anchors: Vec::new(),
            pen_dragging_handle: None,
            pen_cursor: Vec2::ZERO,
            clipboard: Vec::new(),
            snap_grid: 0.0,
            insert_parent: None,
            has_image_fills: false,
            imported_images: std::collections::HashMap::new(),
            entered_group: None,
            editing_vector: None,
            vector_selected_point: None,
            vector_edit_anchors: Vec::new(),
            vector_edit_closed: false,
            vector_edit_orig_paths: Vec::new(),
            vector_edit_orig_w: 0.0,
            vector_edit_orig_h: 0.0,
            vector_edit_orig_tx: 0.0,
            vector_edit_orig_ty: 0.0,
            last_click_time: 0.0,
            last_click_node: None,
            comments: Vec::new(),
            comment_counter: 0,
            prototype_links: Vec::new(),
            pending_creation: None,
            text_arc_params: std::collections::HashMap::new(),
            webgl_state: None,
            point_clouds: Vec::new(),
        }
    }

    /// Set snap-to-grid size. 0 = disabled, typical values: 1, 4, 8, 16, 32.
    pub fn set_snap_grid(&mut self, size: f32) {
        self.snap_grid = size.max(0.0);
    }

    /// Set parent for subsequent add_* calls (children go inside this node).
    pub fn set_insert_parent(&mut self, counter: u32, client_id: u32) {
        self.insert_parent = Some(NodeId::new(counter as u64, client_id));
    }

    /// Clear insert parent — subsequent adds go to page root.
    pub fn clear_insert_parent(&mut self) {
        self.insert_parent = None;
    }

    /// Get the effective parent ID for add operations.
    fn effective_parent(&self) -> NodeId {
        self.insert_parent.unwrap_or_else(|| {
            self.document.page(self.current_page).unwrap().tree.root_id()
        })
    }

    /// Get current snap grid size.
    pub fn get_snap_grid(&self) -> f32 {
        self.snap_grid
    }

    /// Mark that the tree changed — invalidates scene cache and triggers re-render.
    /// Invalidate the entire scene cache, forcing a full rebuild on next render.
    ///
    /// # Performance Warning
    ///
    /// **This is the most expensive operation in the app.** On a 1.8M node document,
    /// rebuilding the scene cache takes 500ms–5s. This method should ONLY be called for
    /// **structural changes** that invalidate the cache's contiguous layout:
    ///
    /// - Adding/removing nodes (tree shape changes)
    /// - Reparenting nodes (move_node, group, ungroup)
    /// - Z-order changes (bring to front, send to back)
    /// - Page switching (entirely different tree)
    /// - Importing documents (bulk tree construction)
    /// - Applying remote CRDT operations (arbitrary tree mutations)
    /// - Constraint-based layout (multiple children may move)
    ///
    /// For **leaf property changes** (fill, stroke, opacity, text color, font size,
    /// corner radius, position, size), use the incremental `patch_scene_*()` methods
    /// instead. These update a single item in O(n) scan time without rebuilding:
    ///
    /// - `patch_scene_transform()` — position/size changes
    /// - `patch_scene_style()` — fill, stroke, opacity, blend mode
    /// - `patch_scene_text()` — text content, color, font size
    /// - `patch_scene_shape()` — vector path edits
    /// - `scene_insert_leaf()` / `scene_remove_leaf()` — single node add/remove
    ///
    /// If you're adding a new property setter and reaching for `mark_dirty()`,
    /// stop and write an incremental updater instead.
    fn mark_dirty(&mut self) {
        self.needs_render = true;
        self.scene_cache = None;
        self.scene_cache_viewport = None;
        self.spatial_grid.clear();
        self.scene_node_index.clear();
    }

    /// Build NodeId → scene index map for O(1) lookups in patch_scene_*.
    fn rebuild_scene_node_index(&mut self) {
        self.scene_node_index.clear();
        if let Some(items) = self.scene_cache.as_ref() {
            self.scene_node_index.reserve(items.len());
            for (i, item) in items.iter().enumerate() {
                self.scene_node_index.insert(item.node_id, i);
            }
        }
    }

    /// Mark that only the overlay changed (selection, etc) — re-render without scene rebuild.
    /// The scene cache and spatial grid remain valid since the tree didn't change.
    fn mark_selection_dirty(&mut self) {
        self.needs_render = true;
    }

    /// Fast hit test using the scene cache (pre-computed world_bounds in contiguous array).
    /// O(visible_frames) instead of O(N) HashMap lookups.
    /// Falls back to tree-based hit test if cache is empty.
    fn hit_test_scene(&mut self, wx: f32, wy: f32) -> Option<NodeId> {
        // Ensure scene cache exists
        if self.scene_cache.is_none() {
            let page = self.document.page(self.current_page).unwrap();
            let root_id = page.tree.root_id();
            let full_viewport = AABB::new(
                Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec2::new(f32::INFINITY, f32::INFINITY),
            );
            let items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &full_viewport);
            self.scene_cache = Some(items);
            self.rebuild_scene_node_index();
        }
        let items = self.scene_cache.as_ref().unwrap();
        let point = Vec2::new(wx, wy);

        // Use spatial grid for O(1) lookup if available
        if !self.spatial_grid.is_empty() {
            let col = (wx / self.spatial_grid_cell_size).floor() as i32;
            let row = (wy / self.spatial_grid_cell_size).floor() as i32;
            let mut best: Option<NodeId> = None;
            if let Some(entries) = self.spatial_grid.get(&(col, row)) {
                for &(start, end) in entries {
                    let end = end.min(items.len());
                    let mut i = start;
                    while i < end {
                        let item = &items[i];
                        let inside = item.world_bounds.contains_point(point);
                        if !inside {
                            if item.clips && item.descendant_count > 0 {
                                i += 1 + item.descendant_count;
                                continue;
                            }
                            i += 1;
                            continue;
                        }
                        let is_container = item.clips && item.descendant_count > 0
                            && item.style.fills.is_empty() && item.style.strokes.is_empty();
                        if !is_container {
                            best = Some(item.node_id);
                        }
                        i += 1;
                    }
                }
            }
            return best;
        }

        // Fallback: scan all items
        let mut best: Option<NodeId> = None;
        let mut i = 0;
        while i < items.len() {
            let item = &items[i];
            let inside = item.world_bounds.contains_point(point);

            if !inside {
                if item.clips && item.descendant_count > 0 {
                    i += 1 + item.descendant_count;
                    continue;
                }
                i += 1;
                continue;
            }

            let is_container = item.clips && item.descendant_count > 0
                && item.style.fills.is_empty() && item.style.strokes.is_empty();
            if !is_container {
                best = Some(item.node_id);
            }

            i += 1;
        }

        // Group-aware selection: walk up to find the nearest group ancestor.
        // If we're inside an entered group, return the leaf directly (current behavior).
        // Otherwise, return the group frame if the leaf is inside a non-root group.
        if let Some(leaf_id) = best {
            if let Some(entered) = self.entered_group {
                // Inside a group — only return hits that are descendants of the entered group
                let page = self.document.page(self.current_page).unwrap();
                let mut id = leaf_id;
                let mut is_inside = false;
                while let Some(parent) = page.tree.parent_of(&id) {
                    if parent == entered {
                        is_inside = true;
                        break;
                    }
                    id = parent;
                }
                if is_inside { return Some(leaf_id); } else { return None; }
            }

            // Not inside a group — walk up to find nearest group ancestor.
            // A "group" is a non-clipping Frame (clip_content=false).
            // Artboards and regular frames clip (clip_content=true) and are not treated as groups.
            let page = self.document.page(self.current_page).unwrap();
            let root_id = page.tree.root_id();
            let mut id = leaf_id;
            let mut group_candidate: Option<NodeId> = None;
            while let Some(parent) = page.tree.parent_of(&id) {
                // Check if this parent is a group BEFORE breaking on root
                if let Some(parent_node) = page.tree.get(&parent) {
                    // A group = Frame with clip_content=false
                    if let NodeKind::Frame { clip_content: false, .. } = parent_node.kind {
                        group_candidate = Some(parent);
                    }
                }
                if parent == root_id {
                    break;
                }
                id = parent;
            }
            return Some(group_candidate.unwrap_or(leaf_id));
        }
        best
    }

    /// Patch a single node's transform/size in the cached scene items.
    /// O(1) lookup via scene_node_index, then O(descendants) propagation.
    fn patch_scene_transform(&mut self, node_id: NodeId, new_local_tx: f32, new_local_ty: f32, w: Option<f32>, h: Option<f32>) {
        let idx = match self.scene_node_index.get(&node_id) {
            Some(&i) => i,
            None => return,
        };
        if let Some(items) = self.scene_cache.as_mut() {
            if idx < items.len() && items[idx].node_id == node_id {
                    let old_world_tx = items[idx].world_transform.tx;
                    let old_world_ty = items[idx].world_transform.ty;

                    // Compute parent's world offset from scene cache.
                    // Parent is the item whose descendant range includes this node.
                    let mut parent_world_tx = 0.0f32;
                    let mut parent_world_ty = 0.0f32;
                    for pi in (0..idx).rev() {
                        let end = pi + 1 + items[pi].descendant_count;
                        if end > idx {
                            parent_world_tx = items[pi].world_transform.tx;
                            parent_world_ty = items[pi].world_transform.ty;
                            break;
                        }
                    }

                    // For simple (non-rotated) nodes: world = parent_world + local
                    // This handles the common case. For rotated parents we'd need
                    // full matrix composition, but Figma nodes rarely have rotation
                    // on container transforms during interactive dragging.
                    let new_world_tx = parent_world_tx + new_local_tx;
                    let new_world_ty = parent_world_ty + new_local_ty;
                    let dx = new_world_tx - old_world_tx;
                    let dy = new_world_ty - old_world_ty;

                    // Update the node itself
                    items[idx].world_transform.tx = new_world_tx;
                    items[idx].world_transform.ty = new_world_ty;
                    let iw = w.unwrap_or(items[idx].world_bounds.max.x - items[idx].world_bounds.min.x);
                    let ih = h.unwrap_or(items[idx].world_bounds.max.y - items[idx].world_bounds.min.y);
                    items[idx].world_bounds = AABB::new(
                        Vec2::new(new_world_tx, new_world_ty),
                        Vec2::new(new_world_tx + iw, new_world_ty + ih),
                    );

                    // Propagate delta to all descendants (group children move with parent)
                    let desc = items[idx].descendant_count;
                    if desc > 0 && (dx != 0.0 || dy != 0.0) {
                        for di in 1..=desc {
                            let child = &mut items[idx + di];
                            child.world_transform.tx += dx;
                            child.world_transform.ty += dy;
                            child.world_bounds = AABB::new(
                                Vec2::new(child.world_bounds.min.x + dx, child.world_bounds.min.y + dy),
                                Vec2::new(child.world_bounds.max.x + dx, child.world_bounds.max.y + dy),
                            );
                        }
                        // Spatial grid bounds changed for moved descendants
                        self.spatial_grid.clear();
                    }
                }
            }
        }

    /// Update the full transform matrix of a cached RenderItem in-place.
    /// Used for rotation changes — avoids full scene rebuild.
    fn patch_scene_full_transform(&mut self, node_id: NodeId, new_transform: rendero_core::properties::Transform, w: f32, h: f32) {
        let idx = match self.scene_node_index.get(&node_id) {
            Some(&i) => i,
            None => return,
        };
        if let Some(items) = self.scene_cache.as_mut() {
            if idx < items.len() && items[idx].node_id == node_id {
                // Find parent world transform
                let mut parent_tx = rendero_core::properties::Transform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };
                for pi in (0..idx).rev() {
                    let end = pi + 1 + items[pi].descendant_count;
                    if end > idx {
                        parent_tx = items[pi].world_transform;
                        break;
                    }
                }
                // world = parent * local
                let wt = rendero_core::properties::Transform {
                    a: parent_tx.a * new_transform.a + parent_tx.c * new_transform.b,
                    b: parent_tx.b * new_transform.a + parent_tx.d * new_transform.b,
                    c: parent_tx.a * new_transform.c + parent_tx.c * new_transform.d,
                    d: parent_tx.b * new_transform.c + parent_tx.d * new_transform.d,
                    tx: parent_tx.a * new_transform.tx + parent_tx.c * new_transform.ty + parent_tx.tx,
                    ty: parent_tx.b * new_transform.tx + parent_tx.d * new_transform.ty + parent_tx.ty,
                };
                items[idx].world_transform = wt;
                // Recompute bounds from transformed corners
                let corners = [
                    (wt.tx, wt.ty),
                    (wt.a * w + wt.tx, wt.b * w + wt.ty),
                    (wt.c * h + wt.tx, wt.d * h + wt.ty),
                    (wt.a * w + wt.c * h + wt.tx, wt.b * w + wt.d * h + wt.ty),
                ];
                let min_x = corners.iter().map(|c| c.0).fold(f32::INFINITY, f32::min);
                let min_y = corners.iter().map(|c| c.1).fold(f32::INFINITY, f32::min);
                let max_x = corners.iter().map(|c| c.0).fold(f32::NEG_INFINITY, f32::max);
                let max_y = corners.iter().map(|c| c.1).fold(f32::NEG_INFINITY, f32::max);
                items[idx].world_bounds = AABB::new(
                    Vec2::new(min_x, min_y),
                    Vec2::new(max_x, max_y),
                );
            }
        }
    }

    /// Update the style of a cached RenderItem in-place. O(1) via scene_node_index.
    /// Used for fill, stroke, opacity, blend mode changes — avoids full scene rebuild.
    fn patch_scene_style(&mut self, node_id: NodeId) {
        let style = {
            let page = match self.document.page(self.current_page) {
                Some(p) => p,
                None => return,
            };
            match page.tree.get(&node_id) {
                Some(n) => n.style.clone(),
                None => return,
            }
        };
        let idx = match self.scene_node_index.get(&node_id) {
            Some(&i) => i,
            None => return,
        };
        if let Some(items) = self.scene_cache.as_mut() {
            if idx < items.len() && items[idx].node_id == node_id {
                items[idx].style = style;
            }
        }
    }

    /// Update the shape and bounds of a cached RenderItem in-place. O(n) scan.
    /// Used for vector path edits — avoids full scene rebuild.
    fn patch_scene_shape(&mut self, node_id: NodeId) {
        let (shape, w, h, tx, ty) = {
            let page = match self.document.page(self.current_page) {
                Some(p) => p,
                None => return,
            };
            let node = match page.tree.get(&node_id) {
                Some(n) => n,
                None => return,
            };
            let shape = match &node.kind {
                NodeKind::Vector { paths } => {
                    let fill_rule = paths.first()
                        .map(|p| p.fill_rule)
                        .unwrap_or(FillRule::NonZero);
                    let commands: Vec<_> = paths.iter()
                        .flat_map(|p| p.commands.iter().cloned())
                        .collect();
                    rendero_renderer::scene::RenderShape::Path { commands, fill_rule }
                }
                _ => return,
            };
            (shape, node.width, node.height, node.transform.tx, node.transform.ty)
        };
        let idx = match self.scene_node_index.get(&node_id) {
            Some(&i) => i,
            None => return,
        };
        if let Some(items) = self.scene_cache.as_mut() {
            if idx < items.len() && items[idx].node_id == node_id {
                items[idx].shape = shape;
                items[idx].world_bounds = AABB::from_size(tx, ty, w, h);
                self.needs_render = true;
            }
        }
    }

    /// Update a text node's shape (runs) and style in the scene cache in-place.
    /// Text color lives in TextRun.color inside RenderShape::Text, not in Style.
    /// Avoids full scene rebuild on text color/content changes.
    fn patch_scene_text(&mut self, node_id: NodeId) {
        let (runs, width, height, align, vertical_align, style) = {
            let page = match self.document.page(self.current_page) {
                Some(p) => p,
                None => return,
            };
            let node = match page.tree.get(&node_id) {
                Some(n) => n,
                None => return,
            };
            if let NodeKind::Text { ref runs, align, vertical_align, .. } = node.kind {
                (runs.clone(), node.width, node.height, align, vertical_align, node.style.clone())
            } else {
                return;
            }
        };
        let idx = match self.scene_node_index.get(&node_id) {
            Some(&i) => i,
            None => return,
        };
        if let Some(items) = self.scene_cache.as_mut() {
            if idx < items.len() && items[idx].node_id == node_id {
                items[idx].shape = rendero_renderer::scene::RenderShape::Text {
                    runs, width, height, align, vertical_align,
                };
                items[idx].style = style;
                self.needs_render = true;
            }
        }
    }

    /// Update corner radii in a cached RenderItem's shape in-place. O(1) via scene_node_index.
    /// Corner radii only affect RenderShape::Rect — avoids full scene rebuild.
    fn patch_scene_corner_radii(&mut self, node_id: NodeId, radii: rendero_core::node::CornerRadii) {
        let idx = match self.scene_node_index.get(&node_id) {
            Some(&i) => i,
            None => return,
        };
        if let Some(items) = self.scene_cache.as_mut() {
            if idx < items.len() && items[idx].node_id == node_id {
                if let rendero_renderer::scene::RenderShape::Rect { ref mut corner_radii, .. } = items[idx].shape {
                    *corner_radii = radii;
                }
                self.needs_render = true;
            }
        }
    }

    /// Incrementally insert a leaf node into the scene cache.
    /// Avoids full 502ms rebuild for 1.8M items. O(N) scan to find parent, O(1) insert.
    /// Only works for leaf nodes (no children). For structural changes, use mark_dirty().
    fn scene_insert_leaf(&mut self, node: &rendero_core::node::Node, parent_id: NodeId) {
        let items = match self.scene_cache.as_mut() {
            Some(items) => items,
            None => return, // No cache to update
        };

        // Find parent's position via O(1) index lookup
        let parent_idx = self.scene_node_index.get(&parent_id).copied();
        let parent_idx = match parent_idx {
            Some(idx) => idx,
            None => {
                // Parent not in cache (e.g. invisible). Fall back to full rebuild.
                self.scene_cache = None;
                return;
            }
        };

        // Compute world transform from parent's cached transform
        let parent_wt = items[parent_idx].world_transform;
        let world_transform = node.transform.then(&parent_wt);
        let wx = world_transform.tx;
        let wy = world_transform.ty;
        let world_bounds = AABB::new(
            Vec2::new(wx, wy),
            Vec2::new(wx + node.width, wy + node.height),
        );

        // Build RenderShape from NodeKind
        let (shape, clips) = match &node.kind {
            rendero_core::node::NodeKind::Rectangle { corner_radii } => (
                rendero_renderer::scene::RenderShape::Rect {
                    width: node.width, height: node.height,
                    corner_radii: *corner_radii,
                }, false
            ),
            rendero_core::node::NodeKind::Ellipse { arc_start, arc_end, inner_radius_ratio } => (
                rendero_renderer::scene::RenderShape::Ellipse {
                    width: node.width, height: node.height,
                    arc_start: *arc_start, arc_end: *arc_end,
                    inner_radius_ratio: *inner_radius_ratio,
                }, false
            ),
            rendero_core::node::NodeKind::Text { runs, align, vertical_align, .. } => (
                rendero_renderer::scene::RenderShape::Text {
                    runs: runs.clone(), width: node.width, height: node.height,
                    align: *align, vertical_align: *vertical_align,
                }, false
            ),
            rendero_core::node::NodeKind::Frame { clip_content, corner_radii, .. } => (
                rendero_renderer::scene::RenderShape::Rect {
                    width: node.width, height: node.height,
                    corner_radii: *corner_radii,
                }, *clip_content
            ),
            rendero_core::node::NodeKind::Line => (
                rendero_renderer::scene::RenderShape::Line { length: node.width }, false
            ),
            _ => (
                rendero_renderer::scene::RenderShape::Rect {
                    width: node.width, height: node.height,
                    corner_radii: rendero_core::node::CornerRadii::default(),
                }, false
            ),
        };

        // Insert right after parent's last descendant
        let insert_at = parent_idx + 1 + items[parent_idx].descendant_count;
        let z_index = if insert_at > 0 { items[insert_at - 1].z_index + 1 } else { 0 };

        items.insert(insert_at, RenderItem {
            node_id: node.id,
            world_transform,
            world_bounds,
            style: node.style.clone(),
            shape,
            z_index,
            clips,
            descendant_count: 0,
            is_mask: false,
        });

        // Update ancestor descendant_counts
        // Walk backwards from parent to update all ancestors that contain this subtree
        items[parent_idx].descendant_count += 1;
        // Also update grandparents etc. — any ancestor whose range now includes insert_at
        for j in (0..parent_idx).rev() {
            let end = j + 1 + items[j].descendant_count;
            if end >= insert_at {
                items[j].descendant_count += 1;
            }
        }

        // Update scene_node_index: shift indices >= insert_at, add new entry
        for val in self.scene_node_index.values_mut() {
            if *val >= insert_at {
                *val += 1;
            }
        }
        self.scene_node_index.insert(node.id, insert_at);

        // Incrementally update spatial grid: shift indices >= insert_at by +1, add new item
        if !self.spatial_grid.is_empty() {
            // Shift existing entries
            for entries in self.spatial_grid.values_mut() {
                for entry in entries.iter_mut() {
                    if entry.0 >= insert_at {
                        entry.0 += 1;
                        entry.1 += 1;
                    } else if entry.1 > insert_at {
                        // Range spans the insertion point — extend end
                        entry.1 += 1;
                    }
                }
            }
            // Add new item to grid
            let b = &items[insert_at].world_bounds;
            let cell = self.spatial_grid_cell_size;
            let col_min = (b.min.x / cell).floor() as i32;
            let col_max = (b.max.x / cell).floor() as i32;
            let row_min = (b.min.y / cell).floor() as i32;
            let row_max = (b.max.y / cell).floor() as i32;
            for row in row_min..=row_max {
                for col in col_min..=col_max {
                    self.spatial_grid.entry((col, row))
                        .or_insert_with(Vec::new)
                        .push((insert_at, insert_at + 1));
                }
            }
        }
        self.needs_render = true;
    }

    /// Incrementally remove a leaf node from the scene cache.
    fn scene_remove_leaf(&mut self, node_id: NodeId) {
        let items = match self.scene_cache.as_mut() {
            Some(items) => items,
            None => return,
        };

        // O(1) lookup via scene_node_index
        let remove_idx = match self.scene_node_index.get(&node_id).copied() {
            Some(idx) => idx,
            None => return,
        };

        let desc_count = items[remove_idx].descendant_count;
        // Remove item + its descendants
        items.drain(remove_idx..=remove_idx + desc_count);

        // Update ancestor descendant_counts — only items with descendants can be ancestors
        let removed = 1 + desc_count;
        for j in (0..remove_idx).rev() {
            if items[j].descendant_count == 0 { continue; } // leaf, skip
            let end = j + 1 + items[j].descendant_count;
            if end >= remove_idx {
                items[j].descendant_count -= removed;
            }
        }

        // Update scene_node_index: remove entry and shift indices down
        self.scene_node_index.remove(&node_id);
        // Remove descendant entries too
        if desc_count > 0 {
            // Collect node_ids of removed descendants (they're gone from items, get from index)
            let to_remove: Vec<NodeId> = self.scene_node_index.iter()
                .filter(|(_, &v)| v >= remove_idx && v < remove_idx + removed)
                .map(|(k, _)| *k)
                .collect();
            for k in to_remove {
                self.scene_node_index.remove(&k);
            }
        }
        for val in self.scene_node_index.values_mut() {
            if *val >= remove_idx {
                *val -= removed;
            }
        }

        // Incrementally update spatial grid: remove entries for removed items, shift remaining
        if !self.spatial_grid.is_empty() {
            let removed = 1 + desc_count;
            for entries in self.spatial_grid.values_mut() {
                // Remove entries that reference removed items
                entries.retain(|&(start, _)| start < remove_idx || start >= remove_idx + removed);
                // Shift entries after remove_idx
                for entry in entries.iter_mut() {
                    if entry.0 >= remove_idx + removed {
                        entry.0 -= removed;
                        entry.1 -= removed;
                    } else if entry.1 > remove_idx {
                        // Range end extends past removed region — shrink
                        entry.1 = entry.1.saturating_sub(removed);
                    }
                }
            }
        }
        self.needs_render = true;
    }

    /// Mark dirty but only need style re-render, not full scene rebuild.
    fn mark_style_dirty(&mut self, node_id: NodeId) {
        self.patch_scene_style(node_id);
        self.needs_render = true;
    }

    /// Apply constraints to children when a parent frame is resized.
    /// old_w/old_h: previous size, new_w/new_h: current size.
    fn apply_constraints(&mut self, parent_id: NodeId, old_w: f32, old_h: f32, new_w: f32, new_h: f32) {
        use rendero_core::properties::ConstraintType;
        let dw = new_w - old_w;
        let dh = new_h - old_h;
        if dw == 0.0 && dh == 0.0 { return; }

        // Gather child IDs and their constraint info
        let children: Vec<(NodeId, ConstraintType, ConstraintType, f32, f32, f32, f32)> = {
            let page = match self.document.page(self.current_page) {
                Some(p) => p,
                None => return,
            };
            let child_ids: Vec<NodeId> = match page.tree.children_of(&parent_id) {
                Some(c) => c.iter().copied().collect(),
                None => return,
            };
            child_ids.iter().filter_map(|cid| {
                let n = page.tree.get(cid)?;
                Some((*cid, n.constraint_horizontal, n.constraint_vertical,
                    n.transform.tx, n.transform.ty, n.width, n.height))
            }).collect()
        };

        // Apply constraints
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return,
        };
        for (cid, ch, cv, tx, ty, w, h) in &children {
            let child = match page.tree.get_mut(&cid) {
                Some(n) => n,
                None => continue,
            };
            // Horizontal constraint
            match ch {
                ConstraintType::Min => { /* pinned to left — do nothing */ }
                ConstraintType::Max => {
                    // Pin to right edge: move by dw
                    child.transform.tx = tx + dw;
                }
                ConstraintType::MinMax => {
                    // Stretch: keep left, grow width
                    child.width = (w + dw).max(1.0);
                }
                ConstraintType::Center => {
                    // Keep centered: move by half dw
                    child.transform.tx = tx + dw * 0.5;
                }
                ConstraintType::Scale => {
                    // Scale proportionally
                    if old_w > 0.0 {
                        let ratio = new_w / old_w;
                        child.transform.tx = tx * ratio;
                        child.width = w * ratio;
                    }
                }
            }
            // Vertical constraint
            match cv {
                ConstraintType::Min => { /* pinned to top — do nothing */ }
                ConstraintType::Max => {
                    child.transform.ty = ty + dh;
                }
                ConstraintType::MinMax => {
                    child.height = (h + dh).max(1.0);
                }
                ConstraintType::Center => {
                    child.transform.ty = ty + dh * 0.5;
                }
                ConstraintType::Scale => {
                    if old_h > 0.0 {
                        let ratio = new_h / old_h;
                        child.transform.ty = ty * ratio;
                        child.height = h * ratio;
                    }
                }
            }
        }
    }

    pub fn set_viewport(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;
        self.needs_render = true; // camera-only, keep cache
    }

    /// Convert screen coordinates to world coordinates.
    fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        (sx / self.cam_zoom + self.cam_x, sy / self.cam_zoom + self.cam_y)
    }

    /// Compute world position (tx, ty) of a node by walking up the parent chain.
    /// For top-level nodes this equals node.transform.tx/ty.
    /// For nested nodes this composes all ancestor transforms.
    fn node_world_pos(&self, node_id: &NodeId) -> (f32, f32) {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return (0.0, 0.0),
        };
        let node = match page.tree.get(node_id) {
            Some(n) => n,
            None => return (0.0, 0.0),
        };
        let mut wx = node.transform.tx;
        let mut wy = node.transform.ty;
        let mut cur = *node_id;
        while let Some(parent_id) = page.tree.parent_of(&cur) {
            if let Some(parent) = page.tree.get(&parent_id) {
                wx += parent.transform.tx;
                wy += parent.transform.ty;
            }
            cur = parent_id;
        }
        (wx, wy)
    }

    /// Get a node's world-space bounding box: [x, y, width, height].
    /// Accounts for all parent transforms (works at any nesting depth).
    pub fn get_node_world_bounds(&self, counter: u32, client_id: u32) -> Vec<f32> {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let (wx, wy) = self.node_world_pos(&node_id);
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return vec![0.0, 0.0, 0.0, 0.0],
        };
        let node = match page.tree.get(&node_id) {
            Some(n) => n,
            None => return vec![0.0, 0.0, 0.0, 0.0],
        };
        vec![wx, wy, node.width, node.height]
    }

    /// Zoom centered on a screen point. delta > 0 zooms in, < 0 zooms out.
    pub fn zoom(&mut self, delta: f32, screen_x: f32, screen_y: f32) {
        let (wx, wy) = self.screen_to_world(screen_x, screen_y);
        let factor = if delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
        self.cam_zoom = (self.cam_zoom * factor).clamp(0.02, 256.0);
        // Adjust pan so the world point under cursor stays fixed
        self.cam_x = wx - screen_x / self.cam_zoom;
        self.cam_y = wy - screen_y / self.cam_zoom;
        self.needs_render = true; // camera-only, keep cache
    }

    /// Start panning (called on middle-click down or space+click).
    pub fn pan_start(&mut self, screen_x: f32, screen_y: f32) {
        self.panning = true;
        self.pan_start_x = screen_x;
        self.pan_start_y = screen_y;
        self.pan_orig_cam_x = self.cam_x;
        self.pan_orig_cam_y = self.cam_y;
    }

    /// Continue panning.
    pub fn pan_move(&mut self, screen_x: f32, screen_y: f32) {
        if !self.panning { return; }
        let dx = (screen_x - self.pan_start_x) / self.cam_zoom;
        let dy = (screen_y - self.pan_start_y) / self.cam_zoom;
        self.cam_x = self.pan_orig_cam_x - dx;
        self.cam_y = self.pan_orig_cam_y - dy;
        self.needs_render = true; // camera-only, keep cache
    }

    /// Stop panning.
    pub fn pan_end(&mut self) {
        self.panning = false;
    }

    /// Get current camera state as [cam_x, cam_y, zoom].
    pub fn get_camera(&self) -> Vec<f32> {
        vec![self.cam_x, self.cam_y, self.cam_zoom]
    }

    /// Set camera position and zoom directly.
    pub fn set_camera(&mut self, x: f32, y: f32, zoom: f32) {
        self.cam_x = x;
        self.cam_y = y;
        self.cam_zoom = zoom;
        self.needs_render = true;
    }

    /// Add a rectangle. Returns node ID as [counter, client_id].
    pub fn add_rectangle(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        r: f32, g: f32, b: f32, a: f32,
    ) -> Vec<u32> {
        let id = self.document.next_id();
        let mut node = Node::rectangle(id, name, width, height);
        node.transform = Transform::translate(x, y);
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        // Incremental scene update: insert into cache without full rebuild
        self.scene_insert_leaf(&node, parent_id);
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add an ellipse. Returns node ID as [counter, client_id].
    pub fn add_ellipse(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        r: f32, g: f32, b: f32, a: f32,
    ) -> Vec<u32> {
        let id = self.document.next_id();
        let mut node = Node::ellipse(id, name, width, height);
        node.transform = Transform::translate(x, y);
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        self.scene_insert_leaf(&node, parent_id);
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add a text node. Returns node ID as [counter, client_id].
    pub fn add_text(
        &mut self, name: &str, content: &str, x: f32, y: f32,
        font_size: f32, r: f32, g: f32, b: f32, a: f32,
    ) -> Vec<u32> {
        let id = self.document.next_id();
        let color = Color::new(r, g, b, a);
        let mut node = Node::text(id, name, content, font_size, color);
        node.transform = Transform::translate(x, y);

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        self.scene_insert_leaf(&node, parent_id);
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add a rectangle with a linear gradient fill.
    /// stop_positions and stop_colors are parallel arrays. Each color is [r, g, b, a].
    pub fn add_gradient_rectangle(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        start_x: f32, start_y: f32, end_x: f32, end_y: f32,
        stop_positions: Vec<f32>, stop_colors: Vec<f32>,
    ) -> Vec<u32> {
        let id = self.document.next_id();
        let mut node = Node::rectangle(id, name, width, height);
        node.transform = Transform::translate(x, y);

        let mut stops = Vec::new();
        for i in 0..stop_positions.len() {
            let ci = i * 4;
            if ci + 3 < stop_colors.len() {
                stops.push(GradientStop::new(
                    stop_positions[i],
                    Color::new(stop_colors[ci], stop_colors[ci+1], stop_colors[ci+2], stop_colors[ci+3]),
                ));
            }
        }

        node.style.fills.push(Paint::LinearGradient {
            stops,
            start: Vec2::new(start_x, start_y),
            end: Vec2::new(end_x, end_y),
        });

        let parent_id = self.effective_parent();
        let node_for_scene = node.clone();
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.scene_insert_leaf(&node_for_scene, parent_id);
        self.needs_render = true;
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add a frame.
    pub fn add_frame(
        &mut self, name: &str, x: f32, y: f32, w: f32, h: f32,
        r: f32, g: f32, b: f32, a: f32,
    ) -> Vec<u32> {
        let id = self.document.next_id();
        let mut node = Node::frame(id, name, w, h);
        node.transform = Transform::translate(x, y);
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        self.scene_insert_leaf(&node, parent_id);
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add a line from (x1,y1) to (x2,y2) with stroke color.
    pub fn add_line(
        &mut self, name: &str, x1: f32, y1: f32, x2: f32, y2: f32,
        r: f32, g: f32, b: f32, a: f32, stroke_width: f32,
    ) -> Vec<u32> {
        use rendero_core::node::{PathCommand, VectorPath};

        let min_x = x1.min(x2);
        let min_y = y1.min(y2);
        let w = (x2 - x1).abs().max(1.0);
        let h = (y2 - y1).abs().max(1.0);

        let id = self.document.next_id();
        let mut node = Node::rectangle(id, name, w, h);
        node.kind = NodeKind::Vector {
            paths: vec![VectorPath {
                commands: vec![
                    PathCommand::MoveTo(Vec2::new(x1 - min_x, y1 - min_y)),
                    PathCommand::LineTo(Vec2::new(x2 - min_x, y2 - min_y)),
                ],
                fill_rule: FillRule::NonZero,
            }],
        };
        node.transform = Transform::translate(min_x, min_y);
        node.style.fills.clear(); // Lines have no fill
        node.style.strokes.push(Paint::Solid(Color::new(r, g, b, a)));
        node.style.stroke_weight = stroke_width;

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        self.pending_ops.push(Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        });
        let node_for_undo = node.clone();
        let node_for_scene = node.clone();
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        self.scene_insert_leaf(&node_for_scene, parent_id);
        self.needs_render = true;
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add an image node from raw RGBA pixel data.
    /// Returns node ID as [counter, client_id].
    pub fn add_image(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        image_data: Vec<u8>, image_width: u32, image_height: u32,
    ) -> Vec<u32> {
        let id = self.document.next_id();
        let node = {
            let mut n = Node::image(id, name, width, height, image_width, image_height, image_data);
            n.transform = Transform::translate(x, y);
            n
        };

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        let node_for_scene = node.clone();
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        self.has_image_fills = true;
        self.scene_insert_leaf(&node_for_scene, parent_id);
        self.needs_render = true;
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add a rectangle with an image fill (URL-based, loaded by renderer).
    /// `path` is relative to /imports/ (e.g. "starbucks.png").
    /// `scale_mode`: "fill", "fit", "tile", "stretch".
    pub fn add_image_fill(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        path: &str, scale_mode: &str, opacity: f32,
    ) -> Vec<u32> {
        let sm = match scale_mode {
            "fit" => ImageScaleMode::Fit,
            "tile" => ImageScaleMode::Tile,
            "stretch" => ImageScaleMode::Stretch,
            _ => ImageScaleMode::Fill,
        };

        let id = self.document.next_id();
        let mut node = Node::rectangle(id, name, width, height);
        node.transform = Transform::translate(x, y);
        node.style.fills.clear();
        node.style.fills.push(Paint::Image {
            path: path.to_string(),
            scale_mode: sm,
            opacity: opacity.clamp(0.0, 1.0),
        });

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        self.pending_ops.push(Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        });
        let node_for_undo = node.clone();
        let node_for_scene = node.clone();
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        self.has_image_fills = true;
        self.scene_insert_leaf(&node_for_scene, parent_id);
        self.needs_render = true;
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Set image fill on an existing node (replace all fills with an image fill).
    pub fn set_image_fill(
        &mut self, counter: u32, client_id: u32,
        path: &str, scale_mode: &str, opacity: f32,
    ) -> bool {
        let node_id = NodeId::new(counter as u64, client_id);
        let sm = match scale_mode {
            "fit" => ImageScaleMode::Fit,
            "tile" => ImageScaleMode::Tile,
            "stretch" => ImageScaleMode::Stretch,
            _ => ImageScaleMode::Fill,
        };
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.fills.clear();
        node.style.fills.push(Paint::Image {
            path: path.to_string(),
            scale_mode: sm,
            opacity: opacity.clamp(0.0, 1.0),
        });
        self.has_image_fills = true;
        self.patch_scene_style(node_id);
        self.needs_render = true;
        true
    }

    /// Add a vector shape from flat path data.
    /// Format: each command is [type, ...args]
    ///   0, x, y           = MoveTo
    ///   1, x, y           = LineTo
    ///   2, c1x, c1y, c2x, c2y, x, y = CubicTo
    ///   3                 = Close
    /// `width`/`height` = bounding box for hit-testing.
    pub fn add_vector(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        r: f32, g: f32, b: f32, a: f32,
        path_data: Vec<f32>,
    ) -> Vec<u32> {
        use rendero_core::node::{PathCommand, VectorPath};

        let mut commands = Vec::new();
        let mut i = 0;
        while i < path_data.len() {
            let cmd_type = path_data[i] as u8;
            match cmd_type {
                0 if i + 2 < path_data.len() => {
                    commands.push(PathCommand::MoveTo(Vec2::new(path_data[i+1], path_data[i+2])));
                    i += 3;
                }
                1 if i + 2 < path_data.len() => {
                    commands.push(PathCommand::LineTo(Vec2::new(path_data[i+1], path_data[i+2])));
                    i += 3;
                }
                2 if i + 6 < path_data.len() => {
                    commands.push(PathCommand::CubicTo {
                        control1: Vec2::new(path_data[i+1], path_data[i+2]),
                        control2: Vec2::new(path_data[i+3], path_data[i+4]),
                        to: Vec2::new(path_data[i+5], path_data[i+6]),
                    });
                    i += 7;
                }
                3 => {
                    commands.push(PathCommand::Close);
                    i += 1;
                }
                _ => { i += 1; } // skip unknown
            }
        }

        let id = self.document.next_id();
        let mut node = Node::rectangle(id, name, width, height); // start with rect, then override kind
        node.kind = NodeKind::Vector {
            paths: vec![VectorPath {
                commands,
                fill_rule: FillRule::NonZero,
            }],
        };
        node.transform = Transform::translate(x, y);
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        let node_for_scene = node.clone();
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        self.scene_insert_leaf(&node_for_scene, parent_id);
        self.needs_render = true;
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Add a star/polygon. `points` = number of outer points (3=triangle, 5=star, 6=hexagon).
    /// `inner_ratio` = inner radius / outer radius (0.0..1.0). Use 1.0 for regular polygon.
    pub fn add_star(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        r: f32, g: f32, b: f32, a: f32,
        points: u32, inner_ratio: f32,
    ) -> Vec<u32> {
        use rendero_core::node::{PathCommand, VectorPath};

        let cx = width / 2.0;
        let cy = height / 2.0;
        let rx = width / 2.0;
        let ry = height / 2.0;
        let inner_rx = rx * inner_ratio.clamp(0.01, 1.0);
        let inner_ry = ry * inner_ratio.clamp(0.01, 1.0);
        let n = points.max(3) as usize;
        let is_polygon = (inner_ratio - 1.0).abs() < 0.01;

        let mut commands = Vec::new();
        let total_verts = if is_polygon { n } else { n * 2 };
        let angle_step = std::f32::consts::TAU / total_verts as f32;
        let start_angle = -std::f32::consts::FRAC_PI_2; // point up

        for i in 0..total_verts {
            let angle = start_angle + angle_step * i as f32;
            let (is_outer, r_x, r_y) = if is_polygon {
                (true, rx, ry)
            } else if i % 2 == 0 {
                (true, rx, ry)
            } else {
                (false, inner_rx, inner_ry)
            };
            let _ = is_outer;
            let px = cx + r_x * angle.cos();
            let py = cy + r_y * angle.sin();
            if i == 0 {
                commands.push(PathCommand::MoveTo(Vec2::new(px, py)));
            } else {
                commands.push(PathCommand::LineTo(Vec2::new(px, py)));
            }
        }
        commands.push(PathCommand::Close);

        let id = self.document.next_id();
        let mut node = Node::rectangle(id, name, width, height);
        node.kind = NodeKind::Vector {
            paths: vec![VectorPath {
                commands,
                fill_rule: FillRule::NonZero,
            }],
        };
        node.transform = Transform::translate(x, y);
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        let node_for_scene = node.clone();
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        self.scene_insert_leaf(&node_for_scene, parent_id);
        self.needs_render = true;
        vec![id.0.counter as u32, id.0.client_id]
    }

    // ─── Pen tool ──────────────────────────────────────────

    /// Enter pen drawing mode.
    pub fn pen_start(&mut self) {
        self.pen_active = true;
        self.pen_anchors.clear();
        self.pen_dragging_handle = None;
        self.selected.clear();
        self.mark_selection_dirty();
    }

    /// Is the pen tool currently active?
    pub fn pen_is_active(&self) -> bool {
        self.pen_active
    }

    /// Cancel pen tool and discard the path.
    pub fn pen_cancel(&mut self) {
        self.pen_active = false;
        self.pen_anchors.clear();
        self.pen_dragging_handle = None;
        self.mark_selection_dirty();
    }

    // ── Click-drag shape creation ─────────────────────────────────────
    /// Start shape creation mode. Next mousedown+drag will create the shape.
    /// shape_type: "rect", "ellipse", "frame", "star", "text"
    pub fn start_creating(&mut self, shape_type: &str) {
        self.pending_creation = match shape_type {
            "rect" | "rectangle" => Some(ShapeCreationType::Rectangle),
            "ellipse" | "circle" => Some(ShapeCreationType::Ellipse),
            "frame" => Some(ShapeCreationType::Frame),
            "star" => Some(ShapeCreationType::Star),
            "text" => Some(ShapeCreationType::Text),
            _ => None,
        };
    }

    /// Whether we're in creation mode (waiting for mousedown).
    pub fn is_creating(&self) -> bool {
        self.pending_creation.is_some() || matches!(self.mode, InteractionMode::CreatingShape { .. })
    }

    /// Get creation preview rectangle [x, y, w, h] in world coords during drag.
    /// Returns empty vec if not currently dragging a creation.
    pub fn get_creation_preview(&self) -> Vec<f32> {
        if let InteractionMode::CreatingShape { start_wx, start_wy, .. } = self.mode {
            let (cx, cy) = (self.pen_cursor.x, self.pen_cursor.y); // reuse pen_cursor for current mouse
            let x = start_wx.min(cx);
            let y = start_wy.min(cy);
            let w = (cx - start_wx).abs().max(1.0);
            let h = (cy - start_wy).abs().max(1.0);
            vec![x, y, w, h]
        } else {
            vec![]
        }
    }

    /// Cancel creation mode.
    pub fn cancel_creating(&mut self) {
        self.pending_creation = None;
        if matches!(self.mode, InteractionMode::CreatingShape { .. }) {
            self.mode = InteractionMode::Idle;
        }
    }

    /// Mouse down in pen mode (screen coords). Adds an anchor.
    /// If clicking near the first anchor, closes the path.
    pub fn pen_mouse_down(&mut self, sx: f32, sy: f32) {
        if !self.pen_active { return; }
        let (wx, wy) = self.screen_to_world(sx, sy);
        let pos = Vec2::new(wx, wy);

        // Check if clicking near first anchor to close path
        if self.pen_anchors.len() >= 2 {
            let first = self.pen_anchors[0].pos;
            let dist = (pos - first).length();
            if dist < 8.0 / self.cam_zoom {
                self.pen_finish_closed();
                return;
            }
        }

        self.pen_anchors.push(PenAnchor {
            pos,
            handle_in: None,
            handle_out: None,
        });
        self.pen_dragging_handle = Some(pos);
        // Pen anchors are overlay-only — don't invalidate scene cache
        self.mark_selection_dirty();
    }

    /// Mouse drag in pen mode (screen coords). Creates curve handles.
    pub fn pen_mouse_drag(&mut self, sx: f32, sy: f32) {
        if !self.pen_active { return; }
        let (wx, wy) = self.screen_to_world(sx, sy);
        let handle_pos = Vec2::new(wx, wy);

        if let Some(last) = self.pen_anchors.last_mut() {
            let delta = handle_pos - last.pos;
            last.handle_out = Some(delta);
            last.handle_in = Some(-delta); // symmetric handles
        }
        self.pen_dragging_handle = Some(handle_pos);
        self.pen_cursor = handle_pos;
        // Pen handles are overlay-only — don't invalidate scene cache
        self.mark_selection_dirty();
    }

    /// Mouse up in pen mode.
    pub fn pen_mouse_up(&mut self) {
        self.pen_dragging_handle = None;
        // Pen state is overlay-only — don't invalidate scene cache
        self.mark_selection_dirty();
    }

    /// Mouse move in pen mode (for preview line).
    pub fn pen_mouse_move(&mut self, sx: f32, sy: f32) {
        if !self.pen_active { return; }
        let (wx, wy) = self.screen_to_world(sx, sy);
        self.pen_cursor = Vec2::new(wx, wy);
        // Pen cursor is overlay-only — don't invalidate scene cache
        self.mark_selection_dirty();
    }

    /// Finish pen path as open path (double-click or Enter).
    pub fn pen_finish_open(&mut self) {
        if self.pen_anchors.len() < 2 {
            self.pen_cancel();
            return;
        }
        self.commit_pen_path(false);
    }

    /// Finish pen path as closed path (click on first anchor).
    pub fn pen_finish_closed(&mut self) {
        if self.pen_anchors.len() < 3 {
            self.pen_cancel();
            return;
        }
        self.commit_pen_path(true);
    }

    /// Convert pen anchors to a Vector node and add to document.
    fn commit_pen_path(&mut self, closed: bool) {
        use rendero_core::node::{PathCommand, VectorPath, NodeKind};

        let mut commands = Vec::new();
        let anchors = &self.pen_anchors;

        // MoveTo first point
        commands.push(PathCommand::MoveTo(anchors[0].pos));

        for i in 1..anchors.len() {
            let prev = &anchors[i - 1];
            let curr = &anchors[i];

            let has_handles = prev.handle_out.is_some() || curr.handle_in.is_some();
            if has_handles {
                let c1 = prev.pos + prev.handle_out.unwrap_or(Vec2::ZERO);
                let c2 = curr.pos + curr.handle_in.unwrap_or(Vec2::ZERO);
                commands.push(PathCommand::CubicTo {
                    control1: c1,
                    control2: c2,
                    to: curr.pos,
                });
            } else {
                commands.push(PathCommand::LineTo(curr.pos));
            }
        }

        // Close path: curve from last to first
        if closed {
            let last = anchors.last().unwrap();
            let first = &anchors[0];
            let has_handles = last.handle_out.is_some() || first.handle_in.is_some();
            if has_handles {
                let c1 = last.pos + last.handle_out.unwrap_or(Vec2::ZERO);
                let c2 = first.pos + first.handle_in.unwrap_or(Vec2::ZERO);
                commands.push(PathCommand::CubicTo {
                    control1: c1,
                    control2: c2,
                    to: first.pos,
                });
            } else {
                commands.push(PathCommand::LineTo(first.pos));
            }
            commands.push(PathCommand::Close);
        }

        // Compute bounding box for node dimensions
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for anchor in anchors {
            min_x = min_x.min(anchor.pos.x);
            min_y = min_y.min(anchor.pos.y);
            max_x = max_x.max(anchor.pos.x);
            max_y = max_y.max(anchor.pos.y);
            // Also include handles in bounding box
            if let Some(h) = anchor.handle_out {
                let hp = anchor.pos + h;
                min_x = min_x.min(hp.x); min_y = min_y.min(hp.y);
                max_x = max_x.max(hp.x); max_y = max_y.max(hp.y);
            }
            if let Some(h) = anchor.handle_in {
                let hp = anchor.pos + h;
                min_x = min_x.min(hp.x); min_y = min_y.min(hp.y);
                max_x = max_x.max(hp.x); max_y = max_y.max(hp.y);
            }
        }
        let width = (max_x - min_x).max(1.0);
        let height = (max_y - min_y).max(1.0);
        let origin = Vec2::new(min_x, min_y);

        // Convert absolute world coords to local coords (relative to bounding box origin)
        let localize = |p: Vec2| p - origin;
        for cmd in &mut commands {
            match cmd {
                PathCommand::MoveTo(ref mut p) => *p = localize(*p),
                PathCommand::LineTo(ref mut p) => *p = localize(*p),
                PathCommand::CubicTo { ref mut control1, ref mut control2, ref mut to } => {
                    *control1 = localize(*control1);
                    *control2 = localize(*control2);
                    *to = localize(*to);
                }
                PathCommand::QuadTo { ref mut control, ref mut to } => {
                    *control = localize(*control);
                    *to = localize(*to);
                }
                PathCommand::Close => {}
            }
        }

        // Create vector node positioned at bounding box origin
        let id = self.document.next_id();
        let mut node = Node {
            id,
            name: "Vector".to_string(),
            visible: true,
            locked: false,
            transform: Transform::translate(min_x, min_y),
            width,
            height,
            style: rendero_core::properties::Style::default(),
            kind: NodeKind::Vector {
                paths: vec![VectorPath {
                    commands,
                    fill_rule: FillRule::NonZero,
                }],
            },
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        };
        // White stroke for visibility
        node.style.fills.push(Paint::Solid(Color::new(1.0, 1.0, 1.0, 1.0)));

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();

        self.pen_active = false;
        self.pen_anchors.clear();
        self.pen_dragging_handle = None;
        // Structural: new vector node was added to the tree.
        self.mark_dirty();
    }

    /// Get pen path data for overlay rendering.
    /// Returns JSON: { anchors: [{x,y,hox,hoy,hix,hiy}], cursor: {x,y}, closed: false }
    pub fn pen_get_state(&self) -> String {
        if !self.pen_active || self.pen_anchors.is_empty() {
            return String::new();
        }
        let mut parts = Vec::new();
        for a in &self.pen_anchors {
            let hox = a.handle_out.map(|h| h.x).unwrap_or(0.0);
            let hoy = a.handle_out.map(|h| h.y).unwrap_or(0.0);
            let hix = a.handle_in.map(|h| h.x).unwrap_or(0.0);
            let hiy = a.handle_in.map(|h| h.y).unwrap_or(0.0);
            parts.push(format!(
                r#"{{"x":{:.1},"y":{:.1},"hox":{:.1},"hoy":{:.1},"hix":{:.1},"hiy":{:.1}}}"#,
                a.pos.x, a.pos.y, hox, hoy, hix, hiy
            ));
        }
        format!(
            r#"{{"anchors":[{}],"cx":{:.1},"cy":{:.1}}}"#,
            parts.join(","), self.pen_cursor.x, self.pen_cursor.y
        )
    }

    // ─── Vector point editing ────────────────────────────────

    /// Whether we're in vector point editing mode.
    pub fn is_vector_editing(&self) -> bool {
        self.editing_vector.is_some()
    }

    /// Check if screen coords are in the rotation zone (outside resize handles).
    pub fn is_rotation_zone(&self, sx: f32, sy: f32) -> bool {
        let (wx, wy) = self.screen_to_world(sx, sy);
        self.check_rotation_zone(wx, wy)
    }

    /// Get vector edit state as JSON for overlay rendering.
    /// Returns: {"anchors":[{x,y,hox,hoy,hix,hiy}],"selected":N,"closed":bool,"tx":F,"ty":F}
    pub fn vector_edit_get_state(&self) -> String {
        let vec_id = match self.editing_vector {
            Some(id) => id,
            None => return String::new(),
        };
        // Get the node's transform to convert local coords → world coords
        let (tx, ty) = self.document.page(self.current_page)
            .and_then(|p| p.tree.get(&vec_id))
            .map(|n| (n.transform.tx, n.transform.ty))
            .unwrap_or((0.0, 0.0));

        let mut parts = Vec::new();
        for a in &self.vector_edit_anchors {
            let hox = a.handle_out.map(|h| h.x).unwrap_or(0.0);
            let hoy = a.handle_out.map(|h| h.y).unwrap_or(0.0);
            let hix = a.handle_in.map(|h| h.x).unwrap_or(0.0);
            let hiy = a.handle_in.map(|h| h.y).unwrap_or(0.0);
            parts.push(format!(
                r#"{{"x":{:.2},"y":{:.2},"hox":{:.2},"hoy":{:.2},"hix":{:.2},"hiy":{:.2}}}"#,
                a.pos.x, a.pos.y, hox, hoy, hix, hiy
            ));
        }
        let sel = self.vector_selected_point.map(|i| i as i32).unwrap_or(-1);
        format!(
            r#"{{"anchors":[{}],"selected":{},"closed":{},"tx":{:.2},"ty":{:.2}}}"#,
            parts.join(","), sel, self.vector_edit_closed, tx, ty
        )
    }

    /// Exit vector editing mode (from JS, e.g. Escape key).
    pub fn vector_edit_exit(&mut self) {
        if self.editing_vector.is_some() {
            self.exit_vector_edit_and_commit();
        }
    }

    /// Returns the current marquee selection rectangle in world coords, or empty if not dragging.
    /// Format: [min_x, min_y, max_x, max_y]. Used by TypeScript to draw the selection overlay.
    pub fn get_marquee_rect(&self) -> Vec<f32> {
        match self.mode {
            InteractionMode::MarqueeSelect { start_wx, start_wy, current_wx, current_wy } => {
                vec![
                    start_wx.min(current_wx), start_wy.min(current_wy),
                    start_wx.max(current_wx), start_wy.max(current_wy),
                ]
            }
            _ => vec![],
        }
    }

    /// Internal: check if world coords (wx, wy) hit a vector anchor or handle.
    /// Returns (anchor_index, None for anchor / Some(HandleType) for handle).
    fn check_vector_point(&self, wx: f32, wy: f32) -> Option<(usize, Option<HandleType>)> {
        let vec_id = self.editing_vector?;
        let node = self.document.page(self.current_page)?.tree.get(&vec_id)?;
        let tx = node.transform.tx;
        let ty = node.transform.ty;

        // Convert world click to local vector coords
        let lx = wx - tx;
        let ly = wy - ty;

        let threshold = 8.0 / self.cam_zoom; // constant screen-space hit area

        // Check anchors first (higher priority than handles)
        for (i, a) in self.vector_edit_anchors.iter().enumerate() {
            let dx = lx - a.pos.x;
            let dy = ly - a.pos.y;
            if dx * dx + dy * dy < threshold * threshold {
                return Some((i, None));
            }
        }

        // Check handles
        for (i, a) in self.vector_edit_anchors.iter().enumerate() {
            if let Some(h) = a.handle_out {
                let hp = a.pos + h;
                let dx = lx - hp.x;
                let dy = ly - hp.y;
                if dx * dx + dy * dy < threshold * threshold {
                    return Some((i, Some(HandleType::Out)));
                }
            }
            if let Some(h) = a.handle_in {
                let hp = a.pos + h;
                let dx = lx - hp.x;
                let dy = ly - hp.y;
                if dx * dx + dy * dy < threshold * threshold {
                    return Some((i, Some(HandleType::In)));
                }
            }
        }

        None
    }

    /// Internal: apply current edit anchors back to the vector node.
    fn apply_vector_edit(&mut self, vector_id: NodeId) {
        let cmds = rebuild_commands(&self.vector_edit_anchors, self.vector_edit_closed);
        let fill_rule = self.vector_edit_orig_paths.first()
            .map(|p| p.fill_rule)
            .unwrap_or(FillRule::NonZero);

        // Compute new bounding box
        let (min, max) = anchors_bbox(&self.vector_edit_anchors);
        let new_w = (max.x - min.x).max(1.0);
        let new_h = (max.y - min.y).max(1.0);

        if let Some(page) = self.document.page_mut(self.current_page) {
            if let Some(node) = page.tree.get_mut(&vector_id) {
                if let NodeKind::Vector { ref mut paths } = node.kind {
                    *paths = vec![VectorPath { commands: cmds, fill_rule }];
                }
                // Update dimensions to match new bounds
                // Note: we keep anchors in local coords relative to node origin,
                // so we don't need to change transform.tx/ty here
                node.width = new_w;
                node.height = new_h;
            }
        }
        self.patch_scene_shape(vector_id); // fast path: update shape in-place
    }

    /// Internal: exit vector editing and commit undo action.
    fn exit_vector_edit_and_commit(&mut self) {
        if let Some(vec_id) = self.editing_vector.take() {
            // Push undo with the original paths
            self.undo_stack.push(UndoAction::EditVector {
                node_id: vec_id,
                paths: self.vector_edit_orig_paths.clone(),
                width: self.vector_edit_orig_w,
                height: self.vector_edit_orig_h,
                tx: self.vector_edit_orig_tx,
                ty: self.vector_edit_orig_ty,
            });
            self.redo_stack.clear();
        }
        self.vector_selected_point = None;
        self.vector_edit_anchors.clear();
        self.vector_edit_orig_paths.clear();
        self.needs_render = true;
    }

    // ─── Mouse interaction ──────────────────────────────────

    /// Handle mouse down. Coordinates are SCREEN space.
    /// shift=true adds/removes from selection instead of replacing.
    /// Returns true if something was selected.
    pub fn mouse_down(&mut self, sx: f32, sy: f32, shift: bool) -> bool {
        let (wx, wy) = self.screen_to_world(sx, sy);
        let now = js_sys::Date::now();

        // Click-drag shape creation: if pending, start drag
        if let Some(shape_type) = self.pending_creation.take() {
            self.pen_cursor = Vec2::new(wx, wy); // reuse for current pos tracking
            self.mode = InteractionMode::CreatingShape { shape_type, start_wx: wx, start_wy: wy };
            self.selected.clear();
            self.needs_render = true;
            return false;
        }

        // Check rotation zone BEFORE hit testing — rotation zone is OUTSIDE node bounds
        if self.selected.len() == 1 && self.check_rotation_zone(wx, wy) {
            let node_id = self.selected[0];
            if let Some(page) = self.document.page(self.current_page) {
                if let Some(node) = page.tree.get(&node_id) {
                    let (nx, ny) = self.node_world_pos(&node_id);
                    let cx = nx + node.width / 2.0;
                    let cy = ny + node.height / 2.0;
                    let start_angle = (wy - cy).atan2(wx - cx);
                    self.mode = InteractionMode::Rotating {
                        node_id,
                        center_x: cx,
                        center_y: cy,
                        start_angle,
                        orig_transform: node.transform,
                    };
                    return true;
                }
            }
        }

        // Check resize handles BEFORE hit-testing — handles must take priority
        // even when another node is on top at the corner position.
        if !self.selected.is_empty() && !shift {
            if let Some(handle) = self.check_resize_handle(wx, wy) {
                let sel_id = self.selected[0];
                let page = self.document.page(self.current_page).unwrap();
                let node = page.tree.get(&sel_id).unwrap();
                self.mode = InteractionMode::Resizing {
                    node_id: sel_id, handle,
                    start_x: wx, start_y: wy,
                    orig_w: node.width, orig_h: node.height,
                    orig_tx: node.transform.tx, orig_ty: node.transform.ty,
                };
                return true;
            }
        }

        if let Some(node_id) = self.hit_test_scene(wx, wy) {
            // Double-click detection: same node within 400ms → enter group
            let is_double_click = self.last_click_node == Some(node_id)
                && (now - self.last_click_time) < 400.0;
            self.last_click_time = now;
            self.last_click_node = Some(node_id);

            if is_double_click {
                let page = self.document.page(self.current_page).unwrap();
                // Double-click on Vector → enter point editing mode
                if let Some(node) = page.tree.get(&node_id) {
                    if let NodeKind::Vector { ref paths } = node.kind {
                        // Extract anchors and enter edit mode
                        let (anchors, closed) = extract_anchors(paths);
                        if !anchors.is_empty() {
                            self.vector_edit_orig_paths = paths.clone();
                            self.vector_edit_orig_w = node.width;
                            self.vector_edit_orig_h = node.height;
                            self.vector_edit_orig_tx = node.transform.tx;
                            self.vector_edit_orig_ty = node.transform.ty;
                            self.vector_edit_anchors = anchors;
                            self.vector_edit_closed = closed;
                            self.editing_vector = Some(node_id);
                            self.vector_selected_point = None;
                            self.selected = vec![node_id];
                            self.mark_selection_dirty();
                            return true;
                        }
                    }
                }
                // Check if the clicked node is a container (frame/group) with children
                let has_children = page.tree.children_of(&node_id)
                    .map(|c| !c.is_empty())
                    .unwrap_or(false);
                if has_children {
                    // Enter the group — next clicks will select children inside
                    self.entered_group = Some(node_id);
                    self.mark_selection_dirty();
                    return true;
                }
            }

            // If in vector editing mode, check for point/handle hits
            if let Some(vec_id) = self.editing_vector {
                if node_id == vec_id {
                    if let Some((idx, handle_type)) = self.check_vector_point(wx, wy) {
                        let anchor = &self.vector_edit_anchors[idx];
                        let orig = match handle_type {
                            None => anchor.pos,
                            Some(HandleType::Out) => anchor.pos + anchor.handle_out.unwrap_or(Vec2::ZERO),
                            Some(HandleType::In) => anchor.pos + anchor.handle_in.unwrap_or(Vec2::ZERO),
                        };
                        self.vector_selected_point = Some(idx);
                        self.mode = InteractionMode::EditingVector {
                            vector_id: vec_id,
                            point_index: idx,
                            handle_type,
                            start_x: wx,
                            start_y: wy,
                            orig_x: orig.x,
                            orig_y: orig.y,
                        };
                        self.needs_render = true;
                        return true;
                    } else {
                        // Clicked on vector but not on a point → just select the point nearest
                        self.vector_selected_point = None;
                        self.needs_render = true;
                    }
                } else {
                    // Clicked on a different node → exit vector editing
                    self.exit_vector_edit_and_commit();
                }
            }

            let page = self.document.page(self.current_page).unwrap();
            // Note: resize handles are checked BEFORE hit_test (above)

            if shift {
                // Toggle: remove if already selected, add otherwise
                if let Some(pos) = self.selected.iter().position(|id| *id == node_id) {
                    self.selected.remove(pos);
                } else {
                    self.selected.push(node_id);
                }
                self.mode = InteractionMode::Idle;
            } else {
                // If clicking a node that's already selected (multi-select), keep selection.
                // Otherwise replace selection with just this node.
                if !self.selected.contains(&node_id) {
                    self.selected = vec![node_id];
                }
                // Start dragging ALL selected nodes
                let origins: Vec<(NodeId, f32, f32)> = self.selected.iter()
                    .filter_map(|id| page.tree.get(id).map(|n| (*id, n.transform.tx, n.transform.ty)))
                    .collect();
                self.mode = InteractionMode::Dragging {
                    origins,
                    start_x: wx, start_y: wy,
                };
            }
            self.mark_selection_dirty();
            true
        } else {
            self.last_click_time = now;
            self.last_click_node = None;
            // Clicking empty space exits vector editing and group
            if self.editing_vector.is_some() {
                self.exit_vector_edit_and_commit();
            }
            self.entered_group = None;
            if !shift {
                self.selected.clear();
            }
            // Start marquee selection — drag to select multiple nodes
            self.mode = InteractionMode::MarqueeSelect {
                start_wx: wx, start_wy: wy,
                current_wx: wx, current_wy: wy,
            };
            self.mark_selection_dirty();
            false
        }
    }

    /// Handle mouse move (drag/resize). Coordinates are SCREEN space.
    pub fn mouse_move(&mut self, sx: f32, sy: f32) {
        let (x, y) = self.screen_to_world(sx, sy);
        let grid = self.snap_grid;
        let snap = |v: f32| -> f32 {
            if grid > 0.0 { (v / grid).round() * grid } else { v }
        };
        match self.mode {
            InteractionMode::Dragging { ref origins, start_x, start_y } => {
                let dx = x - start_x;
                let dy = y - start_y;
                let origins_clone = origins.clone();
                for &(node_id, orig_tx, orig_ty) in &origins_clone {
                    let new_tx = snap(orig_tx + dx);
                    let new_ty = snap(orig_ty + dy);
                    if let Some(page) = self.document.page_mut(self.current_page) {
                        if let Some(node) = page.tree.get_mut(&node_id) {
                            node.transform.tx = new_tx;
                            node.transform.ty = new_ty;
                        }
                    }
                    self.patch_scene_transform(node_id, new_tx, new_ty, None, None);
                }
                self.needs_render = true;
            }
            InteractionMode::Resizing { node_id, handle, start_x, start_y, orig_w, orig_h, orig_tx, orig_ty } => {
                let dx = x - start_x;
                let dy = y - start_y;
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        match handle {
                            ResizeHandle::BottomRight => {
                                node.width = (orig_w + dx).max(1.0);
                                node.height = (orig_h + dy).max(1.0);
                            }
                            ResizeHandle::Right => {
                                node.width = (orig_w + dx).max(1.0);
                            }
                            ResizeHandle::Bottom => {
                                node.height = (orig_h + dy).max(1.0);
                            }
                            ResizeHandle::TopRight => {
                                node.width = (orig_w + dx).max(1.0);
                                node.height = (orig_h - dy).max(1.0);
                                node.transform.ty = orig_ty + dy;
                            }
                            ResizeHandle::BottomLeft => {
                                node.width = (orig_w - dx).max(1.0);
                                node.height = (orig_h + dy).max(1.0);
                                node.transform.tx = orig_tx + dx;
                            }
                            ResizeHandle::TopLeft => {
                                node.width = (orig_w - dx).max(1.0);
                                node.height = (orig_h - dy).max(1.0);
                                node.transform.tx = orig_tx + dx;
                                node.transform.ty = orig_ty + dy;
                            }
                            ResizeHandle::Top => {
                                node.height = (orig_h - dy).max(1.0);
                                node.transform.ty = orig_ty + dy;
                            }
                            ResizeHandle::Left => {
                                node.width = (orig_w - dx).max(1.0);
                                node.transform.tx = orig_tx + dx;
                            }
                        }
                        // Snap position and size to grid
                        if grid > 0.0 {
                            node.transform.tx = snap(node.transform.tx);
                            node.transform.ty = snap(node.transform.ty);
                            node.width = snap(node.width).max(1.0);
                            node.height = snap(node.height).max(1.0);
                        }
                        let tx = node.transform.tx;
                        let ty = node.transform.ty;
                        let w = node.width;
                        let h = node.height;
                        // Patch cached scene items in-place
                        self.patch_scene_transform(node_id, tx, ty, Some(w), Some(h));
                    }
                }
                self.needs_render = true;
            }
            InteractionMode::Rotating { node_id, center_x, center_y, start_angle, ref orig_transform } => {
                let current_angle = (y - center_y).atan2(x - center_x);
                let delta_angle = current_angle - start_angle;
                // Build new transform: translate to origin, rotate, translate back
                let orig_t = orig_transform.clone();
                let new_transform = {
                    let (s, c) = delta_angle.sin_cos();
                    Transform {
                        a: orig_t.a * c - orig_t.b * s,
                        b: orig_t.a * s + orig_t.b * c,
                        c: orig_t.c * c - orig_t.d * s,
                        d: orig_t.c * s + orig_t.d * c,
                        tx: orig_t.tx,
                        ty: orig_t.ty,
                    }
                };
                let (w, h) = if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.transform = new_transform;
                        (node.width, node.height)
                    } else { (0.0, 0.0) }
                } else { (0.0, 0.0) };
                self.patch_scene_full_transform(node_id, new_transform, w, h);
                self.needs_render = true;
            }
            InteractionMode::EditingVector { vector_id, point_index, handle_type, start_x, start_y, orig_x, orig_y } => {
                let dx = x - start_x;
                let dy = y - start_y;
                let new_x = orig_x + dx;
                let new_y = orig_y + dy;

                // Update the anchor in our edit buffer
                if point_index < self.vector_edit_anchors.len() {
                    let anchor = &mut self.vector_edit_anchors[point_index];
                    match handle_type {
                        None => {
                            // Moving the anchor itself
                            anchor.pos = Vec2::new(new_x, new_y);
                        }
                        Some(HandleType::Out) => {
                            anchor.handle_out = Some(Vec2::new(new_x, new_y) - anchor.pos);
                        }
                        Some(HandleType::In) => {
                            anchor.handle_in = Some(Vec2::new(new_x, new_y) - anchor.pos);
                        }
                    }

                    // Rebuild commands and update the node
                    self.apply_vector_edit(vector_id);
                    self.needs_render = true;
                }
            }
            InteractionMode::MarqueeSelect { start_wx, start_wy, ref mut current_wx, ref mut current_wy } => {
                *current_wx = x;
                *current_wy = y;
                // Live selection via spatial grid for O(visible) instead of O(all nodes)
                let min_x = start_wx.min(x);
                let min_y = start_wy.min(y);
                let max_x = start_wx.max(x);
                let max_y = start_wy.max(y);
                self.selected.clear();
                // Use spatial grid if available (fast path for 100K+ artboards)
                if !self.spatial_grid.is_empty() {
                    let cell = self.spatial_grid_cell_size;
                    let col_min = (min_x / cell).floor() as i32;
                    let col_max = (max_x / cell).ceil() as i32;
                    let row_min = (min_y / cell).floor() as i32;
                    let row_max = (max_y / cell).ceil() as i32;
                    let mut seen = std::collections::HashSet::new();
                    if let Some(items) = self.scene_cache.as_ref() {
                        for col in col_min..=col_max {
                            for row in row_min..=row_max {
                                if let Some(entries) = self.spatial_grid.get(&(col, row)) {
                                    for &(idx, _end) in entries {
                                        if idx < items.len() {
                                            let item = &items[idx];
                                            let nid = item.node_id;
                                            if seen.contains(&nid) { continue; }
                                            seen.insert(nid);
                                            let b = &item.world_bounds;
                                            if b.min.x < max_x && b.max.x > min_x && b.min.y < max_y && b.max.y > min_y {
                                                self.selected.push(nid);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if let Some(page) = self.document.page(self.current_page) {
                    // Fallback: brute-force for small documents without spatial grid
                    if let Some(children) = page.tree.children_of(&page.tree.root_id()) {
                        for child_id in children.iter() {
                            if let Some(node) = page.tree.get(child_id) {
                                let (nwx, nwy) = self.node_world_pos(child_id);
                                let nw = node.width;
                                let nh = node.height;
                                if nwx < max_x && nwx + nw > min_x && nwy < max_y && nwy + nh > min_y {
                                    self.selected.push(*child_id);
                                }
                            }
                        }
                    }
                }
                self.needs_render = true;
            }
            InteractionMode::CreatingShape { .. } => {
                // Update preview position (reuse pen_cursor for tracking)
                self.pen_cursor = Vec2::new(x, y);
                self.needs_render = true;
            }
            InteractionMode::Idle => {}
        }
    }

    /// Handle explicit double-click from browser dblclick event.
    /// Enters group or vector editing mode for the node under cursor.
    /// This avoids timing-based double-click detection which can fail
    /// when the browser event loop adds latency between mousedown events.
    pub fn handle_double_click(&mut self, sx: f32, sy: f32) -> bool {
        let (wx, wy) = self.screen_to_world(sx, sy);
        let Some(node_id) = self.hit_test_scene(wx, wy) else {
            return false;
        };

        let page = self.document.page(self.current_page).unwrap();

        // Double-click on Vector → enter point editing mode
        if let Some(node) = page.tree.get(&node_id) {
            if let NodeKind::Vector { ref paths } = node.kind {
                let (anchors, closed) = extract_anchors(paths);
                if !anchors.is_empty() {
                    self.vector_edit_orig_paths = paths.clone();
                    self.vector_edit_orig_w = node.width;
                    self.vector_edit_orig_h = node.height;
                    self.vector_edit_orig_tx = node.transform.tx;
                    self.vector_edit_orig_ty = node.transform.ty;
                    self.vector_edit_anchors = anchors;
                    self.vector_edit_closed = closed;
                    self.editing_vector = Some(node_id);
                    self.vector_selected_point = None;
                    self.selected = vec![node_id];
                    self.mark_selection_dirty();
                    return true;
                }
            }
        }

        // Double-click on container → enter group
        let has_children = page.tree.children_of(&node_id)
            .map(|c| !c.is_empty())
            .unwrap_or(false);
        if has_children {
            self.entered_group = Some(node_id);
            self.mark_selection_dirty();
            return true;
        }

        false
    }

    /// Handle mouse up. Emits CRDT ops for any drag/resize that happened.
    pub fn mouse_up(&mut self) {
        match self.mode {
            InteractionMode::Dragging { ref origins, .. } => {
                // Push undo for each dragged node with its original position
                for &(node_id, orig_tx, orig_ty) in origins {
                    self.undo_stack.push(UndoAction::MoveNode { node_id, tx: orig_tx, ty: orig_ty });
                }
                self.redo_stack.clear();
                // Capture current values for CRDT ops
                for &(node_id, _, _) in origins {
                    let transform = self.document.page(self.current_page)
                        .and_then(|p| p.tree.get(&node_id))
                        .map(|n| n.transform);
                    if let Some(t) = transform {
                        let op_id = self.document.clock.next_op_id();
                        self.pending_ops.push(Operation {
                            id: op_id,
                            kind: OpKind::SetProperty {
                                node_id,
                                property: rendero_crdt::operation::PropertyUpdate::Transform(t),
                            },
                        });
                    }
                }
            }
            InteractionMode::Resizing { node_id, orig_w, orig_h, orig_tx, orig_ty, .. } => {
                // Push undo with original dimensions and position
                self.undo_stack.push(UndoAction::ResizeNode { node_id, tx: orig_tx, ty: orig_ty, w: orig_w, h: orig_h });
                self.redo_stack.clear();
                // Capture current values for CRDT ops
                let props = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(&node_id))
                    .map(|n| (n.width, n.height, n.transform));
                if let Some((w, h, t)) = props {
                    // Apply constraints to children when parent frame is resized
                    self.apply_constraints(node_id, orig_w, orig_h, w, h);
                    // Structural: constraints move multiple children — would need
                    // patch_scene_transform for each child. TODO: optimize for frames
                    // with few children by patching each one incrementally.
                    self.mark_dirty();

                    let id1 = self.document.clock.next_op_id();
                    self.pending_ops.push(Operation {
                        id: id1,
                        kind: OpKind::SetProperty {
                            node_id,
                            property: rendero_crdt::operation::PropertyUpdate::Width(w),
                        },
                    });
                    let id2 = self.document.clock.next_op_id();
                    self.pending_ops.push(Operation {
                        id: id2,
                        kind: OpKind::SetProperty {
                            node_id,
                            property: rendero_crdt::operation::PropertyUpdate::Height(h),
                        },
                    });
                    let id3 = self.document.clock.next_op_id();
                    self.pending_ops.push(Operation {
                        id: id3,
                        kind: OpKind::SetProperty {
                            node_id,
                            property: rendero_crdt::operation::PropertyUpdate::Transform(t),
                        },
                    });
                }
            }
            InteractionMode::Rotating { node_id, ref orig_transform, .. } => {
                self.undo_stack.push(UndoAction::RotateNode { node_id, transform: orig_transform.clone() });
                self.redo_stack.clear();
                // CRDT op for transform
                let t = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(&node_id))
                    .map(|n| n.transform);
                if let Some(t) = t {
                    let op_id = self.document.clock.next_op_id();
                    self.pending_ops.push(Operation {
                        id: op_id,
                        kind: OpKind::SetProperty {
                            node_id,
                            property: rendero_crdt::operation::PropertyUpdate::Transform(t),
                        },
                    });
                }
            }
            InteractionMode::EditingVector { .. } => {
                // Point drag completed — undo snapshot was already saved at edit start
                // Nothing extra needed here, mode resets below
            }
            InteractionMode::MarqueeSelect { .. } => {
                // Selection was already updated live during mouse_move.
                // Just finalize by switching to Idle.
                self.mark_selection_dirty();
            }
            InteractionMode::CreatingShape { shape_type, start_wx, start_wy } => {
                let end = self.pen_cursor;
                let x = start_wx.min(end.x);
                let y = start_wy.min(end.y);
                let w = (end.x - start_wx).abs().max(2.0);
                let h = (end.y - start_wy).abs().max(2.0);
                // Create the shape at the dragged rectangle and select it
                let id_parts = match shape_type {
                    ShapeCreationType::Rectangle => self.add_rectangle("Rectangle", x, y, w, h, 0.75, 0.75, 0.75, 1.0),
                    ShapeCreationType::Ellipse => self.add_ellipse("Ellipse", x, y, w, h, 0.75, 0.75, 0.75, 1.0),
                    ShapeCreationType::Frame => self.add_frame("Frame", x, y, w, h, 1.0, 1.0, 1.0, 1.0),
                    ShapeCreationType::Star => self.add_star("Star", x, y, w, h, 0.75, 0.75, 0.75, 1.0, 5, 0.5),
                    ShapeCreationType::Text => self.add_text("Text", "Text", x, y, 16.0, 1.0, 1.0, 1.0, 1.0),
                };
                if id_parts.len() == 2 {
                    let node_id = NodeId::new(id_parts[0] as u64, id_parts[1]);
                    self.selected = vec![node_id];
                    self.mark_selection_dirty();
                }
            }
            InteractionMode::Idle => {}
        }
        self.mode = InteractionMode::Idle;
    }

    /// Delete all selected nodes.
    pub fn delete_selected(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let ids: Vec<NodeId> = self.selected.drain(..).collect();
        let mut deleted = false;
        for node_id in ids {
            let parent_id = self.document.page(self.current_page)
                .and_then(|p| p.tree.parent_of(&node_id));
            let node_clone = self.document.page(self.current_page)
                .and_then(|p| p.tree.get(&node_id).cloned());

            let op_id = self.document.clock.next_op_id();
            let op = Operation {
                id: op_id,
                kind: OpKind::DeleteNode { node_id },
            };
            self.pending_ops.push(op);

            if let Some(page) = self.document.page_mut(self.current_page) {
                let _ = page.tree.remove(&node_id);

                if let (Some(node), Some(pid)) = (node_clone, parent_id) {
                    self.undo_stack.push(UndoAction::RemoveNode { node, parent_id: pid });
                    self.redo_stack.clear();
                }
                // Incremental scene update
                self.scene_remove_leaf(node_id);
                deleted = true;
            }
        }
        if deleted {
            self.needs_render = true;
        }
        deleted
    }

    /// Copy selected nodes to internal clipboard.
    pub fn copy_selected(&mut self) -> u32 {
        self.clipboard.clear();
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return 0,
        };
        for sel_id in &self.selected {
            if let Some(node) = page.tree.get(sel_id) {
                self.clipboard.push(node.clone());
            }
        }
        self.clipboard.len() as u32
    }

    /// Paste clipboard nodes offset by (10,10). Selects the pasted nodes.
    pub fn paste(&mut self) -> u32 {
        if self.clipboard.is_empty() {
            return 0;
        }
        let root_id = match self.document.page(self.current_page) {
            Some(p) => p.tree.root_id(),
            None => return 0,
        };
        let mut new_ids = Vec::new();
        for template in &self.clipboard.clone() {
            let new_id = self.document.clock.next_node_id();
            let mut node = template.clone();
            node.id = new_id;
            node.transform.tx += 10.0;
            node.transform.ty += 10.0;

            let op_id = self.document.clock.next_op_id();
            self.pending_ops.push(Operation {
                id: op_id,
                kind: OpKind::InsertNode {
                    node: node.clone(),
                    parent_id: root_id,
                    position: FractionalIndex::end(),
                },
            });
            let node_for_undo = node.clone();
            let node_for_scene = node.clone();
            self.document.add_node(self.current_page, node, root_id, usize::MAX)
                .expect("paste insert failed");
            self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id: root_id });
            self.scene_insert_leaf(&node_for_scene, root_id);
            new_ids.push(new_id);
        }
        self.redo_stack.clear();
        self.selected = new_ids;
        self.needs_render = true;
        self.selected.len() as u32
    }

    /// Duplicate selected nodes in-place (copy + paste in one step).
    pub fn duplicate_selected(&mut self) -> u32 {
        self.copy_selected();
        self.paste()
    }

    /// Combine selected nodes with a boolean operation.
    /// Creates a BooleanOp parent, moves selected nodes under it.
    /// op: 0=Union, 1=Subtract, 2=Intersect, 3=Exclude
    pub fn boolean_op(&mut self, op: u32) -> bool {
        if self.selected.len() < 2 {
            return false;
        }
        let operation = match op {
            0 => BooleanOperation::Union,
            1 => BooleanOperation::Subtract,
            2 => BooleanOperation::Intersect,
            3 => BooleanOperation::Exclude,
            _ => return false,
        };

        let root_id = match self.document.page(self.current_page) {
            Some(p) => p.tree.root_id(),
            None => return false,
        };

        // Create the boolean op node
        let bool_id = self.document.clock.next_node_id();
        let mut bool_node = Node::frame(bool_id, "Boolean Group", 0.0, 0.0);
        bool_node.kind = NodeKind::BooleanOp { operation };

        // Calculate bounding box of selected nodes
        let page = self.document.page(self.current_page).unwrap();
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for sel_id in &self.selected {
            if let Some(node) = page.tree.get(sel_id) {
                min_x = min_x.min(node.transform.tx);
                min_y = min_y.min(node.transform.ty);
                max_x = max_x.max(node.transform.tx + node.width);
                max_y = max_y.max(node.transform.ty + node.height);
            }
        }
        bool_node.transform = Transform::translate(min_x, min_y);
        bool_node.width = max_x - min_x;
        bool_node.height = max_y - min_y;

        // Copy fills from first selected node
        if let Some(first) = self.selected.first() {
            if let Some(node) = page.tree.get(first) {
                bool_node.style.fills = node.style.fills.clone();
            }
        }

        // Insert boolean node, then reparent selected nodes under it
        let ids_to_move: Vec<NodeId> = self.selected.clone();
        self.document.add_node(self.current_page, bool_node, root_id, usize::MAX)
            .expect("insert boolean node failed");

        if let Some(page) = self.document.page_mut(self.current_page) {
            for (i, sel_id) in ids_to_move.iter().enumerate() {
                // Adjust child transforms to be relative to boolean node
                if let Some(node) = page.tree.get_mut(sel_id) {
                    node.transform.tx -= min_x;
                    node.transform.ty -= min_y;
                }
                let _ = page.tree.move_node(*sel_id, bool_id, i);
            }
        }

        self.selected = vec![bool_id];
        // Structural: nodes reparented under new boolean result node.
        self.mark_dirty();
        true
    }

    /// Flatten selected node to a vector path (Cmd+E).
    /// Converts rectangles, ellipses, polygons, etc. to their path representation.
    /// Returns true on success.
    pub fn flatten_selected(&mut self) -> bool {
        if self.selected.len() != 1 {
            return false;
        }
        let node_id = self.selected[0];

        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return false,
        };

        let node = match page.tree.get(&node_id) {
            Some(n) => n,
            None => return false,
        };

        // Already a vector — nothing to do
        if matches!(node.kind, NodeKind::Vector { .. }) {
            return true;
        }

        // Text and Image can't be flattened to paths
        if matches!(node.kind, NodeKind::Text { .. } | NodeKind::Image { .. }) {
            return false;
        }

        let commands = rendero_core::boolean::node_to_path_commands(node);
        if commands.is_empty() {
            return false;
        }

        let path = VectorPath {
            commands,
            fill_rule: FillRule::NonZero,
        };

        let page = self.document.page_mut(self.current_page).unwrap();
        let node = page.tree.get_mut(&node_id).unwrap();
        node.kind = NodeKind::Vector { paths: vec![path] };

        // Structural: node kind changed to Vector.
        self.mark_dirty();
        true
    }

    /// Group selected nodes into a Frame.
    pub fn group_selected(&mut self) -> bool {
        if self.selected.len() < 2 {
            return false;
        }

        let root_id = match self.document.page(self.current_page) {
            Some(p) => p.tree.root_id(),
            None => return false,
        };

        // Calculate bounding box
        let page = self.document.page(self.current_page).unwrap();
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for sel_id in &self.selected {
            if let Some(node) = page.tree.get(sel_id) {
                min_x = min_x.min(node.transform.tx);
                min_y = min_y.min(node.transform.ty);
                max_x = max_x.max(node.transform.tx + node.width);
                max_y = max_y.max(node.transform.ty + node.height);
            }
        }

        let group_id = self.document.clock.next_node_id();
        let mut group = Node::frame(group_id, "Group", max_x - min_x, max_y - min_y);
        group.transform = Transform::translate(min_x, min_y);
        // Groups don't clip their content (unlike artboard frames)
        if let NodeKind::Frame { ref mut clip_content, .. } = group.kind {
            *clip_content = false;
        }

        let ids_to_move: Vec<NodeId> = self.selected.clone();
        self.document.add_node(self.current_page, group, root_id, usize::MAX)
            .expect("insert group failed");

        if let Some(page) = self.document.page_mut(self.current_page) {
            for (i, sel_id) in ids_to_move.iter().enumerate() {
                if let Some(node) = page.tree.get_mut(sel_id) {
                    node.transform.tx -= min_x;
                    node.transform.ty -= min_y;
                }
                let _ = page.tree.move_node(*sel_id, group_id, i);
            }
        }

        self.selected = vec![group_id];
        // Structural: nodes reparented under new group frame.
        self.mark_dirty();
        true
    }

    /// Ungroup: move children of selected group to its parent, remove the group.
    pub fn ungroup_selected(&mut self) -> bool {
        if self.selected.len() != 1 {
            return false;
        }
        let group_id = self.selected[0];

        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return false,
        };

        // Get group's parent and children
        let parent_id = match page.tree.parent_of(&group_id) {
            Some(p) => p,
            None => return false,
        };
        let group_node = match page.tree.get(&group_id) {
            Some(n) => n,
            None => return false,
        };
        let group_tx = group_node.transform.tx;
        let group_ty = group_node.transform.ty;
        let child_ids: Vec<NodeId> = page.tree.children_of(&group_id)
            .map(|cl| cl.iter().copied().collect())
            .unwrap_or_default();

        if child_ids.is_empty() {
            return false;
        }

        // Move children to parent, adjusting transforms
        let page = self.document.page_mut(self.current_page).unwrap();
        for child_id in &child_ids {
            if let Some(node) = page.tree.get_mut(child_id) {
                node.transform.tx += group_tx;
                node.transform.ty += group_ty;
            }
            let _ = page.tree.move_node(*child_id, parent_id, usize::MAX);
        }
        // Remove the empty group
        let _ = page.tree.remove(&group_id);

        self.selected = child_ids;
        // Structural: children reparented to grandparent, group node removed.
        self.mark_dirty();
        true
    }

    /// Create a component from selected nodes (wraps them like group, but NodeKind::Component).
    /// Returns component node ID as [counter, client_id], or empty on failure.
    pub fn create_component(&mut self) -> Vec<u32> {
        if self.selected.is_empty() {
            return vec![];
        }

        let root_id = match self.document.page(self.current_page) {
            Some(p) => p.tree.root_id(),
            None => return vec![],
        };

        // Calculate bounding box of selected nodes
        let page = self.document.page(self.current_page).unwrap();
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for sel_id in &self.selected {
            if let Some(node) = page.tree.get(sel_id) {
                min_x = min_x.min(node.transform.tx);
                min_y = min_y.min(node.transform.ty);
                max_x = max_x.max(node.transform.tx + node.width);
                max_y = max_y.max(node.transform.ty + node.height);
            }
        }

        let comp_id = self.document.clock.next_node_id();
        let mut comp = Node::component(comp_id, "Component", max_x - min_x, max_y - min_y);
        comp.transform = Transform::translate(min_x, min_y);

        let ids_to_move: Vec<NodeId> = self.selected.clone();
        self.document.add_node(self.current_page, comp, root_id, usize::MAX)
            .expect("insert component failed");

        if let Some(page) = self.document.page_mut(self.current_page) {
            for (i, sel_id) in ids_to_move.iter().enumerate() {
                if let Some(node) = page.tree.get_mut(sel_id) {
                    node.transform.tx -= min_x;
                    node.transform.ty -= min_y;
                }
                let _ = page.tree.move_node(*sel_id, comp_id, i);
            }
        }

        self.selected = vec![comp_id];
        // Structural: nodes reparented under new component node.
        self.mark_dirty();
        vec![comp_id.0.counter as u32, comp_id.0.client_id]
    }

    /// Create an instance of a component. Deep-clones the component's children.
    /// Returns instance node ID as [counter, client_id], or empty on failure.
    pub fn create_instance(&mut self, comp_counter: u32, comp_client_id: u32) -> Vec<u32> {
        let comp_id = NodeId(rendero_core::id::LogicalClock {
            counter: comp_counter as u64,
            client_id: comp_client_id,
        });

        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return vec![],
        };

        let comp_node = match page.tree.get(&comp_id) {
            Some(n) => n,
            None => return vec![],
        };

        // Verify it's actually a component
        if !matches!(comp_node.kind, NodeKind::Component) {
            return vec![];
        }

        let comp_w = comp_node.width;
        let comp_h = comp_node.height;
        let comp_tx = comp_node.transform.tx;
        let comp_ty = comp_node.transform.ty;

        // Collect children to deep-clone (gather data while we have immutable borrow)
        let children_data = self.collect_subtree_data(comp_id);

        let root_id = self.document.page(self.current_page).unwrap().tree.root_id();

        // Create instance node, offset to the right of the component
        let inst_id = self.document.clock.next_node_id();
        let mut inst = Node::instance(inst_id, "Instance", comp_id, comp_w, comp_h);
        inst.transform = Transform::translate(comp_tx + comp_w + 20.0, comp_ty);

        self.document.add_node(self.current_page, inst, root_id, usize::MAX)
            .expect("insert instance failed");

        // Deep-clone children into the instance
        self.clone_children_into(inst_id, &children_data);

        self.selected = vec![inst_id];
        // Structural: new instance node with cloned children added.
        self.mark_dirty();
        vec![inst_id.0.counter as u32, inst_id.0.client_id]
    }

    /// Collect subtree data for deep cloning (node + parent relationship).
    fn collect_subtree_data(&self, root_id: NodeId) -> Vec<(Node, NodeId)> {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return vec![],
        };

        let mut result = Vec::new();
        let mut stack: Vec<NodeId> = Vec::new();

        // Start with root's children
        if let Some(children) = page.tree.children_of(&root_id) {
            for child_id in children.iter() {
                stack.push(*child_id);
            }
        }

        // BFS: collect each node with its parent
        let mut queue_idx = 0;
        while queue_idx < stack.len() {
            let nid = stack[queue_idx];
            queue_idx += 1;

            if let Some(node) = page.tree.get(&nid) {
                let parent = page.tree.parent_of(&nid).unwrap_or(root_id);
                result.push((node.clone(), parent));

                if let Some(children) = page.tree.children_of(&nid) {
                    for child_id in children.iter() {
                        stack.push(*child_id);
                    }
                }
            }
        }

        result
    }

    /// Clone collected subtree data into a new parent, remapping IDs.
    fn clone_children_into(&mut self, new_parent_id: NodeId, data: &[(Node, NodeId)]) {
        use std::collections::HashMap;
        let mut id_map: HashMap<NodeId, NodeId> = HashMap::new();

        // First pass: assign new IDs and remap parent references
        for (node, _old_parent) in data {
            let new_id = self.document.clock.next_node_id();
            id_map.insert(node.id, new_id);
        }

        // Second pass: insert nodes with new IDs under remapped parents
        for (node, old_parent) in data {
            let new_id = *id_map.get(&node.id).unwrap();
            let mapped_parent = id_map.get(old_parent).copied().unwrap_or(new_parent_id);

            let mut cloned = node.clone();
            cloned.id = new_id;

            // If this is an instance, update component_id if it was remapped
            if let NodeKind::Instance { ref mut component_id, .. } = cloned.kind {
                if let Some(mapped) = id_map.get(component_id) {
                    *component_id = *mapped;
                }
            }

            self.document.add_node(self.current_page, cloned, mapped_parent, usize::MAX)
                .expect("clone child failed");
        }
    }

    /// Detach an instance: convert it to a plain Frame, keeping its children.
    /// Returns true on success.
    pub fn detach_instance(&mut self) -> bool {
        if self.selected.len() != 1 {
            return false;
        }
        let inst_id = self.selected[0];

        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };

        let node = match page.tree.get_mut(&inst_id) {
            Some(n) => n,
            None => return false,
        };

        // Must be an instance
        if !matches!(node.kind, NodeKind::Instance { .. }) {
            return false;
        }

        // Convert to Frame
        node.kind = NodeKind::Frame {
            clip_content: true,
            auto_layout: None,
            corner_radii: rendero_core::node::CornerRadii::default(),
        };

        // Structural: node kind changed from Instance to Frame.
        self.mark_dirty();
        true
    }

    /// Exit the currently entered group. Selects the group itself.
    pub fn exit_group(&mut self) {
        if let Some(gid) = self.entered_group.take() {
            self.selected = vec![gid];
            self.mark_selection_dirty();
        }
    }

    /// Returns the entered group's counter and client_id, or (-1, -1) if none.
    pub fn get_entered_group(&self) -> Vec<i64> {
        match self.entered_group {
            Some(id) => vec![id.0.counter as i64, id.0.client_id as i64],
            None => vec![-1, -1],
        }
    }

    /// Select all direct children of the current page root.
    pub fn select_all(&mut self) {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return,
        };
        let root_id = page.tree.root_id();
        self.selected = match page.tree.children_of(&root_id) {
            Some(children) => children.iter().cloned().collect(),
            None => Vec::new(),
        };
        self.mark_selection_dirty();
    }

    /// Bring selected nodes to front (top of z-order within their parent).
    pub fn bring_to_front(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        for sel_id in self.selected.clone() {
            if let Some(parent_id) = page.tree.parent_of(&sel_id) {
                let len = page.tree.children_of(&parent_id).map(|c| c.len()).unwrap_or(0);
                let _ = page.tree.move_node(sel_id, parent_id, len);
            }
        }
        // Structural: z-order changed (move_node changes child index).
        self.mark_dirty();
        true
    }

    /// Send selected nodes to back (bottom of z-order within their parent).
    pub fn send_to_back(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        for sel_id in self.selected.clone().iter().rev() {
            if let Some(parent_id) = page.tree.parent_of(sel_id) {
                let _ = page.tree.move_node(*sel_id, parent_id, 0);
            }
        }
        // Structural: z-order changed (move_node changes child index).
        self.mark_dirty();
        true
    }

    /// Bring selected nodes forward one step in z-order.
    pub fn bring_forward(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let mut moved = false;
        for sel_id in self.selected.clone() {
            if let Some(parent_id) = page.tree.parent_of(&sel_id) {
                if let Some(children) = page.tree.children_of(&parent_id) {
                    let len = children.len();
                    if let Some(idx) = children.iter().position(|c| *c == sel_id) {
                        if idx + 1 < len {
                            let _ = page.tree.move_node(sel_id, parent_id, idx + 2);
                            moved = true;
                        }
                    }
                }
            }
        }
        if moved { self.mark_dirty(); }
        moved
    }

    /// Send selected nodes backward one step in z-order.
    pub fn send_backward(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let mut moved = false;
        for sel_id in self.selected.clone().iter().rev() {
            if let Some(parent_id) = page.tree.parent_of(sel_id) {
                if let Some(children) = page.tree.children_of(&parent_id) {
                    if let Some(idx) = children.iter().position(|c| c == sel_id) {
                        if idx > 0 {
                            let _ = page.tree.move_node(*sel_id, parent_id, idx - 1);
                            moved = true;
                        }
                    }
                }
            }
        }
        if moved { self.mark_dirty(); }
        moved
    }

    /// Align selected nodes. direction: 0=left, 1=center-h, 2=right, 3=top, 4=center-v, 5=bottom
    pub fn align_selected(&mut self, direction: u32) -> bool {
        if self.selected.len() < 2 {
            return false;
        }

        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return false,
        };

        // Gather bounds
        let mut bounds: Vec<(NodeId, f32, f32, f32, f32)> = Vec::new();
        for sel_id in &self.selected {
            if let Some(node) = page.tree.get(sel_id) {
                bounds.push((*sel_id, node.transform.tx, node.transform.ty, node.width, node.height));
            }
        }

        let target = match direction {
            0 => bounds.iter().map(|b| b.1).fold(f32::INFINITY, f32::min), // left
            1 => { // center-h
                let min_x = bounds.iter().map(|b| b.1).fold(f32::INFINITY, f32::min);
                let max_x = bounds.iter().map(|b| b.1 + b.3).fold(f32::NEG_INFINITY, f32::max);
                (min_x + max_x) / 2.0
            }
            2 => bounds.iter().map(|b| b.1 + b.3).fold(f32::NEG_INFINITY, f32::max), // right
            3 => bounds.iter().map(|b| b.2).fold(f32::INFINITY, f32::min), // top
            4 => { // center-v
                let min_y = bounds.iter().map(|b| b.2).fold(f32::INFINITY, f32::min);
                let max_y = bounds.iter().map(|b| b.2 + b.4).fold(f32::NEG_INFINITY, f32::max);
                (min_y + max_y) / 2.0
            }
            5 => bounds.iter().map(|b| b.2 + b.4).fold(f32::NEG_INFINITY, f32::max), // bottom
            _ => return false,
        };

        let page = self.document.page_mut(self.current_page).unwrap();
        for (node_id, tx, ty, w, h) in &bounds {
            if let Some(node) = page.tree.get_mut(node_id) {
                match direction {
                    0 => node.transform.tx = target,
                    1 => node.transform.tx = target - w / 2.0,
                    2 => node.transform.tx = target - w,
                    3 => node.transform.ty = target,
                    4 => node.transform.ty = target - h / 2.0,
                    5 => node.transform.ty = target - h,
                    _ => {}
                }
            }
        }

        // TODO(perf): Could patch_scene_transform per selected node instead.
        // Alignment only moves N selected nodes (typically <10).
        self.mark_dirty();
        true
    }

    /// Distribute selected nodes evenly. direction: 0=horizontal, 1=vertical
    pub fn distribute_selected(&mut self, direction: u32) -> bool {
        if self.selected.len() < 3 {
            return false;
        }

        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return false,
        };

        let mut items: Vec<(NodeId, f32, f32, f32, f32)> = Vec::new();
        for sel_id in &self.selected {
            if let Some(node) = page.tree.get(sel_id) {
                items.push((*sel_id, node.transform.tx, node.transform.ty, node.width, node.height));
            }
        }

        // Sort by position in the distribution direction
        match direction {
            0 => items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap()),
            1 => items.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap()),
            _ => return false,
        }

        let n = items.len();
        let page = self.document.page_mut(self.current_page).unwrap();

        match direction {
            0 => {
                let first_x = items[0].1;
                let last_x = items[n - 1].1;
                let total_w: f32 = items.iter().map(|i| i.3).sum();
                let gap = (last_x + items[n-1].3 - first_x - total_w) / (n as f32 - 1.0);
                let mut x = first_x;
                for (node_id, _, _, w, _) in &items {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        node.transform.tx = x;
                    }
                    x += w + gap;
                }
            }
            1 => {
                let first_y = items[0].2;
                let last_y = items[n - 1].2;
                let total_h: f32 = items.iter().map(|i| i.4).sum();
                let gap = (last_y + items[n-1].4 - first_y - total_h) / (n as f32 - 1.0);
                let mut y = first_y;
                for (node_id, _, _, _, h) in &items {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        node.transform.ty = y;
                    }
                    y += h + gap;
                }
            }
            _ => {}
        }

        // TODO(perf): Could patch_scene_transform per distributed node instead.
        // Distribution only moves N selected nodes (typically <10).
        self.mark_dirty();
        true
    }

    /// Zoom to fit all content on the current page.
    pub fn zoom_to_fit(&mut self) -> bool {
        // Use scene cache for fast bounding box (contiguous array, no HashMap lookups)
        if self.scene_cache.is_none() {
            let page = match self.document.page(self.current_page) {
                Some(p) => p,
                None => return false,
            };
            let root_id = page.tree.root_id();
            let full_viewport = AABB::new(
                Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec2::new(f32::INFINITY, f32::INFINITY),
            );
            self.scene_cache = Some(rendero_renderer::scene::build_scene(&page.tree, &root_id, &full_viewport));
            self.rebuild_scene_node_index();
        }
        let items = self.scene_cache.as_ref().unwrap();
        if items.is_empty() {
            return false;
        }

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        // Only check top-level items (descendant_count > 0 = frames, skip their children)
        let mut i = 0;
        while i < items.len() {
            let item = &items[i];
            min_x = min_x.min(item.world_bounds.min.x);
            min_y = min_y.min(item.world_bounds.min.y);
            max_x = max_x.max(item.world_bounds.max.x);
            max_y = max_y.max(item.world_bounds.max.y);
            if item.descendant_count > 0 {
                i += 1 + item.descendant_count; // Skip children
            } else {
                i += 1;
            }
        }

        let content_w = max_x - min_x;
        let content_h = max_y - min_y;
        if content_w <= 0.0 || content_h <= 0.0 {
            return false;
        }

        let padding = 50.0;
        let vw = self.viewport_width as f32 - padding * 2.0;
        let vh = self.viewport_height as f32 - padding * 2.0;
        let zoom = (vw / content_w).min(vh / content_h).clamp(0.02, 256.0);

        self.cam_zoom = zoom;
        self.cam_x = min_x - padding / zoom;
        self.cam_y = min_y - padding / zoom;
        self.needs_render = true;
        true
    }

    /// Get selected node IDs. Returns flat array: [counter0, client0, counter1, client1, ...].
    pub fn get_selected(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.selected.len() * 2);
        for id in &self.selected {
            out.push(id.0.counter as u32);
            out.push(id.0.client_id);
        }
        out
    }

    // ─── Rendering ──────────────────────────────────────────

    /// Render and return RGBA pixels. Only re-renders if needed.
    /// Build screen-space RenderItems from current page + camera.
    /// Shared between raster and Canvas 2D render paths.
    fn build_screen_items(&mut self, width: u32, height: u32) -> Vec<RenderItem> {
        let page = self.document.page(self.current_page).unwrap();
        let root_id = page.tree.root_id();

        // Build full scene once, cache until document changes (mark_dirty clears cache).
        // No viewport restriction — pan/zoom never triggers rebuild.
        let raw_items = if self.scene_cache.is_none() {
            let full_viewport = AABB::new(
                Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec2::new(f32::INFINITY, f32::INFINITY),
            );
            let items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &full_viewport);
            self.scene_cache = Some(items);
            self.rebuild_scene_node_index();
            self.scene_cache.as_ref().unwrap()
        } else {
            self.scene_cache.as_ref().unwrap()
        };

        // Apply camera transform to cached world-space items → screen-space
        let mut items = Vec::with_capacity(raw_items.len());
        for item in raw_items {
            let mut screen_item = item.clone();
            screen_item.world_transform.tx = (item.world_transform.tx - self.cam_x) * self.cam_zoom;
            screen_item.world_transform.ty = (item.world_transform.ty - self.cam_y) * self.cam_zoom;
            screen_item.world_transform.a = item.world_transform.a * self.cam_zoom;
            screen_item.world_transform.b = item.world_transform.b * self.cam_zoom;
            screen_item.world_transform.c = item.world_transform.c * self.cam_zoom;
            screen_item.world_transform.d = item.world_transform.d * self.cam_zoom;
            screen_item.world_bounds = AABB::new(
                Vec2::new(
                    (item.world_bounds.min.x - self.cam_x) * self.cam_zoom,
                    (item.world_bounds.min.y - self.cam_y) * self.cam_zoom,
                ),
                Vec2::new(
                    (item.world_bounds.max.x - self.cam_x) * self.cam_zoom,
                    (item.world_bounds.max.y - self.cam_y) * self.cam_zoom,
                ),
            );
            items.push(screen_item);
        }
        items
    }

    /// Raster render — returns raw RGBA pixel buffer. Used for PNG export and fallback.
    pub fn render(&mut self, width: u32, height: u32) -> Vec<u8> {
        let items = self.build_screen_items(width, height);
        let screen_viewport = AABB::new(Vec2::ZERO, Vec2::new(width as f32, height as f32));

        let output = pipeline::render_items(&items, screen_viewport);
        let mut pixels = output.to_pixels(width, height);

        // Draw selection overlay
        for sel_id in &self.selected {
            let (wx, wy) = self.node_world_pos(sel_id);
            if let Some(node) = self.document.page(self.current_page).unwrap().tree.get(sel_id) {
                draw_selection_box(&mut pixels, width, height, wx, wy, node.width, node.height, self.cam_x, self.cam_y, self.cam_zoom);
            }
        }

        self.needs_render = false;
        pixels
    }

    // ─── Rendero custom: batch APIs + WebGL2 + point clouds ───

    /// Batch add multiple ellipses in one call. Format: [x, y, w, h, r, g, b, a] × N.
    /// Skips CRDT ops, undo stack, and per-node scene updates for maximum throughput.
    /// Returns the number of ellipses added.
    pub fn add_ellipses_batch(&mut self, data: &[f32]) -> u32 {
        let stride = 8;
        let count = data.len() / stride;
        if count == 0 { return 0; }

        let parent_id = self.effective_parent();
        let shared_name = String::from("b");

        for i in 0..count {
            let base = i * stride;
            let x = data[base];
            let y = data[base + 1];
            let w = data[base + 2];
            let h = data[base + 3];
            let r = data[base + 4];
            let g = data[base + 5];
            let b_col = data[base + 6];
            let a = data[base + 7];

            let id = self.document.next_id();
            let mut node = Node::ellipse(id, shared_name.clone(), w, h);
            node.transform = Transform::translate(x, y);
            node.style.fills.push(Paint::Solid(Color::new(r, g, b_col, a)));

            self.document.add_node(self.current_page, node, parent_id, usize::MAX)
                .expect("batch insert failed");
        }

        self.mark_dirty();
        count as u32
    }

    /// Add a GPU-direct point cloud from packed Float32Array: [x, y, w, h, r, g, b, a] × N.
    /// Point clouds bypass the document tree entirely — data goes straight to GPU.
    /// Returns cloud ID.
    pub fn add_point_cloud(&mut self, gl: &web_sys::WebGl2RenderingContext, data: &[f32]) -> u32 {
        if self.webgl_state.is_none() {
            match webgl::WebGlState::new(gl) {
                Ok(state) => self.webgl_state = Some(state),
                Err(e) => {
                    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(
                        &format!("WebGL init failed: {}", e)
                    ));
                    return u32::MAX;
                }
            }
        }

        let state = self.webgl_state.as_ref().unwrap();
        match webgl::PointCloud::new(gl, state, data.to_vec()) {
            Ok(cloud) => {
                let id = self.point_clouds.len() as u32;
                self.point_clouds.push(cloud);
                self.needs_render = true;
                id
            }
            Err(e) => {
                web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(
                    &format!("PointCloud creation failed: {}", e)
                ));
                u32::MAX
            }
        }
    }

    /// Remove all point clouds and free GPU resources.
    pub fn clear_point_clouds(&mut self, gl: &web_sys::WebGl2RenderingContext) {
        for cloud in &self.point_clouds {
            cloud.delete(gl);
        }
        self.point_clouds.clear();
        self.needs_render = true;
    }

    /// Total number of points across all point clouds.
    pub fn point_cloud_count(&self) -> usize {
        self.point_clouds.iter().map(|c| c.total_points as usize).sum()
    }

    /// WebGL2 instanced render — batches Rects and Ellipses into 2 GPU draw calls.
    /// 10-50x faster than Canvas2D for data-dense scenes (10K+ visible shapes).
    pub fn render_webgl(&mut self, gl: &web_sys::WebGl2RenderingContext, width: u32, height: u32, dpr: f32) {
        let page = self.document.page(self.current_page).unwrap();
        let root_id = page.tree.root_id();

        if self.scene_cache.is_none() {
            let full_viewport = AABB::new(
                Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec2::new(f32::INFINITY, f32::INFINITY),
            );
            let items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &full_viewport);
            self.spatial_grid.clear();
            self.scene_cache = Some(items);
        }

        if self.spatial_grid.is_empty() {
            let items = self.scene_cache.as_ref().unwrap();
            let cell = self.spatial_grid_cell_size;
            if items.len() > 1 {
                let mut idx = 1;
                while idx < items.len() {
                    let item = &items[idx];
                    let end = idx + 1 + item.descendant_count;
                    let b = &item.world_bounds;
                    let col_min = (b.min.x / cell).floor() as i32;
                    let col_max = (b.max.x / cell).floor() as i32;
                    let row_min = (b.min.y / cell).floor() as i32;
                    let row_max = (b.max.y / cell).floor() as i32;
                    for row in row_min..=row_max {
                        for col in col_min..=col_max {
                            self.spatial_grid.entry((col, row))
                                .or_insert_with(Vec::new)
                                .push((idx, end));
                        }
                    }
                    idx = end;
                }
            }
        }

        if self.webgl_state.is_none() {
            match webgl::WebGlState::new(gl) {
                Ok(state) => self.webgl_state = Some(state),
                Err(e) => {
                    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!("WebGL init failed: {}", e)));
                    return;
                }
            }
        }

        let items = self.scene_cache.as_ref().unwrap();
        let state = self.webgl_state.as_ref().unwrap();

        self.last_drawn_count = webgl::render_webgl(
            gl, state, items, &self.spatial_grid, self.spatial_grid_cell_size,
            width as f64, height as f64,
            self.cam_x as f64, self.cam_y as f64, self.cam_zoom as f64,
            dpr as f64,
        );

        if !self.point_clouds.is_empty() {
            let state = self.webgl_state.as_ref().unwrap();
            self.last_drawn_count += webgl::render_point_clouds(
                gl, state, &mut self.point_clouds,
                width as f64, height as f64,
                self.cam_x as f64, self.cam_y as f64, self.cam_zoom as f64,
                dpr as f64,
            );
        }

        self.needs_render = false;
    }

    // ─── End Rendero custom ───

    /// Canvas 2D vector render — draws directly to a browser canvas context.
    /// GPU-accelerated, no pixel buffer transfer.
    /// `dpr` is the device pixel ratio for crisp Retina rendering.
    pub fn render_canvas2d(&mut self, ctx: &CanvasRenderingContext2d, width: u32, height: u32, dpr: f32) {
        let page = self.document.page(self.current_page).unwrap();
        let root_id = page.tree.root_id();

        // Build full scene once, cache until document changes
        if self.scene_cache.is_none() {
            let full_viewport = AABB::new(
                Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec2::new(f32::INFINITY, f32::INFINITY),
            );
            let items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &full_viewport);
            self.spatial_grid.clear();
            self.scene_cache = Some(items);
            self.rebuild_scene_node_index();
        }

        // Rebuild spatial grid if cleared (after incremental scene updates)
        if self.spatial_grid.is_empty() {
            let items = self.scene_cache.as_ref().unwrap();
            let cell = self.spatial_grid_cell_size;
            if items.len() > 1 {
                let mut idx = 1; // skip root (item 0)
                while idx < items.len() {
                    let item = &items[idx];
                    let end = idx + 1 + item.descendant_count;
                    let b = &item.world_bounds;
                    let col_min = (b.min.x / cell).floor() as i32;
                    let col_max = (b.max.x / cell).floor() as i32;
                    let row_min = (b.min.y / cell).floor() as i32;
                    let row_max = (b.max.y / cell).floor() as i32;
                    for row in row_min..=row_max {
                        for col in col_min..=col_max {
                            self.spatial_grid.entry((col, row))
                                .or_insert_with(Vec::new)
                                .push((idx, end));
                        }
                    }
                    idx = end;
                }
            }
        }
        let items = self.scene_cache.as_ref().unwrap();

        self.last_drawn_count = canvas2d::render_items_with_camera(
            ctx, items, &self.spatial_grid, self.spatial_grid_cell_size,
            width as f64, height as f64,
            self.cam_x as f64, self.cam_y as f64, self.cam_zoom as f64,
            dpr as f64,
            &self.text_arc_params,
        );

        // Draw selection overlay via Canvas 2D
        let page = self.document.page(self.current_page).unwrap();
        if !self.selected.is_empty() && self.selected.len() <= 500 {
            // Individual selection boxes for small selections
            for sel_id in &self.selected {
                if let Some(node) = page.tree.get(sel_id) {
                    let (wx, wy) = self.node_world_pos(sel_id);
                    let sx = (wx - self.cam_x) * self.cam_zoom;
                    let sy = (wy - self.cam_y) * self.cam_zoom;
                    let sw = node.width * self.cam_zoom;
                    let sh = node.height * self.cam_zoom;
                    canvas2d::draw_selection(ctx, sx as f64, sy as f64, sw as f64, sh as f64, dpr as f64);
                }
            }
        } else if !self.selected.is_empty() {
            // Mass selection: compute bounding box by skipping descendants (top-level only).
            // Previous code iterated ALL 1.8M items — 34ms. Now skip via descendant_count — <0.1ms.
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut idx = 0;
            while idx < items.len() {
                let item = &items[idx];
                min_x = min_x.min(item.world_bounds.min.x);
                min_y = min_y.min(item.world_bounds.min.y);
                max_x = max_x.max(item.world_bounds.max.x);
                max_y = max_y.max(item.world_bounds.max.y);
                idx += 1 + item.descendant_count;
            }
            let sx = ((min_x - self.cam_x) * self.cam_zoom) as f64;
            let sy = ((min_y - self.cam_y) * self.cam_zoom) as f64;
            let sw = ((max_x - min_x) * self.cam_zoom) as f64;
            let sh = ((max_y - min_y) * self.cam_zoom) as f64;
            canvas2d::draw_selection(ctx, sx, sy, sw, sh, dpr as f64);
        }

        self.needs_render = false;
    }

    /// Export the canvas at 1:1 scale without selection overlay.
    /// Returns raw RGBA pixel data. JS converts to PNG via canvas.
    pub fn export_pixels(&self, width: u32, height: u32) -> Vec<u8> {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return vec![0u8; (width * height * 4) as usize],
        };
        let root_id = page.tree.root_id();
        let viewport = AABB::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(width as f32, height as f32),
        );
        let raw_items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &viewport);
        let output = pipeline::render_items(&raw_items, viewport);
        output.to_pixels(width, height)
    }

    /// Export the current page as SVG string.
    pub fn export_svg(&self, width: u32, height: u32) -> String {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return String::from("<svg></svg>"),
        };
        let root_id = page.tree.root_id();
        // Auto-fit: if width/height are 0, compute bounding box of all content
        let viewport = if width == 0 || height == 0 {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            if let Some(children) = page.tree.children_of(&root_id) {
                for child_id in children.iter() {
                    if let Some(node) = page.tree.get(child_id) {
                        let tx = node.transform.tx;
                        let ty = node.transform.ty;
                        min_x = min_x.min(tx);
                        min_y = min_y.min(ty);
                        max_x = max_x.max(tx + node.width);
                        max_y = max_y.max(ty + node.height);
                    }
                }
            }
            if min_x > max_x { min_x = 0.0; max_x = 100.0; min_y = 0.0; max_y = 100.0; }
            // Add 10px padding
            AABB::new(
                Vec2::new(min_x - 10.0, min_y - 10.0),
                Vec2::new(max_x + 10.0, max_y + 10.0),
            )
        } else {
            AABB::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(width as f32, height as f32),
            )
        };
        rendero_renderer::svg::export_svg(&page.tree, &root_id, viewport)
    }

    /// Import a .fig file's JSON (from fig2json) into the document.
    /// Returns JSON: {"pages": N, "nodes": N, "errors": [...]}
    pub fn import_fig_json(&mut self, json_str: &str, image_base: &str) -> String {
        let result = fig_import::import_fig_json(&mut self.document, json_str, image_base);
        if result.has_image_fills { self.has_image_fills = true; }
        // Switch to first imported page (skip the default "Page 1")
        if result.pages_imported > 0 && self.document.pages.len() > 1 {
            self.current_page = 1; // first imported page
        }
        // Structural: bulk JSON import adds entire document tree.
        self.mark_dirty();
        format!(
            "{{\"pages\":{},\"nodes\":{},\"errors\":{}}}",
            result.pages_imported,
            result.nodes_imported,
            serde_json::to_string(&result.errors).unwrap_or_else(|_| "[]".into()),
        )
    }

    /// Import a .fig binary directly. No external tools needed.
    /// Returns JSON: {"pages":N,"nodes":N,"images":[path,...],"errors":[...]}
    pub fn import_fig_binary(&mut self, bytes: &[u8]) -> String {
        let fig_result = match fig_import_crate::convert_fig(bytes) {
            Ok(r) => r,
            Err(e) => {
                return format!("{{\"pages\":0,\"nodes\":0,\"images\":[],\"errors\":[\"{}\"]}}", e);
            }
        };

        // Import directly from the Value tree — no JSON string round-trip (avoids OOM on large files)
        let result = fig_import::import_fig_value(&mut self.document, &fig_result.document, "");
        if result.has_image_fills { self.has_image_fills = true; }

        // Switch to first imported page
        if result.pages_imported > 0 && self.document.pages.len() > 1 {
            self.current_page = 1;
        }
        // Structural: bulk import added entire tree of nodes.
        self.mark_dirty();

        // Store image bytes for JS retrieval and return paths
        let image_paths: Vec<String> = fig_result.images.iter()
            .map(|(path, _)| format!("\"{}\"", path))
            .collect();
        for (path, bytes) in fig_result.images {
            // Store under multiple keys so fig_import.rs can find images regardless of extension.
            // ZIP has "images/hash.jpg" or "images/hash.png", but transform.rs produces
            // "images/hash" (no ext), and fig_import.rs appends ".png" for extensionless paths.
            // So we store under: original, extensionless, .png-suffixed, and .jpg-suffixed.
            if let Some(stem) = path.strip_suffix(".png").or_else(|| path.strip_suffix(".jpg")) {
                self.imported_images.insert(stem.to_string(), bytes.clone());
                self.imported_images.insert(format!("{}.png", stem), bytes.clone());
                self.imported_images.insert(format!("{}.jpg", stem), bytes.clone());
            } else if !path.contains('.') {
                self.imported_images.insert(format!("{}.png", path), bytes.clone());
                self.imported_images.insert(format!("{}.jpg", path), bytes.clone());
            }
            self.imported_images.insert(path, bytes);
        }

        format!(
            "{{\"pages\":{},\"nodes\":{},\"images\":[{}],\"errors\":{}}}",
            result.pages_imported,
            result.nodes_imported,
            image_paths.join(","),
            serde_json::to_string(&result.errors).unwrap_or_else(|_| "[]".into()),
        )
    }

    /// Get image bytes extracted from a .fig ZIP by path.
    /// Returns the raw PNG/JPEG bytes, or empty vec if not found.
    pub fn get_imported_image(&self, path: &str) -> Vec<u8> {
        self.imported_images.get(path).cloned().unwrap_or_default()
    }

    /// Import a single page from fig JSON (for large files).
    /// JS should parse the full JSON, extract each page object, and stringify it individually.
    pub fn import_fig_page_json(&mut self, page_json: &str, image_base: &str) -> String {
        let result = fig_import::import_fig_page_json(&mut self.document, page_json, image_base);
        if result.has_image_fills { self.has_image_fills = true; }
        // Structural: bulk page import adds an entire page subtree.
        self.mark_dirty();
        format!(
            "{{\"pages\":{},\"nodes\":{},\"errors\":{}}}",
            result.pages_imported,
            result.nodes_imported,
            serde_json::to_string(&result.errors).unwrap_or_else(|_| "[]".into()),
        )
    }

    /// Export the entire document as JSON for persistence.
    pub fn export_document_json(&self) -> String {
        let snap = self.document.to_snapshot();
        serde_json::to_string(&snap).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    /// Import a document from JSON snapshot, replacing the current document.
    /// Returns status JSON: {"ok":true,"pages":N,"nodes":N} or {"ok":false,"error":"..."}
    pub fn import_document_json(&mut self, json: &str) -> String {
        let snap: rendero_core::document::DocumentSnapshot = match serde_json::from_str(json) {
            Ok(s) => s,
            Err(e) => return format!("{{\"ok\":false,\"error\":\"{}\"}}", e),
        };
        let page_count = snap.pages.len();
        let node_count: usize = snap.pages.iter().map(|p| p.tree.nodes.len()).sum();
        self.document = rendero_core::document::Document::from_snapshot(snap);
        self.current_page = 0;
        self.selected.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.mark_dirty();
        format!("{{\"ok\":true,\"pages\":{},\"nodes\":{}}}", page_count, node_count)
    }

    /// Get image fills visible in the current viewport as JSON.
    /// Returns: [[path, screenX, screenY, screenW, screenH, opacity], ...]
    /// JS uses this to overlay HTMLImageElements after WASM renders the scene.
    pub fn get_visible_image_fills(&mut self, width: u32, height: u32) -> String {
        // Use scene cache directly — NO clone. Camera applied inline.
        let page = self.document.page(self.current_page).unwrap();
        let root_id = page.tree.root_id();

        if self.scene_cache.is_none() {
            let full_viewport = AABB::new(
                Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
                Vec2::new(f32::INFINITY, f32::INFINITY),
            );
            let items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &full_viewport);
            self.scene_cache = Some(items);
            self.rebuild_scene_node_index();
        }
        let items = self.scene_cache.as_ref().unwrap();

        // Quick scan: bail early if no image fills exist at all
        if !self.has_image_fills {
            return "[]".to_string();
        }

        let mut entries = Vec::new();
        let mut clip_stack: Vec<(usize, [f32; 4])> = Vec::new();
        let w = width as f32;
        let h = height as f32;
        let cam_x = self.cam_x;
        let cam_y = self.cam_y;
        let zoom = self.cam_zoom;
        let len = items.len();
        let mut i = 0;

        while i < len {
            let item = &items[i];

            while let Some((end, _)) = clip_stack.last() {
                if i >= *end { clip_stack.pop(); } else { break; }
            }

            // Apply camera transform to bounds inline (no clone)
            let sx_min = (item.world_bounds.min.x - cam_x) * zoom;
            let sy_min = (item.world_bounds.min.y - cam_y) * zoom;
            let sx_max = (item.world_bounds.max.x - cam_x) * zoom;
            let sy_max = (item.world_bounds.max.y - cam_y) * zoom;

            let on_screen = sx_max >= 0.0 && sy_max >= 0.0 && sx_min <= w && sy_min <= h;

            if on_screen {
                for fill in &item.style.fills {
                    if let Paint::Image { path, opacity, scale_mode } = fill {
                        let sw = sx_max - sx_min;
                        let sh = sy_max - sy_min;
                        let escaped_path = path.replace('"', "\\\"");
                        let mode_str = match scale_mode {
                            ImageScaleMode::Fill => "fill",
                            ImageScaleMode::Fit => "fit",
                            ImageScaleMode::Tile => "tile",
                            ImageScaleMode::Stretch => "stretch",
                        };
                        if let Some(clip) = Self::intersect_clips(&clip_stack) {
                            entries.push(format!(
                                "[\"{}\",{},{},{},{},{},{},{},{},{},\"{}\"]",
                                escaped_path, sx_min, sy_min, sw, sh, opacity,
                                clip[0], clip[1], clip[2], clip[3], mode_str
                            ));
                        } else {
                            entries.push(format!(
                                "[\"{}\",{},{},{},{},{},null,null,null,null,\"{}\"]",
                                escaped_path, sx_min, sy_min, sw, sh, opacity, mode_str
                            ));
                        }
                    }
                }
            }

            if item.clips && item.descendant_count > 0 {
                let cw = sx_max - sx_min;
                let ch = sy_max - sy_min;
                if cw.is_finite() && ch.is_finite() {
                    clip_stack.push((i + 1 + item.descendant_count, [sx_min, sy_min, cw, ch]));
                }
            }

            i += 1;
        }

        format!("[{}]", entries.join(","))
    }

    fn intersect_clips(clip_stack: &[(usize, [f32; 4])]) -> Option<[f32; 4]> {
        if clip_stack.is_empty() {
            return None;
        }
        let mut result = clip_stack[0].1;
        for &(_, clip) in &clip_stack[1..] {
            let x1 = result[0].max(clip[0]);
            let y1 = result[1].max(clip[1]);
            let x2 = (result[0] + result[2]).min(clip[0] + clip[2]);
            let y2 = (result[1] + result[3]).min(clip[1] + clip[3]);
            if x2 <= x1 || y2 <= y1 {
                return Some([0.0, 0.0, 0.0, 0.0]);
            }
            result = [x1, y1, x2 - x1, y2 - y1];
        }
        Some(result)
    }

    /// Check if a re-render is needed.
    pub fn needs_render(&self) -> bool {
        self.needs_render
    }

    pub fn node_count(&self) -> usize {
        self.document.page(self.current_page).map(|p| p.tree.len()).unwrap_or(0)
    }

    /// Number of items drawn in last render frame (for diagnostics).
    pub fn drawn_count(&self) -> usize {
        self.last_drawn_count
    }

    // ─── CRDT Sync ────────────────────────────────────────────

    /// Get pending ops as JSON and clear the queue.
    pub fn get_pending_ops(&mut self) -> String {
        let ops = std::mem::take(&mut self.pending_ops);
        serde_json::to_string(&ops).unwrap_or_else(|_| "[]".into())
    }

    /// Apply remote operations (JSON array of Operation).
    /// Returns number of ops applied.
    pub fn apply_remote_ops(&mut self, json: &str) -> u32 {
        let ops: Vec<Operation> = match serde_json::from_str(json) {
            Ok(o) => o,
            Err(_) => return 0,
        };
        let mut applied = 0u32;
        for op in &ops {
            // Merge remote clock to maintain causality
            self.document.clock.merge(op.id.0.counter);
            if let Some(page) = self.document.page_mut(self.current_page) {
                match apply::apply(&mut page.tree, op) {
                    apply::ApplyResult::Applied => { applied += 1; }
                    _ => {}
                }
            }
        }
        if applied > 0 {
            // Structural: remote CRDT ops can add/remove/move any node.
            self.mark_dirty();
        }
        applied
    }

    /// Apply an undo action and return the reverse action for the opposite stack.
    fn apply_undo_action(&mut self, action: &UndoAction) -> Option<UndoAction> {
        match action {
            UndoAction::AddNode { node, .. } => {
                // Reverse of add = remove
                let node_id = node.id;
                let parent_id = self.document.page(self.current_page)
                    .and_then(|p| p.tree.parent_of(&node_id));
                let node_clone = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(&node_id).cloned());
                if let Some(page) = self.document.page_mut(self.current_page) {
                    let _ = page.tree.remove(&node_id);
                }
                match (node_clone, parent_id) {
                    (Some(n), Some(pid)) => Some(UndoAction::RemoveNode { node: n, parent_id: pid }),
                    _ => None,
                }
            }
            UndoAction::RemoveNode { node, parent_id } => {
                // Reverse of remove = add
                let node_clone = node.clone();
                if let Some(page) = self.document.page_mut(self.current_page) {
                    let _ = page.tree.insert(node.clone(), *parent_id, 0);
                }
                Some(UndoAction::AddNode { node: node_clone, parent_id: *parent_id })
            }
            UndoAction::MoveNode { node_id, tx, ty } => {
                let cur = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(node_id))
                    .map(|n| (n.transform.tx, n.transform.ty));
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        node.transform.tx = *tx;
                        node.transform.ty = *ty;
                    }
                }
                cur.map(|(cx, cy)| UndoAction::MoveNode { node_id: *node_id, tx: cx, ty: cy })
            }
            UndoAction::ResizeNode { node_id, tx, ty, w, h } => {
                let cur = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(node_id))
                    .map(|n| (n.transform.tx, n.transform.ty, n.width, n.height));
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        node.transform.tx = *tx;
                        node.transform.ty = *ty;
                        node.width = *w;
                        node.height = *h;
                    }
                }
                cur.map(|(cx, cy, cw, ch)| UndoAction::ResizeNode { node_id: *node_id, tx: cx, ty: cy, w: cw, h: ch })
            }
            UndoAction::ChangeFill { node_id, fills } => {
                let cur = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(node_id))
                    .map(|n| n.style.fills.clone());
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        node.style.fills = fills.clone();
                    }
                }
                cur.map(|old_fills| UndoAction::ChangeFill { node_id: *node_id, fills: old_fills })
            }
            UndoAction::ChangeName { node_id, name } => {
                let cur = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(node_id))
                    .map(|n| n.name.clone());
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        node.name = name.clone();
                    }
                }
                cur.map(|old_name| UndoAction::ChangeName { node_id: *node_id, name: old_name })
            }
            UndoAction::ChangeText { node_id, runs, width, height } => {
                let cur = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(node_id))
                    .and_then(|n| {
                        if let NodeKind::Text { runs: ref old_runs, .. } = n.kind {
                            Some((old_runs.clone(), n.width, n.height))
                        } else {
                            None
                        }
                    });
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        if let NodeKind::Text { runs: ref mut node_runs, .. } = node.kind {
                            *node_runs = runs.clone();
                        }
                        node.width = *width;
                        node.height = *height;
                    }
                }
                cur.map(|(old_runs, old_w, old_h)| UndoAction::ChangeText {
                    node_id: *node_id, runs: old_runs, width: old_w, height: old_h,
                })
            }
            UndoAction::EditVector { node_id, paths, width, height, tx, ty } => {
                let cur = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(node_id))
                    .and_then(|n| {
                        if let NodeKind::Vector { ref paths } = n.kind {
                            Some((paths.clone(), n.width, n.height, n.transform.tx, n.transform.ty))
                        } else {
                            None
                        }
                    });
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        if let NodeKind::Vector { paths: ref mut node_paths } = node.kind {
                            *node_paths = paths.clone();
                        }
                        node.width = *width;
                        node.height = *height;
                        node.transform.tx = *tx;
                        node.transform.ty = *ty;
                    }
                }
                cur.map(|(old_paths, old_w, old_h, old_tx, old_ty)| UndoAction::EditVector {
                    node_id: *node_id, paths: old_paths, width: old_w, height: old_h, tx: old_tx, ty: old_ty,
                })
            }
            UndoAction::RotateNode { node_id, transform } => {
                let cur = self.document.page(self.current_page)
                    .and_then(|p| p.tree.get(node_id))
                    .map(|n| n.transform);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(node_id) {
                        node.transform = *transform;
                    }
                }
                cur.map(|old_t| UndoAction::RotateNode { node_id: *node_id, transform: old_t })
            }
        }
    }

    /// Undo the last action. Returns true if something was undone.
    pub fn undo(&mut self) -> bool {
        let action = match self.undo_stack.pop() {
            Some(a) => a,
            None => return false,
        };
        if let Some(reverse) = self.apply_undo_action(&action) {
            self.redo_stack.push(reverse);
        }
        self.apply_undo_scene_update(&action);
        true
    }

    /// Redo the last undone action. Returns true if something was redone.
    pub fn redo(&mut self) -> bool {
        let action = match self.redo_stack.pop() {
            Some(a) => a,
            None => return false,
        };
        if let Some(reverse) = self.apply_undo_action(&action) {
            self.undo_stack.push(reverse);
        }
        self.apply_undo_scene_update(&action);
        true
    }

    /// Incrementally update scene cache after an undo/redo action.
    /// Avoids full 500ms+ scene rebuild for 1.8M nodes.
    fn apply_undo_scene_update(&mut self, action: &UndoAction) {
        match action {
            UndoAction::AddNode { node, .. } => {
                // Undo of add = remove. Node was already removed from tree.
                self.scene_remove_leaf(node.id);
            }
            UndoAction::RemoveNode { node, parent_id } => {
                // Undo of remove = re-add. Node was already re-added to tree.
                self.scene_insert_leaf(node, *parent_id);
            }
            UndoAction::MoveNode { node_id, tx, ty } => {
                self.patch_scene_transform(*node_id, *tx, *ty, None, None);
                self.needs_render = true;
            }
            UndoAction::ResizeNode { node_id, tx, ty, w, h } => {
                self.patch_scene_transform(*node_id, *tx, *ty, Some(*w), Some(*h));
                self.needs_render = true;
            }
            UndoAction::ChangeFill { node_id, .. } => {
                self.patch_scene_style(*node_id);
                self.needs_render = true;
            }
            UndoAction::ChangeName { .. } => {
                self.needs_render = true;
            }
            UndoAction::ChangeText { node_id, width, height, .. } => {
                // Text changes affect shape (runs) — update in-place
                self.patch_scene_text(*node_id);
                if let Some(page) = self.document.page(self.current_page) {
                    if let Some(node) = page.tree.get(node_id) {
                        self.patch_scene_transform(*node_id, node.transform.tx, node.transform.ty, Some(node.width), Some(node.height));
                    }
                }
                self.needs_render = true;
            }
            UndoAction::EditVector { node_id, .. } => {
                // Vector path changed — update shape in-place (fast path)
                self.patch_scene_shape(*node_id);
            }
            UndoAction::RotateNode { node_id, .. } => {
                // Rotation changes the full transform matrix — need scene rebuild
                self.mark_dirty();
            }
        }
    }

    /// Get properties of the selected node as JSON.
    /// Returns empty string if nothing is selected.
    pub fn get_node_info(&self, counter: u32, client_id: u32) -> String {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return String::new(),
        };
        let node = match page.tree.get(&node_id) {
            Some(n) => n,
            None => return String::new(),
        };

        // For text nodes, get color from the first run; for others, from style.fills
        let fill_color = if let NodeKind::Text { runs, .. } = &node.kind {
            runs.first().map(|r| {
                let c = &r.color;
                format!("rgba({},{},{},{:.2})", (c.r()*255.0) as u8, (c.g()*255.0) as u8, (c.b()*255.0) as u8, c.a())
            }).unwrap_or_default()
        } else {
            node.style.fills.first().map(|f| match f {
                Paint::Solid(c) => format!("rgba({},{},{},{:.2})", (c.r()*255.0) as u8, (c.g()*255.0) as u8, (c.b()*255.0) as u8, c.a()),
                Paint::LinearGradient { .. } => "linear-gradient".to_string(),
                Paint::RadialGradient { .. } => "radial-gradient".to_string(),
                _ => "unknown".to_string(),
            }).unwrap_or_default()
        };

        // Serialize full fills array for gradient UI
        let fills_json = {
            let fills_src = if let NodeKind::Text { runs, .. } = &node.kind {
                // For text: synthesize fill from first run
                if let Some(run) = runs.first() {
                    if let Some(ref paint) = run.fill_override {
                        vec![paint.clone()]
                    } else {
                        vec![Paint::Solid(run.color)]
                    }
                } else { vec![] }
            } else {
                node.style.fills.clone()
            };
            let entries: Vec<String> = fills_src.iter().map(|p| Self::paint_to_json(p)).collect();
            format!("[{}]", entries.join(","))
        };

        let node_type = match &node.kind {
            NodeKind::Frame { .. } => "frame",
            NodeKind::Rectangle { .. } => "rectangle",
            NodeKind::Ellipse { .. } => "ellipse",
            NodeKind::Line => "line",
            NodeKind::Polygon { .. } => "polygon",
            NodeKind::Vector { .. } => "vector",
            NodeKind::Text { .. } => "text",
            NodeKind::BooleanOp { .. } => "boolean",
            NodeKind::Component => "component",
            NodeKind::Instance { .. } => "instance",
            NodeKind::Image { .. } => "image",
        };

        let (text_content, font_size, font_family, font_weight, letter_spacing, line_height, text_decoration, text_vertical_align) = if let NodeKind::Text { runs, vertical_align, .. } = &node.kind {
            let text = runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join("");
            let size = runs.first().map(|r| r.font_size).unwrap_or(24.0);
            let family = runs.first().map(|r| r.font_family.as_str()).unwrap_or("Inter");
            let weight = runs.first().map(|r| r.font_weight).unwrap_or(400);
            let ls = runs.first().map(|r| r.letter_spacing).unwrap_or(0.0);
            let lh = runs.first().and_then(|r| r.line_height).unwrap_or(0.0);
            let dec = runs.first().map(|r| match r.decoration {
                rendero_core::node::TextDecoration::None => "none",
                rendero_core::node::TextDecoration::Underline => "underline",
                rendero_core::node::TextDecoration::Strikethrough => "strikethrough",
            }).unwrap_or("none");
            let va = match vertical_align {
                rendero_core::node::TextVerticalAlign::Top => "top",
                rendero_core::node::TextVerticalAlign::Center => "center",
                rendero_core::node::TextVerticalAlign::Bottom => "bottom",
            };
            (text, size, family, weight, ls, lh, dec, va)
        } else {
            (String::new(), 0.0, "Inter", 400u16, 0.0, 0.0, "none", "top")
        };

        // Escape quotes in text content and name for JSON safety
        let escaped_text = text_content.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_name = node.name.replace('\\', "\\\\").replace('"', "\\\"");

        let blend_str = match node.style.blend_mode {
            rendero_core::properties::BlendMode::Normal => "normal",
            rendero_core::properties::BlendMode::Multiply => "multiply",
            rendero_core::properties::BlendMode::Screen => "screen",
            rendero_core::properties::BlendMode::Overlay => "overlay",
            rendero_core::properties::BlendMode::Darken => "darken",
            rendero_core::properties::BlendMode::Lighten => "lighten",
            rendero_core::properties::BlendMode::ColorDodge => "color-dodge",
            rendero_core::properties::BlendMode::ColorBurn => "color-burn",
            rendero_core::properties::BlendMode::HardLight => "hard-light",
            rendero_core::properties::BlendMode::SoftLight => "soft-light",
            rendero_core::properties::BlendMode::Difference => "difference",
            rendero_core::properties::BlendMode::Exclusion => "exclusion",
            rendero_core::properties::BlendMode::Hue => "hue",
            rendero_core::properties::BlendMode::Saturation => "saturation",
            rendero_core::properties::BlendMode::ColorMode => "color",
            rendero_core::properties::BlendMode::Luminosity => "luminosity",
        };

        let stroke_color = node.style.strokes.first().map(|s| match s {
            Paint::Solid(c) => format!("rgba({},{},{},{:.2})", (c.r()*255.0) as u8, (c.g()*255.0) as u8, (c.b()*255.0) as u8, c.a()),
            _ => "gradient".to_string(),
        }).unwrap_or_default();
        let stroke_weight = node.style.stroke_weight;

        // Constraint info
        let ch_str = match node.constraint_horizontal {
            rendero_core::properties::ConstraintType::Min => "left",
            rendero_core::properties::ConstraintType::Max => "right",
            rendero_core::properties::ConstraintType::MinMax => "leftRight",
            rendero_core::properties::ConstraintType::Center => "center",
            rendero_core::properties::ConstraintType::Scale => "scale",
        };
        let cv_str = match node.constraint_vertical {
            rendero_core::properties::ConstraintType::Min => "top",
            rendero_core::properties::ConstraintType::Max => "bottom",
            rendero_core::properties::ConstraintType::MinMax => "topBottom",
            rendero_core::properties::ConstraintType::Center => "center",
            rendero_core::properties::ConstraintType::Scale => "scale",
        };

        // Auto-layout info for frames
        let auto_layout_json = if let NodeKind::Frame { auto_layout: Some(al), .. } = &node.kind {
            let dir = match al.direction {
                rendero_core::properties::LayoutDirection::Horizontal => "horizontal",
                rendero_core::properties::LayoutDirection::Vertical => "vertical",
            };
            format!(
                r#","autoLayout":{{"direction":"{}","spacing":{:.0},"padTop":{:.0},"padRight":{:.0},"padBottom":{:.0},"padLeft":{:.0}}}"#,
                dir, al.spacing, al.padding_top, al.padding_right, al.padding_bottom, al.padding_left
            )
        } else {
            String::new()
        };

        let mask_json = if node.is_mask { r#","isMask":true"# } else { "" };

        // Component instance: include reference to component definition
        let component_json = if let NodeKind::Instance { component_id, .. } = &node.kind {
            format!(r#","componentId":[{},{}]"#, component_id.0.counter, component_id.0.client_id)
        } else {
            String::new()
        };

        // Rotation in degrees from transform matrix
        let rotation_deg = node.transform.b.atan2(node.transform.a).to_degrees();
        let rotation_json = if rotation_deg.abs() > 0.01 {
            format!(r#","rotation":{:.1}"#, rotation_deg)
        } else {
            String::new()
        };

        // Stroke alignment
        let stroke_align_str = match node.style.stroke_align {
            rendero_core::properties::StrokeAlign::Inside => "inside",
            rendero_core::properties::StrokeAlign::Center => "center",
            rendero_core::properties::StrokeAlign::Outside => "outside",
        };
        let stroke_align_json = format!(r#","strokeAlign":"{}""#, stroke_align_str);

        // Text typography properties
        let typography_json = {
            let ff = format!(r#","fontFamily":"{}""#, font_family);
            let fw = if font_weight != 400 { format!(r#","fontWeight":{}"#, font_weight) } else { String::new() };
            let ls = if letter_spacing.abs() > 0.01 { format!(r#","letterSpacing":{:.1}"#, letter_spacing) } else { String::new() };
            let lh = if line_height > 0.0 { format!(r#","lineHeight":{:.1}"#, line_height) } else { String::new() };
            let dec = if text_decoration != "none" { format!(r#","textDecoration":"{}""#, text_decoration) } else { String::new() };
            let va = if text_vertical_align != "top" { format!(r#","textVerticalAlign":"{}""#, text_vertical_align) } else { String::new() };
            format!("{}{}{}{}{}{}", ff, fw, ls, lh, dec, va)
        };

        format!(
            r#"{{"name":"{}","x":{:.1},"y":{:.1},"width":{:.1},"height":{:.1},"fill":"{}","fills":{},"type":"{}","text":"{}","fontSize":{:.1},"opacity":{:.2},"blendMode":"{}","stroke":"{}","strokeWeight":{:.1},"constraintH":"{}","constraintV":"{}"{}{}{}{}{}{}}}"#,
            escaped_name, node.transform.tx, node.transform.ty, node.width, node.height, fill_color, fills_json, node_type, escaped_text, font_size, node.style.opacity, blend_str, stroke_color, stroke_weight, ch_str, cv_str, auto_layout_json, mask_json, component_json, rotation_json, stroke_align_json, typography_json
        )
    }

    /// Total number of layers (children of root).
    pub fn layer_count(&self) -> u32 {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return 0,
        };
        let root_id = page.tree.root_id();
        match page.tree.children_of(&root_id) {
            Some(c) => c.len() as u32,
            None => 0,
        }
    }

    /// Get a range of layers as JSON: [{"id":[counter,client_id],"name":"..."}]
    /// `start` is 0-based index, `count` is max items to return.
    pub fn get_layers_range(&self, start: u32, count: u32) -> String {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return "[]".to_string(),
        };
        let root_id = page.tree.root_id();
        let child_list = match page.tree.children_of(&root_id) {
            Some(c) => c,
            None => return "[]".to_string(),
        };

        let total = child_list.len();
        let start_idx = (start as usize).min(total);
        let take_count = (count as usize).min(total.saturating_sub(start_idx));

        let mut entries = Vec::with_capacity(take_count);
        for child_id in child_list.iter().skip(start_idx).take(take_count) {
            if let Some(node) = page.tree.get(child_id) {
                let escaped = node.name.replace('\\', "\\\\").replace('"', "\\\"");
                entries.push(format!(
                    r#"{{"id":[{},{}],"name":"{}"}}"#,
                    child_id.0.counter, child_id.0.client_id, escaped
                ));
            }
        }
        format!("[{}]", entries.join(","))
    }

    /// Get layer list as JSON array: [{"id":[counter,client_id],"name":"..."}]
    pub fn get_layers(&self) -> String {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return "[]".to_string(),
        };
        let root_id = page.tree.root_id();
        let child_list = match page.tree.children_of(&root_id) {
            Some(c) => c,
            None => return "[]".to_string(),
        };

        let mut entries = Vec::new();
        for child_id in child_list.iter() {
            if let Some(node) = page.tree.get(child_id) {
                let escaped = node.name.replace('\\', "\\\\").replace('"', "\\\"");
                entries.push(format!(
                    r#"{{"id":[{},{}],"name":"{}"}}"#,
                    child_id.0.counter, child_id.0.client_id, escaped
                ));
            }
        }
        format!("[{}]", entries.join(","))
    }

    /// Get layer tree as flat DFS list with depth info.
    /// `expanded_ids` is comma-separated "counter:client" pairs for expanded nodes.
    /// Returns JSON: [{"id":[c,cl],"name":"...","depth":N,"hasChildren":bool,"kind":"frame"|...}]
    /// Only descends into expanded nodes. Supports virtualized rendering.
    pub fn get_tree_layers(&self, expanded_ids: &str, start: u32, count: u32) -> Vec<u32> {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return vec![0],  // [total_count]
        };

        // Parse expanded set
        let mut expanded = std::collections::HashSet::new();
        for pair in expanded_ids.split(',') {
            let parts: Vec<&str> = pair.split(':').collect();
            if parts.len() == 2 {
                if let (Ok(c), Ok(cl)) = (parts[0].parse::<u64>(), parts[1].parse::<u32>()) {
                    expanded.insert(NodeId::new(c, cl));
                }
            }
        }

        // DFS walk, collecting visible rows
        let root_id = page.tree.root_id();
        let mut rows: Vec<(NodeId, u16, bool, u8)> = Vec::new(); // (id, depth, hasChildren, kind)

        fn walk(
            tree: &rendero_core::tree::DocumentTree,
            node_id: &NodeId,
            depth: u16,
            expanded: &std::collections::HashSet<NodeId>,
            rows: &mut Vec<(NodeId, u16, bool, u8)>,
        ) {
            let children = tree.children_of(node_id);
            let has_children = children.map(|c| c.len() > 0).unwrap_or(false);
            let kind = tree.get(node_id).map(|n| match &n.kind {
                rendero_core::node::NodeKind::Frame { .. } => 0u8,
                rendero_core::node::NodeKind::Rectangle { .. } => 1,
                rendero_core::node::NodeKind::Ellipse { .. } => 2,
                rendero_core::node::NodeKind::Text { .. } => 3,
                rendero_core::node::NodeKind::Vector { .. } => 4,
                rendero_core::node::NodeKind::Image { .. } => 5,
                rendero_core::node::NodeKind::BooleanOp { .. } => 6,
                _ => 7,
            }).unwrap_or(7);

            rows.push((*node_id, depth, has_children, kind));

            if has_children && expanded.contains(node_id) {
                if let Some(children) = tree.children_of(node_id) {
                    for child_id in children.iter() {
                        walk(tree, child_id, depth + 1, expanded, rows);
                    }
                }
            }
        }

        // Walk root's children in natural order (first child = top of layer panel)
        if let Some(children) = page.tree.children_of(&root_id) {
            for child_id in children.iter() {
                walk(&page.tree, child_id, 0, &expanded, &mut rows);
            }
        }

        let total = rows.len() as u32;
        let start_idx = (start as usize).min(rows.len());
        let take_count = (count as usize).min(rows.len().saturating_sub(start_idx));

        // Pack into u32 array: [total, (counter, client_id, depth_hasChildren_kind) × N]
        // depth_hasChildren_kind = depth << 16 | hasChildren << 8 | kind
        let mut result = Vec::with_capacity(1 + take_count * 3);
        result.push(total);
        for (id, depth, has_children, kind) in rows.iter().skip(start_idx).take(take_count) {
            result.push(id.0.counter as u32);
            result.push(id.0.client_id);
            let packed = (*depth as u32) << 16 | ((*has_children as u32) << 8) | (*kind as u32);
            result.push(packed);
        }
        result
    }

    /// Find nodes by name substring. Returns JSON array of {counter, client_id, name, info}.
    pub fn find_nodes_by_name(&self, query: &str) -> String {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return "[]".into(),
        };
        let root = page.tree.root_id();
        let traversal = page.tree.traverse_depth_first(&root);
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        for node_id in &traversal {
            if let Some(node) = page.tree.get(node_id) {
                if node.name.to_lowercase().contains(&query_lower) {
                    let info = self.get_node_info(node_id.0.counter as u32, node_id.0.client_id);
                    results.push(format!(
                        "{{\"counter\":{},\"client_id\":{},\"info\":{}}}",
                        node_id.0.counter, node_id.0.client_id, info
                    ));
                    if results.len() >= 20 { break; }
                }
            }
        }
        format!("[{}]", results.join(","))
    }

    /// Get a node's name by ID. Returns empty string if not found.
    pub fn get_node_name(&self, counter: u32, client_id: u32) -> String {
        let node_id = NodeId::new(counter as u64, client_id);
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return String::new(),
        };
        match page.tree.get(&node_id) {
            Some(n) => n.name.clone(),
            None => String::new(),
        }
    }

    /// Get all image assets on the current page.
    /// Returns JSON array: [{type:"node"|"fill", key:string, name:string, counter:u64, client_id:u32}]
    /// "node" = NodeKind::Image (raw pixels), "fill" = Paint::Image (referenced by path).
    pub fn get_all_image_keys(&self) -> String {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return "[]".into(),
        };
        let root = page.tree.root_id();
        let traversal = page.tree.traverse_depth_first(&root);
        let mut results = Vec::new();
        let mut seen_paths = std::collections::BTreeSet::new();
        for node_id in &traversal {
            if let Some(node) = page.tree.get(node_id) {
                // NodeKind::Image — raw pixel image nodes
                if matches!(&node.kind, rendero_core::node::NodeKind::Image { .. }) {
                    results.push(format!(
                        "{{\"type\":\"node\",\"key\":\"{}\",\"name\":\"{}\",\"counter\":{},\"client_id\":{}}}",
                        node.name.replace('"', "\\\""),
                        node.name.replace('"', "\\\""),
                        node_id.0.counter, node_id.0.client_id
                    ));
                }
                // Paint::Image fills
                for fill in &node.style.fills {
                    if let rendero_core::properties::Paint::Image { path, .. } = fill {
                        if seen_paths.insert(path.clone()) {
                            results.push(format!(
                                "{{\"type\":\"fill\",\"key\":\"{}\",\"name\":\"{}\",\"counter\":{},\"client_id\":{}}}",
                                path.replace('"', "\\\""),
                                node.name.replace('"', "\\\""),
                                node_id.0.counter, node_id.0.client_id
                            ));
                        }
                    }
                }
            }
        }
        format!("[{}]", results.join(","))
    }

    /// Find nodes that use a specific image key. Returns JSON array of {counter, client_id, name}.
    pub fn find_nodes_with_image(&self, image_key: &str) -> String {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return "[]".into(),
        };
        let root = page.tree.root_id();
        let traversal = page.tree.traverse_depth_first(&root);
        let mut results = Vec::new();
        for node_id in &traversal {
            if let Some(node) = page.tree.get(node_id) {
                // Match by node name (for NodeKind::Image) or by fill path
                let is_match = node.name == image_key || node.style.fills.iter().any(|f| {
                    matches!(f, rendero_core::properties::Paint::Image { path, .. } if path == image_key)
                });
                if is_match {
                    results.push(format!(
                        "{{\"counter\":{},\"client_id\":{},\"name\":\"{}\"}}",
                        node_id.0.counter, node_id.0.client_id,
                        node.name.replace('"', "\\\"")
                    ));
                    if results.len() >= 50 { break; }
                }
            }
        }
        format!("[{}]", results.join(","))
    }

    // ─── Page management ─────────────────────────────────────────────

    /// Get number of pages.
    pub fn page_count(&self) -> u32 {
        self.document.pages.len() as u32
    }

    /// Get current page index.
    pub fn current_page_index(&self) -> u32 {
        self.current_page as u32
    }

    /// Get pages as JSON: [{"index":0,"name":"Page 1"},...]
    pub fn get_pages(&self) -> String {
        let mut entries = Vec::new();
        for (i, page) in self.document.pages.iter().enumerate() {
            let escaped = page.name.replace('"', "\\\"");
            entries.push(format!(r#"{{"index":{},"name":"{}"}}"#, i, escaped));
        }
        format!("[{}]", entries.join(","))
    }

    /// Add a new page and return its index.
    pub fn add_page(&mut self, name: &str) -> u32 {
        self.document.add_page(name);
        (self.document.pages.len() - 1) as u32
    }

    /// Switch to a different page by index.
    pub fn switch_page(&mut self, index: u32) -> bool {
        if (index as usize) < self.document.pages.len() {
            self.current_page = index as usize;
            self.selected.clear();
            // Structural: entirely different page/tree — cache is for the old page.
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Rename a page.
    pub fn rename_page(&mut self, index: u32, name: &str) -> bool {
        if let Some(page) = self.document.pages.get_mut(index as usize) {
            page.name = name.to_string();
            true
        } else {
            false
        }
    }

    /// Select a node by ID (from layers panel click). Replaces current selection.
    pub fn select_node(&mut self, counter: u32, client_id: u32) {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        self.selected = vec![node_id];
        self.mark_selection_dirty();
    }

    /// Toggle a node in/out of the selection (shift-click in layers panel).
    pub fn toggle_select_node(&mut self, counter: u32, client_id: u32) {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        if let Some(pos) = self.selected.iter().position(|id| *id == node_id) {
            self.selected.remove(pos);
        } else {
            self.selected.push(node_id);
        }
        self.mark_selection_dirty();
    }

    /// Set node position from the properties panel.
    pub fn set_node_position(&mut self, counter: u32, client_id: u32, x: f32, y: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let old_tx = node.transform.tx;
        let old_ty = node.transform.ty;
        self.undo_stack.push(UndoAction::MoveNode { node_id, tx: old_tx, ty: old_ty });
        self.redo_stack.clear();
        node.transform.tx = x;
        node.transform.ty = y;
        self.patch_scene_transform(node_id, x, y, None, None);
        self.needs_render = true;
        true
    }

    /// Set node size from the properties panel.
    pub fn set_node_size(&mut self, counter: u32, client_id: u32, w: f32, h: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let old = UndoAction::ResizeNode { node_id, tx: node.transform.tx, ty: node.transform.ty, w: node.width, h: node.height };
        self.undo_stack.push(old);
        self.redo_stack.clear();
        node.width = w;
        node.height = h;
        let tx = node.transform.tx;
        let ty = node.transform.ty;
        self.patch_scene_transform(node_id, tx, ty, Some(w), Some(h));
        self.needs_render = true;
        true
    }

    /// Set node rotation in degrees. Preserves scale.
    pub fn set_node_rotation(&mut self, counter: u32, client_id: u32, degrees: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let old_transform = node.transform;
        self.undo_stack.push(UndoAction::RotateNode { node_id, transform: old_transform });
        self.redo_stack.clear();

        // Extract current scale from the transform
        let sx = (old_transform.a * old_transform.a + old_transform.b * old_transform.b).sqrt();
        let sy = (old_transform.c * old_transform.c + old_transform.d * old_transform.d).sqrt();

        let rad = degrees.to_radians();
        let (sin, cos) = rad.sin_cos();
        node.transform.a = sx * cos;
        node.transform.b = sx * sin;
        node.transform.c = -sy * sin;
        node.transform.d = sy * cos;

        let new_transform = node.transform;
        let w = node.width;
        let h = node.height;
        self.patch_scene_full_transform(node_id, new_transform, w, h);
        self.needs_render = true;
        true
    }

    /// Set node fill color (RGBA 0-1 range).
    /// For text nodes, also updates the per-run text color.
    pub fn set_node_fill(&mut self, counter: u32, client_id: u32, r: f32, g: f32, b: f32, a: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let color = Color::new(r, g, b, a);
        // For text nodes, update per-run color (that's what the renderer uses)
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            let old_width = node.width;
            let old_height = node.height;
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: old_width, height: old_height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                run.color = color;
            }
        } else {
            let old_fills = node.style.fills.clone();
            self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
            self.redo_stack.clear();
            node.style.fills = vec![Paint::Solid(color)];
        }
        // Text: update RenderShape (runs have color) + style in scene cache in-place.
        // Non-text: update style only — fills are in style, not shape.
        // Both avoid full scene rebuild (which costs seconds on 1.8M nodes).
        if matches!(self.document.page(self.current_page)
            .and_then(|p| p.tree.get(&node_id))
            .map(|n| &n.kind), Some(NodeKind::Text { .. })) {
            self.patch_scene_text(node_id);
        } else {
            self.mark_style_dirty(node_id);
        }
        true
    }

    /// Set all fills on a node from a JSON array. Handles solid, gradient, and image fills.
    /// JSON format: [{"type":"solid","r":255,"g":0,"b":0,"a":1.0}, {"type":"linear","startX":0,...,"stops":[...]}, ...]
    pub fn set_node_fills_json(&mut self, counter: u32, client_id: u32, fills_json: &str) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        // Parse JSON
        let arr: Vec<serde_json::Value> = match serde_json::from_str(fills_json) {
            Ok(v) => v, Err(_) => return false,
        };
        let mut new_fills = Vec::new();
        for entry in &arr {
            let t = entry["type"].as_str().unwrap_or("");
            match t {
                "solid" => {
                    let r = entry["r"].as_f64().unwrap_or(0.0) as f32 / 255.0;
                    let g = entry["g"].as_f64().unwrap_or(0.0) as f32 / 255.0;
                    let b = entry["b"].as_f64().unwrap_or(0.0) as f32 / 255.0;
                    let a = entry["a"].as_f64().unwrap_or(1.0) as f32;
                    new_fills.push(Paint::Solid(Color::new(r, g, b, a)));
                }
                "linear" => {
                    let stops = Self::parse_json_stops(&entry["stops"]);
                    new_fills.push(Paint::LinearGradient {
                        stops,
                        start: Vec2::new(entry["startX"].as_f64().unwrap_or(0.0) as f32, entry["startY"].as_f64().unwrap_or(0.0) as f32),
                        end: Vec2::new(entry["endX"].as_f64().unwrap_or(1.0) as f32, entry["endY"].as_f64().unwrap_or(1.0) as f32),
                    });
                }
                "radial" => {
                    let stops = Self::parse_json_stops(&entry["stops"]);
                    new_fills.push(Paint::RadialGradient {
                        stops,
                        center: Vec2::new(entry["centerX"].as_f64().unwrap_or(0.5) as f32, entry["centerY"].as_f64().unwrap_or(0.5) as f32),
                        radius: entry["radius"].as_f64().unwrap_or(0.5) as f32,
                    });
                }
                "angular" => {
                    let stops = Self::parse_json_stops(&entry["stops"]);
                    new_fills.push(Paint::AngularGradient {
                        stops,
                        center: Vec2::new(entry["centerX"].as_f64().unwrap_or(0.5) as f32, entry["centerY"].as_f64().unwrap_or(0.5) as f32),
                        start_angle: entry["startAngle"].as_f64().unwrap_or(0.0) as f32,
                    });
                }
                "diamond" => {
                    let stops = Self::parse_json_stops(&entry["stops"]);
                    new_fills.push(Paint::DiamondGradient {
                        stops,
                        center: Vec2::new(entry["centerX"].as_f64().unwrap_or(0.5) as f32, entry["centerY"].as_f64().unwrap_or(0.5) as f32),
                        radius: entry["radius"].as_f64().unwrap_or(0.5) as f32,
                    });
                }
                "image" => {
                    let path = entry["path"].as_str().unwrap_or("").to_string();
                    let mode = match entry["scaleMode"].as_str().unwrap_or("fill") {
                        "fit" => ImageScaleMode::Fit, "tile" => ImageScaleMode::Tile,
                        "stretch" => ImageScaleMode::Stretch, _ => ImageScaleMode::Fill,
                    };
                    let opacity = entry["opacity"].as_f64().unwrap_or(1.0) as f32;
                    new_fills.push(Paint::Image { path, scale_mode: mode, opacity });
                }
                _ => {}
            }
        }

        // For text nodes, update run fills
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: node.width, height: node.height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                if let Some(fill) = new_fills.first() {
                    match fill {
                        Paint::Solid(c) => { run.color = *c; run.fill_override = None; }
                        other => { run.fill_override = Some(other.clone()); }
                    }
                }
            }
            self.patch_scene_text(node_id);
            self.needs_render = true;
        } else {
            let old_fills = node.style.fills.clone();
            self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
            self.redo_stack.clear();
            node.style.fills = new_fills;
            self.mark_style_dirty(node_id);
        }
        true
    }

    /// Parse gradient stops from JSON array: [{"position":0,"r":255,"g":0,"b":0,"a":1}, ...]
    fn parse_json_stops(val: &serde_json::Value) -> Vec<GradientStop> {
        let mut stops = Vec::new();
        if let Some(arr) = val.as_array() {
            for s in arr {
                let pos = s["position"].as_f64().unwrap_or(0.0) as f32;
                let r = s["r"].as_f64().unwrap_or(0.0) as f32 / 255.0;
                let g = s["g"].as_f64().unwrap_or(0.0) as f32 / 255.0;
                let b = s["b"].as_f64().unwrap_or(0.0) as f32 / 255.0;
                let a = s["a"].as_f64().unwrap_or(1.0) as f32;
                stops.push(GradientStop::new(pos, Color::new(r, g, b, a)));
            }
        }
        stops
    }

    /// Helper: parse stop_positions + stop_colors flat arrays into Vec<GradientStop>.
    fn parse_gradient_stops(stop_positions: &[f32], stop_colors: &[f32]) -> Vec<GradientStop> {
        let mut stops = Vec::new();
        for i in 0..stop_positions.len() {
            let ci = i * 4;
            if ci + 3 < stop_colors.len() {
                stops.push(GradientStop::new(
                    stop_positions[i],
                    Color::new(stop_colors[ci], stop_colors[ci+1], stop_colors[ci+2], stop_colors[ci+3]),
                ));
            }
        }
        stops
    }

    /// Serialize a Paint to JSON string for get_node_info fills array.
    fn paint_to_json(paint: &Paint) -> String {
        fn stops_json(stops: &[GradientStop]) -> String {
            let ss: Vec<String> = stops.iter().map(|s| {
                format!(r#"{{"position":{:.3},"r":{},"g":{},"b":{},"a":{:.2}}}"#,
                    s.position, (s.color.r()*255.0) as u8, (s.color.g()*255.0) as u8, (s.color.b()*255.0) as u8, s.color.a())
            }).collect();
            format!("[{}]", ss.join(","))
        }
        match paint {
            Paint::Solid(c) => format!(
                r#"{{"type":"solid","r":{},"g":{},"b":{},"a":{:.2}}}"#,
                (c.r()*255.0) as u8, (c.g()*255.0) as u8, (c.b()*255.0) as u8, c.a()),
            Paint::LinearGradient { stops, start, end } => format!(
                r#"{{"type":"linear","startX":{:.3},"startY":{:.3},"endX":{:.3},"endY":{:.3},"stops":{}}}"#,
                start.x, start.y, end.x, end.y, stops_json(stops)),
            Paint::RadialGradient { stops, center, radius } => format!(
                r#"{{"type":"radial","centerX":{:.3},"centerY":{:.3},"radius":{:.3},"stops":{}}}"#,
                center.x, center.y, radius, stops_json(stops)),
            Paint::AngularGradient { stops, center, start_angle } => format!(
                r#"{{"type":"angular","centerX":{:.3},"centerY":{:.3},"startAngle":{:.3},"stops":{}}}"#,
                center.x, center.y, start_angle, stops_json(stops)),
            Paint::DiamondGradient { stops, center, radius } => format!(
                r#"{{"type":"diamond","centerX":{:.3},"centerY":{:.3},"radius":{:.3},"stops":{}}}"#,
                center.x, center.y, radius, stops_json(stops)),
            Paint::Image { path, scale_mode, opacity } => {
                let mode = match scale_mode {
                    ImageScaleMode::Fill => "fill", ImageScaleMode::Fit => "fit",
                    ImageScaleMode::Tile => "tile", ImageScaleMode::Stretch => "stretch",
                };
                let escaped = path.replace('\\', "\\\\").replace('"', "\\\"");
                format!(r#"{{"type":"image","path":"{}","scaleMode":"{}","opacity":{:.2}}}"#, escaped, mode, opacity)
            }
        }
    }

    /// Set linear gradient fill on any node. Replaces existing fills.
    /// start/end are in 0..1 normalized coordinates (relative to node bounds).
    pub fn set_node_linear_gradient(
        &mut self, counter: u32, client_id: u32,
        start_x: f32, start_y: f32, end_x: f32, end_y: f32,
        stop_positions: Vec<f32>, stop_colors: Vec<f32>,
    ) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        let stops = Self::parse_gradient_stops(&stop_positions, &stop_colors);
        let old_fills = node.style.fills.clone();
        self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
        self.redo_stack.clear();
        node.style.fills = vec![Paint::LinearGradient {
            stops,
            start: Vec2::new(start_x, start_y),
            end: Vec2::new(end_x, end_y),
        }];
        self.mark_style_dirty(node_id);
        true
    }

    /// Set radial gradient fill on any node. Replaces existing fills.
    /// center is in 0..1 normalized coordinates. radius is 0..1 (1.0 = full extent).
    pub fn set_node_radial_gradient(
        &mut self, counter: u32, client_id: u32,
        center_x: f32, center_y: f32, radius: f32,
        stop_positions: Vec<f32>, stop_colors: Vec<f32>,
    ) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        let stops = Self::parse_gradient_stops(&stop_positions, &stop_colors);
        let old_fills = node.style.fills.clone();
        self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
        self.redo_stack.clear();
        node.style.fills = vec![Paint::RadialGradient {
            stops,
            center: Vec2::new(center_x, center_y),
            radius,
        }];
        self.mark_style_dirty(node_id);
        true
    }

    /// Set angular (conic) gradient fill on any node. Replaces existing fills.
    pub fn set_node_angular_gradient(
        &mut self, counter: u32, client_id: u32,
        center_x: f32, center_y: f32, start_angle: f32,
        stop_positions: Vec<f32>, stop_colors: Vec<f32>,
    ) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        let stops = Self::parse_gradient_stops(&stop_positions, &stop_colors);
        let old_fills = node.style.fills.clone();
        self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
        self.redo_stack.clear();
        node.style.fills = vec![Paint::AngularGradient {
            stops,
            center: Vec2::new(center_x, center_y),
            start_angle,
        }];
        self.mark_style_dirty(node_id);
        true
    }

    /// Append a solid fill to existing fills (for multiple fills per node).
    pub fn add_node_fill(&mut self, counter: u32, client_id: u32, r: f32, g: f32, b: f32, a: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        let old_fills = node.style.fills.clone();
        self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
        self.redo_stack.clear();
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));
        self.mark_style_dirty(node_id);
        true
    }

    /// Append a linear gradient fill to existing fills.
    pub fn add_node_linear_gradient(
        &mut self, counter: u32, client_id: u32,
        start_x: f32, start_y: f32, end_x: f32, end_y: f32,
        stop_positions: Vec<f32>, stop_colors: Vec<f32>,
    ) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        let stops = Self::parse_gradient_stops(&stop_positions, &stop_colors);
        let old_fills = node.style.fills.clone();
        self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
        self.redo_stack.clear();
        node.style.fills.push(Paint::LinearGradient {
            stops,
            start: Vec2::new(start_x, start_y),
            end: Vec2::new(end_x, end_y),
        });
        self.mark_style_dirty(node_id);
        true
    }

    /// Append a radial gradient fill to existing fills.
    pub fn add_node_radial_gradient(
        &mut self, counter: u32, client_id: u32,
        center_x: f32, center_y: f32, radius: f32,
        stop_positions: Vec<f32>, stop_colors: Vec<f32>,
    ) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        let stops = Self::parse_gradient_stops(&stop_positions, &stop_colors);
        let old_fills = node.style.fills.clone();
        self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
        self.redo_stack.clear();
        node.style.fills.push(Paint::RadialGradient {
            stops,
            center: Vec2::new(center_x, center_y),
            radius,
        });
        self.mark_style_dirty(node_id);
        true
    }

    /// Set node name.
    pub fn set_node_name(&mut self, counter: u32, client_id: u32, name: &str) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let old_name = node.name.clone();
        self.undo_stack.push(UndoAction::ChangeName { node_id, name: old_name });
        self.redo_stack.clear();
        node.name = name.to_string();
        // Name doesn't affect rendering — no scene rebuild needed
        self.needs_render = true;
        true
    }

    /// Set the text content of a text node.
    pub fn set_node_text(&mut self, counter: u32, client_id: u32, text: &str) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            let old_width = node.width;
            let old_height = node.height;
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: old_width, height: old_height,
            });
            self.redo_stack.clear();
            // Update text in first run, preserve styling
            if let Some(run) = runs.first_mut() {
                let font_size = run.font_size;
                run.text = text.to_string();
                // Recalculate size
                node.width = text.len() as f32 * font_size * 0.65;
                node.height = font_size * 1.5;
            }
            let tx = node.transform.tx;
            let ty = node.transform.ty;
            let w = node.width;
            let h = node.height;
            self.patch_scene_text(node_id);
            self.patch_scene_transform(node_id, tx, ty, Some(w), Some(h));
            true
        } else {
            false
        }
    }

    /// Set font size of a text node.
    pub fn set_node_font_size(&mut self, counter: u32, client_id: u32, size: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            let old_width = node.width;
            let old_height = node.height;
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: old_width, height: old_height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                run.font_size = size;
            }
            // Recalculate size based on first run
            if let Some(run) = runs.first() {
                node.width = run.text.len() as f32 * size * 0.65;
                node.height = size * 1.5;
            }
            let tx = node.transform.tx;
            let ty = node.transform.ty;
            let w = node.width;
            let h = node.height;
            self.patch_scene_text(node_id);
            self.patch_scene_transform(node_id, tx, ty, Some(w), Some(h));
            true
        } else {
            false
        }
    }

    /// Set font family on a text node (e.g. "Inter", "Roboto", "Poppins").
    pub fn set_node_font_family(&mut self, counter: u32, client_id: u32, family: &str) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: node.width, height: node.height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                run.font_family = family.to_string();
            }
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set font weight on a text node (300=Light, 400=Regular, 500=Medium, 600=Semibold, 700=Bold).
    pub fn set_node_font_weight(&mut self, counter: u32, client_id: u32, weight: u16) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: node.width, height: node.height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                run.font_weight = weight;
            }
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set letter spacing on a text node (in pixels).
    pub fn set_letter_spacing(&mut self, counter: u32, client_id: u32, spacing: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: node.width, height: node.height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                run.letter_spacing = spacing;
            }
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set line height on a text node (in pixels, 0 = auto).
    pub fn set_line_height(&mut self, counter: u32, client_id: u32, height: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: node.width, height: node.height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                run.line_height = if height > 0.0 { Some(height) } else { None };
            }
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set text decoration: "none", "underline", or "strikethrough".
    pub fn set_text_decoration(&mut self, counter: u32, client_id: u32, decoration: &str) -> bool {
        use rendero_core::node::TextDecoration;
        let dec = match decoration {
            "underline" => TextDecoration::Underline,
            "strikethrough" => TextDecoration::Strikethrough,
            _ => TextDecoration::None,
        };
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            let old_runs = runs.clone();
            self.undo_stack.push(UndoAction::ChangeText {
                node_id, runs: old_runs, width: node.width, height: node.height,
            });
            self.redo_stack.clear();
            for run in runs.iter_mut() {
                run.decoration = dec;
            }
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set text vertical alignment: "top", "center", or "bottom".
    pub fn set_text_vertical_align(&mut self, counter: u32, client_id: u32, align: &str) -> bool {
        use rendero_core::node::TextVerticalAlign;
        let va = match align {
            "center" => TextVerticalAlign::Center,
            "bottom" => TextVerticalAlign::Bottom,
            _ => TextVerticalAlign::Top,
        };
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut vertical_align, .. } = node.kind {
            *vertical_align = va;
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set text horizontal alignment: "left", "center", "right".
    pub fn set_text_align(&mut self, counter: u32, client_id: u32, align: &str) -> bool {
        use rendero_core::node::TextAlign;
        let ha = match align {
            "center" => TextAlign::Center,
            "right" => TextAlign::Right,
            _ => TextAlign::Left,
        };
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        if let NodeKind::Text { ref mut align, .. } = node.kind {
            *align = ha;
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set gradient fill on text (all runs). Type: "linear" or "radial".
    /// For linear: extra = [start_x, start_y, end_x, end_y]. For radial: extra = [center_x, center_y, radius].
    pub fn set_text_gradient_fill(
        &mut self, counter: u32, client_id: u32, gradient_type: &str,
        extra: Vec<f32>, stop_positions: Vec<f32>, stop_colors: Vec<f32>,
    ) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p, None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n, None => return false,
        };
        let stops = Self::parse_gradient_stops(&stop_positions, &stop_colors);
        let paint = match gradient_type {
            "linear" if extra.len() >= 4 => Paint::LinearGradient {
                stops,
                start: Vec2::new(extra[0], extra[1]),
                end: Vec2::new(extra[2], extra[3]),
            },
            "radial" if extra.len() >= 3 => Paint::RadialGradient {
                stops,
                center: Vec2::new(extra[0], extra[1]),
                radius: extra[2],
            },
            "angular" if extra.len() >= 3 => Paint::AngularGradient {
                stops,
                center: Vec2::new(extra[0], extra[1]),
                start_angle: extra[2],
            },
            _ => return false,
        };
        if let NodeKind::Text { ref mut runs, .. } = node.kind {
            for run in runs.iter_mut() {
                run.fill_override = Some(paint.clone());
            }
            self.patch_scene_text(node_id);
            self.needs_render = true;
            true
        } else {
            false
        }
    }

    /// Set text-on-arc parameters. radius=0 removes arc rendering.
    /// start_angle is in radians (−PI/2 = top of circle, PI/2 = bottom).
    pub fn set_text_arc(&mut self, counter: u32, client_id: u32, radius: f32, start_angle: f32, letter_spacing: f32) {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        if radius > 0.0 {
            self.text_arc_params.insert(node_id, (radius, start_angle, letter_spacing));
        } else {
            self.text_arc_params.remove(&node_id);
        }
        self.needs_render = true;
    }

    /// Get text-on-arc parameters for a node. Returns [radius, start_angle, letter_spacing] or empty.
    pub fn get_text_arc(&self, counter: u32, client_id: u32) -> Vec<f32> {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        match self.text_arc_params.get(&node_id) {
            Some(&(r, a, s)) => vec![r, a, s],
            None => vec![],
        }
    }

    /// Set stroke alignment: "inside", "center", or "outside".
    pub fn set_stroke_align(&mut self, counter: u32, client_id: u32, align: &str) -> bool {
        let sa = match align {
            "inside" => StrokeAlign::Inside,
            "outside" => StrokeAlign::Outside,
            _ => StrokeAlign::Center,
        };
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let old_fills = node.style.fills.clone();
        self.undo_stack.push(UndoAction::ChangeFill { node_id, fills: old_fills });
        self.redo_stack.clear();
        node.style.stroke_align = sa;
        self.mark_dirty();
        self.needs_render = true;
        true
    }

    /// Set corner radius on a rectangle or frame node.
    /// If all four values are the same, uses uniform radius. Otherwise per-corner.
    pub fn set_node_corner_radius(
        &mut self, counter: u32, client_id: u32,
        tl: f32, tr: f32, br: f32, bl: f32,
    ) -> bool {
        use rendero_core::node::CornerRadii;
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let radii = if tl == tr && tr == br && br == bl {
            CornerRadii::Uniform(tl)
        } else {
            CornerRadii::PerCorner { top_left: tl, top_right: tr, bottom_right: br, bottom_left: bl }
        };
        match &mut node.kind {
            NodeKind::Rectangle { corner_radii, .. } => {
                *corner_radii = radii;
                // Leaf property: update shape in scene cache in-place.
                self.patch_scene_corner_radii(node_id, radii);
                true
            }
            NodeKind::Frame { corner_radii, .. } => {
                *corner_radii = radii;
                // Leaf property: update shape in scene cache in-place.
                self.patch_scene_corner_radii(node_id, radii);
                true
            }
            _ => false,
        }
    }

    /// Add a rounded rectangle. Returns node ID as [counter, client_id].
    pub fn add_rounded_rect(
        &mut self, name: &str, x: f32, y: f32, width: f32, height: f32,
        r: f32, g: f32, b: f32, a: f32,
        radius: f32,
    ) -> Vec<u32> {
        use rendero_core::node::CornerRadii;
        let id = self.document.next_id();
        let mut node = Node::rectangle(id, name, width, height);
        node.transform = Transform::translate(x, y);
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));
        node.kind = NodeKind::Rectangle { corner_radii: CornerRadii::Uniform(radius) };

        let parent_id = self.effective_parent();
        let op_id = self.document.clock.next_op_id();
        let op = Operation {
            id: op_id,
            kind: OpKind::InsertNode {
                node: node.clone(),
                parent_id,
                position: FractionalIndex::end(),
            },
        };
        self.pending_ops.push(op);

        let node_for_undo = node.clone();
        self.scene_insert_leaf(&node, parent_id);
        self.document.add_node(self.current_page, node, parent_id, usize::MAX).expect("insert failed");
        self.undo_stack.push(UndoAction::AddNode { node: node_for_undo, parent_id });
        self.redo_stack.clear();
        vec![id.0.counter as u32, id.0.client_id]
    }

    /// Set blend mode on a node. mode: 0=Normal, 1=Multiply, 2=Screen, 3=Overlay,
    /// 4=Darken, 5=Lighten, 6=ColorDodge, 7=ColorBurn, 8=HardLight, 9=SoftLight,
    /// 10=Difference, 11=Exclusion, 12=Hue, 13=Saturation, 14=Color, 15=Luminosity.
    pub fn set_node_blend_mode(&mut self, counter: u32, client_id: u32, mode: u32) -> bool {
        use rendero_core::properties::BlendMode;
        let blend = match mode {
            0 => BlendMode::Normal,
            1 => BlendMode::Multiply,
            2 => BlendMode::Screen,
            3 => BlendMode::Overlay,
            4 => BlendMode::Darken,
            5 => BlendMode::Lighten,
            6 => BlendMode::ColorDodge,
            7 => BlendMode::ColorBurn,
            8 => BlendMode::HardLight,
            9 => BlendMode::SoftLight,
            10 => BlendMode::Difference,
            11 => BlendMode::Exclusion,
            12 => BlendMode::Hue,
            13 => BlendMode::Saturation,
            14 => BlendMode::ColorMode,
            15 => BlendMode::Luminosity,
            _ => return false,
        };
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.blend_mode = blend;
        self.mark_style_dirty(node_id);
        true
    }

    /// Set opacity on a node (0.0 to 1.0).
    pub fn set_node_opacity(&mut self, counter: u32, client_id: u32, opacity: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.opacity = opacity.clamp(0.0, 1.0);
        self.mark_style_dirty(node_id);
        true
    }

    /// Set or unset the mask flag on a node.
    /// When true, the node's shape clips all subsequent siblings until the parent ends.
    pub fn set_node_mask(&mut self, counter: u32, client_id: u32, is_mask: bool) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.is_mask = is_mask;
        // Structural: mask changes affect rendering of siblings, need scene rebuild
        // to recompute which items are masked.
        self.mark_dirty();
        true
    }

    /// Set stroke on a node (color + weight). Replaces all existing strokes.
    pub fn set_node_stroke(&mut self, counter: u32, client_id: u32, r: f32, g: f32, b: f32, a: f32, weight: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.strokes = vec![Paint::Solid(Color::new(r, g, b, a))];
        node.style.stroke_weight = weight;
        self.mark_style_dirty(node_id);
        true
    }

    /// Remove all strokes from a node.
    pub fn remove_node_stroke(&mut self, counter: u32, client_id: u32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.strokes.clear();
        node.style.stroke_weight = 0.0;
        self.mark_style_dirty(node_id);
        true
    }

    /// Add a drop shadow effect to a node.
    pub fn add_drop_shadow(&mut self, counter: u32, client_id: u32, r: f32, g: f32, b: f32, a: f32, ox: f32, oy: f32, blur: f32, spread: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.effects.push(Effect::DropShadow {
            color: Color::new(r, g, b, a),
            offset: glam::Vec2::new(ox, oy),
            blur_radius: blur,
            spread,
        });
        self.mark_style_dirty(node_id);
        true
    }

    /// Add an inner shadow effect to a node.
    pub fn add_inner_shadow(&mut self, counter: u32, client_id: u32, r: f32, g: f32, b: f32, a: f32, ox: f32, oy: f32, blur: f32, spread: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.effects.push(Effect::InnerShadow {
            color: Color::new(r, g, b, a),
            offset: glam::Vec2::new(ox, oy),
            blur_radius: blur,
            spread,
        });
        self.mark_style_dirty(node_id);
        true
    }

    /// Add a layer blur effect to a node.
    pub fn add_blur(&mut self, counter: u32, client_id: u32, radius: f32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.effects.push(Effect::LayerBlur { radius });
        self.mark_style_dirty(node_id);
        true
    }

    /// Set dash pattern on a node's stroke.
    pub fn set_dash_pattern(&mut self, counter: u32, client_id: u32, dashes: Vec<f32>) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.style.dash_pattern = dashes;
        self.mark_style_dirty(node_id);
        true
    }

    /// Set auto-layout on a frame node.
    /// direction: 0=Horizontal, 1=Vertical
    /// After setting, compute_layout is called to position children.
    pub fn set_auto_layout(
        &mut self, counter: u32, client_id: u32,
        direction: u32, spacing: f32,
        pad_top: f32, pad_right: f32, pad_bottom: f32, pad_left: f32,
    ) -> bool {
        use rendero_core::properties::{AutoLayout, LayoutDirection, SizingMode, LayoutAlign};
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        let dir = if direction == 0 { LayoutDirection::Horizontal } else { LayoutDirection::Vertical };
        let al = AutoLayout {
            direction: dir,
            spacing,
            padding_top: pad_top,
            padding_right: pad_right,
            padding_bottom: pad_bottom,
            padding_left: pad_left,
            primary_sizing: SizingMode::Fixed,
            counter_sizing: SizingMode::Fixed,
            align: LayoutAlign::Start,
        };
        match &mut node.kind {
            NodeKind::Frame { auto_layout, .. } => {
                *auto_layout = Some(al);
            }
            _ => return false,
        }
        // Run layout computation
        let root_id = page.tree.root_id();
        rendero_core::layout::compute_layout(&mut page.tree, &root_id);
        // Structural: auto-layout repositions/resizes multiple children recursively.
        // TODO(perf): compute_layout could return a list of moved nodes for incremental patching.
        self.mark_dirty();
        true
    }

    /// Remove auto-layout from a frame.
    pub fn remove_auto_layout(&mut self, counter: u32, client_id: u32) -> bool {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        match &mut node.kind {
            NodeKind::Frame { auto_layout, .. } => {
                *auto_layout = None;
            }
            _ => return false,
        }
        // Structural: removing auto-layout may revert children to absolute positions.
        // TODO(perf): if children don't actually move, this could be a no-op.
        self.mark_dirty();
        true
    }

    /// Set constraints on a node. h: 0=left, 1=right, 2=leftRight, 3=center, 4=scale
    /// v: 0=top, 1=bottom, 2=topBottom, 3=center, 4=scale
    pub fn set_node_constraints(&mut self, counter: u32, client_id: u32, h: u32, v: u32) -> bool {
        use rendero_core::properties::ConstraintType;
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page_mut(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let node = match page.tree.get_mut(&node_id) {
            Some(n) => n,
            None => return false,
        };
        node.constraint_horizontal = match h {
            0 => ConstraintType::Min,
            1 => ConstraintType::Max,
            2 => ConstraintType::MinMax,
            3 => ConstraintType::Center,
            4 => ConstraintType::Scale,
            _ => ConstraintType::Min,
        };
        node.constraint_vertical = match v {
            0 => ConstraintType::Min,
            1 => ConstraintType::Max,
            2 => ConstraintType::MinMax,
            3 => ConstraintType::Center,
            4 => ConstraintType::Scale,
            _ => ConstraintType::Min,
        };
        self.needs_render = true;
        true
    }

    // ─── Internal ───────────────────────────────────────────

    fn check_resize_handle(&self, x: f32, y: f32) -> Option<ResizeHandle> {
        let sel_id = *self.selected.first()?;
        let page = self.document.page(self.current_page)?;
        let node = page.tree.get(&sel_id)?;

        let (nx, ny) = self.node_world_pos(&sel_id);
        let nw = node.width;
        let nh = node.height;
        let threshold = 6.0 / self.cam_zoom; // constant screen-space size

        // Corner handles (checked first — they overlap edge handles)
        let corners = [
            (nx, ny, ResizeHandle::TopLeft),
            (nx + nw, ny, ResizeHandle::TopRight),
            (nx, ny + nh, ResizeHandle::BottomLeft),
            (nx + nw, ny + nh, ResizeHandle::BottomRight),
        ];
        for (hx, hy, handle) in corners {
            if (x - hx).abs() <= threshold && (y - hy).abs() <= threshold {
                return Some(handle);
            }
        }

        // Edge handles (midpoints)
        let edges = [
            (nx + nw / 2.0, ny, ResizeHandle::Top),
            (nx + nw, ny + nh / 2.0, ResizeHandle::Right),
            (nx + nw / 2.0, ny + nh, ResizeHandle::Bottom),
            (nx, ny + nh / 2.0, ResizeHandle::Left),
        ];
        for (hx, hy, handle) in edges {
            if (x - hx).abs() <= threshold && (y - hy).abs() <= threshold {
                return Some(handle);
            }
        }

        None
    }

    /// Check if mouse is in the rotation zone (just outside a corner handle).
    /// Returns true if the mouse is 6-18px (screen space) from any corner.
    fn check_rotation_zone(&self, x: f32, y: f32) -> bool {
        let sel_id = match self.selected.first() {
            Some(id) => *id,
            None => return false,
        };
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return false,
        };
        let _node = match page.tree.get(&sel_id) {
            Some(n) => n,
            None => return false,
        };

        let (nx, ny) = self.node_world_pos(&sel_id);
        let nw = _node.width;
        let nh = _node.height;
        let inner = 6.0 / self.cam_zoom;
        let outer = 18.0 / self.cam_zoom;

        let corners = [
            (nx, ny), (nx + nw, ny), (nx, ny + nh), (nx + nw, ny + nh),
        ];
        for (hx, hy) in corners {
            let dx = (x - hx).abs();
            let dy = (y - hy).abs();
            let dist = (dx * dx + dy * dy).sqrt();
            // Rotation zone: outside resize handle but within outer threshold
            if dist > inner && dist <= outer {
                return true;
            }
        }
        false
    }

    // ─── Comment system ─────────────────────────────────────────

    /// Add a comment at world position (x, y). Returns the comment ID.
    pub fn add_comment(&mut self, x: f32, y: f32, text: &str, author: &str) -> u32 {
        self.comment_counter += 1;
        let id = self.comment_counter;
        self.comments.push(Comment {
            id,
            x,
            y,
            text: text.to_string(),
            author: author.to_string(),
            timestamp: js_sys::Date::now(),
            resolved: false,
        });
        id
    }

    /// Get all comments as JSON array.
    pub fn get_comments(&self) -> String {
        let items: Vec<String> = self.comments.iter().map(|c| {
            let escaped_text = c.text.replace('\\', "\\\\").replace('"', "\\\"");
            let escaped_author = c.author.replace('\\', "\\\\").replace('"', "\\\"");
            format!(
                r#"{{"id":{},"x":{:.1},"y":{:.1},"text":"{}","author":"{}","timestamp":{:.0},"resolved":{}}}"#,
                c.id, c.x, c.y, escaped_text, escaped_author, c.timestamp, c.resolved
            )
        }).collect();
        format!("[{}]", items.join(","))
    }

    /// Resolve or unresolve a comment.
    pub fn resolve_comment(&mut self, comment_id: u32, resolved: bool) -> bool {
        if let Some(c) = self.comments.iter_mut().find(|c| c.id == comment_id) {
            c.resolved = resolved;
            true
        } else {
            false
        }
    }

    /// Delete a comment by ID.
    pub fn delete_comment(&mut self, comment_id: u32) -> bool {
        let before = self.comments.len();
        self.comments.retain(|c| c.id != comment_id);
        self.comments.len() < before
    }

    /// Get comment count.
    pub fn comment_count(&self) -> u32 {
        self.comments.len() as u32
    }

    // ─── Prototype interactions ─────────────────────────────────

    /// Add a prototype link from source node to target node.
    /// trigger: "click" | "hover" | "drag"
    /// animation: "instant" | "dissolve" | "slide"
    pub fn add_prototype_link(
        &mut self, src_counter: u32, src_client: u32,
        dst_counter: u32, dst_client: u32,
        trigger: &str, animation: &str,
    ) -> bool {
        let source_id = NodeId(rendero_core::id::LogicalClock { counter: src_counter as u64, client_id: src_client });
        let target_id = NodeId(rendero_core::id::LogicalClock { counter: dst_counter as u64, client_id: dst_client });
        self.prototype_links.push(PrototypeLink {
            source_id,
            target_id,
            trigger: trigger.to_string(),
            animation: animation.to_string(),
        });
        true
    }

    /// Get all prototype links as JSON array.
    pub fn get_prototype_links(&self) -> String {
        let items: Vec<String> = self.prototype_links.iter().map(|l| {
            format!(
                r#"{{"source":[{},{}],"target":[{},{}],"trigger":"{}","animation":"{}"}}"#,
                l.source_id.0.counter, l.source_id.0.client_id,
                l.target_id.0.counter, l.target_id.0.client_id,
                l.trigger, l.animation
            )
        }).collect();
        format!("[{}]", items.join(","))
    }

    /// Remove all prototype links from a source node.
    pub fn remove_prototype_links(&mut self, counter: u32, client_id: u32) -> bool {
        let source_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let before = self.prototype_links.len();
        self.prototype_links.retain(|l| l.source_id != source_id);
        self.prototype_links.len() < before
    }

    /// Get prototype link count.
    pub fn prototype_link_count(&self) -> u32 {
        self.prototype_links.len() as u32
    }

    // ─── Vector network ─────────────────────────────────────────

    /// Get vector network data for a vector node as JSON.
    /// Returns vertices + segments representation (graph-based, not sequential paths).
    /// This is the Figma vector network format: vertices share positions,
    /// segments connect pairs of vertices with bezier tangent handles.
    pub fn get_vector_network(&self, counter: u32, client_id: u32) -> String {
        let node_id = NodeId(rendero_core::id::LogicalClock { counter: counter as u64, client_id });
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return "{}".to_string(),
        };
        let node = match page.tree.get(&node_id) {
            Some(n) => n,
            None => return "{}".to_string(),
        };

        let paths = match &node.kind {
            NodeKind::Vector { paths } => paths,
            _ => return "{}".to_string(),
        };

        // Convert sequential PathCommands to vertex/segment network
        let mut vertices: Vec<String> = Vec::new();
        let mut segments: Vec<String> = Vec::new();
        let mut vertex_map: std::collections::HashMap<(i32, i32), usize> = std::collections::HashMap::new();

        let quantize = |v: f32| -> i32 { (v * 100.0).round() as i32 };

        let mut get_or_add_vertex = |x: f32, y: f32, verts: &mut Vec<String>, vmap: &mut std::collections::HashMap<(i32, i32), usize>| -> usize {
            let key = (quantize(x), quantize(y));
            if let Some(&idx) = vmap.get(&key) {
                idx
            } else {
                let idx = verts.len();
                verts.push(format!(r#"{{"x":{:.2},"y":{:.2}}}"#, x, y));
                vmap.insert(key, idx);
                idx
            }
        };

        for path in paths {
            let mut current_pos = Vec2::ZERO;
            let mut current_vertex: Option<usize> = None;

            for cmd in &path.commands {
                match cmd {
                    PathCommand::MoveTo(to) => {
                        current_pos = *to;
                        current_vertex = Some(get_or_add_vertex(to.x, to.y, &mut vertices, &mut vertex_map));
                    }
                    PathCommand::LineTo(to) => {
                        let from_idx = current_vertex.unwrap_or_else(|| get_or_add_vertex(current_pos.x, current_pos.y, &mut vertices, &mut vertex_map));
                        let to_idx = get_or_add_vertex(to.x, to.y, &mut vertices, &mut vertex_map);
                        segments.push(format!(
                            r#"{{"start":{{"vertex":{}}},"end":{{"vertex":{}}}}}"#,
                            from_idx, to_idx
                        ));
                        current_pos = *to;
                        current_vertex = Some(to_idx);
                    }
                    PathCommand::CubicTo { control1, control2, to } => {
                        let from_idx = current_vertex.unwrap_or_else(|| get_or_add_vertex(current_pos.x, current_pos.y, &mut vertices, &mut vertex_map));
                        let to_idx = get_or_add_vertex(to.x, to.y, &mut vertices, &mut vertex_map);
                        let dx1 = control1.x - current_pos.x;
                        let dy1 = control1.y - current_pos.y;
                        let dx2 = control2.x - to.x;
                        let dy2 = control2.y - to.y;
                        segments.push(format!(
                            r#"{{"start":{{"vertex":{},"dx":{:.2},"dy":{:.2}}},"end":{{"vertex":{},"dx":{:.2},"dy":{:.2}}}}}"#,
                            from_idx, dx1, dy1, to_idx, dx2, dy2
                        ));
                        current_pos = *to;
                        current_vertex = Some(to_idx);
                    }
                    PathCommand::QuadTo { control, to } => {
                        // Approximate as cubic
                        let from_idx = current_vertex.unwrap_or_else(|| get_or_add_vertex(current_pos.x, current_pos.y, &mut vertices, &mut vertex_map));
                        let to_idx = get_or_add_vertex(to.x, to.y, &mut vertices, &mut vertex_map);
                        segments.push(format!(
                            r#"{{"start":{{"vertex":{}}},"end":{{"vertex":{}}}}}"#,
                            from_idx, to_idx
                        ));
                        current_pos = *to;
                        current_vertex = Some(to_idx);
                    }
                    PathCommand::Close => {
                        // Close loops back to the move-to vertex
                    }
                }
            }
        }

        format!(
            r#"{{"vertices":[{}],"segments":[{}]}}"#,
            vertices.join(","), segments.join(",")
        )
    }
}

/// Draw a selection rectangle (blue outline) around a node at world position (wx, wy).
fn draw_selection_box(pixels: &mut [u8], width: u32, height: u32, wx: f32, wy: f32, nw: f32, nh: f32, cam_x: f32, cam_y: f32, cam_zoom: f32) {
    // Convert world coordinates to screen coordinates
    let x0 = ((wx - cam_x) * cam_zoom) as i32;
    let y0 = ((wy - cam_y) * cam_zoom) as i32;
    let x1 = ((wx + nw - cam_x) * cam_zoom) as i32;
    let y1 = ((wy + nh - cam_y) * cam_zoom) as i32;

    // Blue selection color
    let (r, g, b, a) = (66u8, 133u8, 244u8, 255u8);

    // Draw horizontal lines
    for x in x0.max(0)..x1.min(width as i32) {
        set_pixel_safe(pixels, width, height, x as u32, y0.max(0) as u32, r, g, b, a);
        set_pixel_safe(pixels, width, height, x as u32, (y1 - 1).max(0) as u32, r, g, b, a);
    }
    // Draw vertical lines
    for y in y0.max(0)..y1.min(height as i32) {
        set_pixel_safe(pixels, width, height, x0.max(0) as u32, y as u32, r, g, b, a);
        set_pixel_safe(pixels, width, height, (x1 - 1).max(0) as u32, y as u32, r, g, b, a);
    }

    // Draw corner handles (small squares)
    let handle_size = 4i32;
    let corners = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)];
    for (cx, cy) in corners {
        for dy in -handle_size..=handle_size {
            for dx in -handle_size..=handle_size {
                let px = (cx + dx).max(0) as u32;
                let py = (cy + dy).max(0) as u32;
                set_pixel_safe(pixels, width, height, px, py, r, g, b, a);
            }
        }
    }
}

fn set_pixel_safe(pixels: &mut [u8], width: u32, height: u32, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
    if x < width && y < height {
        let idx = ((y * width + x) * 4) as usize;
        if idx + 3 < pixels.len() {
            pixels[idx] = r;
            pixels[idx + 1] = g;
            pixels[idx + 2] = b;
            pixels[idx + 3] = a;
        }
    }
}
