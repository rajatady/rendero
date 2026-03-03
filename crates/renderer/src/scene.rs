//! Scene graph — flattened, render-ready representation of the document tree.
//!
//! The document tree is hierarchical (parent-child). The scene graph is flat:
//! a sorted list of render items with pre-computed world transforms and
//! bounding boxes. This is what the rasterizer actually consumes.
//!
//! Separation of document tree (editing) and scene graph (rendering) means:
//! - Editing doesn't pay rendering costs
//! - Rendering doesn't need to understand tree traversal
//! - Each can be optimized independently

use rendero_core::id::NodeId;
use rendero_core::node::{Node, NodeKind};
use rendero_core::properties::*;
use rendero_core::tree::DocumentTree;
use glam::Vec2;

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec2,
    pub max: Vec2,
}

impl AABB {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn from_size(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            min: Vec2::new(x, y),
            max: Vec2::new(x + w, y + h),
        }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    pub fn contains_point(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn intersect(&self, other: &AABB) -> AABB {
        AABB {
            min: Vec2::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Vec2::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        }
    }

    pub fn union(&self, other: &AABB) -> AABB {
        AABB {
            min: Vec2::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Vec2::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }
}

/// A render item — one thing to draw. Pre-computed for fast rendering.
#[derive(Debug, Clone)]
pub struct RenderItem {
    pub node_id: NodeId,
    /// World-space transform (parent transforms already applied).
    pub world_transform: Transform,
    /// World-space bounding box (for tile intersection tests).
    pub world_bounds: AABB,
    /// The node's visual style.
    pub style: Style,
    /// What to draw.
    pub shape: RenderShape,
    /// Z-order (lower = behind).
    pub z_index: u32,
    /// Whether this item clips its children.
    pub clips: bool,
    /// Number of descendant items that follow this one in the list.
    /// Used by Canvas 2D renderer for clip region management.
    pub descendant_count: usize,
    /// If true, this item acts as a mask — its shape clips subsequent siblings.
    pub is_mask: bool,
}

/// The renderable shape — derived from NodeKind.
/// This is simpler than NodeKind because we've resolved all the
/// editing-specific concerns (components, instances, overrides)
/// into concrete draw commands.
#[derive(Debug, Clone)]
pub enum RenderShape {
    Rect {
        width: f32,
        height: f32,
        corner_radii: rendero_core::node::CornerRadii,
    },
    Ellipse {
        width: f32,
        height: f32,
        arc_start: f32,
        arc_end: f32,
        inner_radius_ratio: f32,
    },
    Line {
        length: f32,
    },
    Path {
        commands: Vec<rendero_core::node::PathCommand>,
        fill_rule: FillRule,
    },
    Text {
        runs: Vec<rendero_core::node::TextRun>,
        width: f32,
        height: f32,
        align: rendero_core::node::TextAlign,
        vertical_align: rendero_core::node::TextVerticalAlign,
    },
    Image {
        width: f32,
        height: f32,
        /// RGBA pixel data from the source image.
        data: Vec<u8>,
        /// Source image dimensions for sampling.
        image_width: u32,
        image_height: u32,
    },
}

/// Build a scene graph from a document tree.
/// Flattens the hierarchy, computes world transforms, sorts by z-order.
pub fn build_scene(tree: &DocumentTree, root: &NodeId, viewport: &AABB) -> Vec<RenderItem> {
    let mut items = Vec::new();
    let mut z_counter = 0u32;

    build_scene_recursive(
        tree,
        root,
        &Transform::IDENTITY,
        viewport,
        &mut items,
        &mut z_counter,
    );

    items
}

fn build_scene_recursive(
    tree: &DocumentTree,
    node_id: &NodeId,
    parent_transform: &Transform,
    viewport: &AABB,
    items: &mut Vec<RenderItem>,
    z_counter: &mut u32,
) {
    let Some(node) = tree.get(node_id) else {
        return;
    };

    if !node.visible {
        return;
    }

    // Fast path for translation-only nodes (no rotation/scale): avoid matrix multiply
    let is_translation = node.transform.a == 1.0 && node.transform.b == 0.0
        && node.transform.c == 0.0 && node.transform.d == 1.0;
    let parent_is_identity = parent_transform.a == 1.0 && parent_transform.b == 0.0
        && parent_transform.c == 0.0 && parent_transform.d == 1.0
        && parent_transform.tx == 0.0 && parent_transform.ty == 0.0;

    let (world_transform, world_bounds) = if is_translation && parent_is_identity {
        // Simple translation: bounds = (tx, ty) to (tx+w, ty+h)
        let wx = node.transform.tx;
        let wy = node.transform.ty;
        (node.transform, AABB::new(Vec2::new(wx, wy), Vec2::new(wx + node.width, wy + node.height)))
    } else {
        let wt = node.transform.then(parent_transform);
        let c0 = wt.apply(Vec2::new(0.0, 0.0));
        let c1 = wt.apply(Vec2::new(node.width, 0.0));
        let c2 = wt.apply(Vec2::new(node.width, node.height));
        let c3 = wt.apply(Vec2::new(0.0, node.height));
        let min_x = c0.x.min(c1.x).min(c2.x).min(c3.x);
        let min_y = c0.y.min(c1.y).min(c2.y).min(c3.y);
        let max_x = c0.x.max(c1.x).max(c2.x).max(c3.x);
        let max_y = c0.y.max(c1.y).max(c2.y).max(c3.y);
        (wt, AABB::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)))
    };

    // Viewport culling — skip nodes entirely outside viewport
    if !world_bounds.intersects(viewport) {
        return;
    }

    // Convert NodeKind to RenderShape
    let (shape, clips) = match &node.kind {
        NodeKind::Frame { clip_content, corner_radii, .. } => (
            RenderShape::Rect {
                width: node.width,
                height: node.height,
                corner_radii: *corner_radii,
            },
            *clip_content,
        ),
        NodeKind::Rectangle { corner_radii } => (
            RenderShape::Rect {
                width: node.width,
                height: node.height,
                corner_radii: *corner_radii,
            },
            false,
        ),
        NodeKind::Ellipse { arc_start, arc_end, inner_radius_ratio } => (
            RenderShape::Ellipse {
                width: node.width,
                height: node.height,
                arc_start: *arc_start,
                arc_end: *arc_end,
                inner_radius_ratio: *inner_radius_ratio,
            },
            false,
        ),
        NodeKind::Line => (
            RenderShape::Line { length: node.width },
            false,
        ),
        NodeKind::Vector { paths } => {
            // Take fill rule from first path, commands from all paths
            let fill_rule = paths.first()
                .map(|p| p.fill_rule)
                .unwrap_or(FillRule::NonZero);
            let commands: Vec<_> = paths.iter()
                .flat_map(|p| p.commands.iter().cloned())
                .collect();
            (RenderShape::Path { commands, fill_rule }, false)
        }
        NodeKind::Polygon { .. } => {
            // TODO: convert polygon to path commands
            (RenderShape::Rect {
                width: node.width,
                height: node.height,
                corner_radii: rendero_core::node::CornerRadii::default(),
            }, false)
        }
        NodeKind::Text { runs, align, vertical_align, .. } => (
            RenderShape::Text {
                runs: runs.clone(),
                width: node.width,
                height: node.height,
                align: *align,
                vertical_align: *vertical_align,
            },
            false,
        ),
        NodeKind::BooleanOp { .. } => {
            // Compute boolean path from children
            if let Some(result) = rendero_core::boolean::compute_boolean(tree, node_id) {
                if result.commands.is_empty() {
                    return; // Empty intersection — nothing to render
                }
                (RenderShape::Path {
                    commands: result.commands,
                    fill_rule: result.fill_rule,
                }, false)
            } else {
                // Fallback: render as rect
                (RenderShape::Rect {
                    width: node.width,
                    height: node.height,
                    corner_radii: rendero_core::node::CornerRadii::default(),
                }, false)
            }
        }
        NodeKind::Component => (
            RenderShape::Rect {
                width: node.width,
                height: node.height,
                corner_radii: rendero_core::node::CornerRadii::default(),
            },
            true, // clip children like a frame
        ),
        NodeKind::Instance { .. } => (
            RenderShape::Rect {
                width: node.width,
                height: node.height,
                corner_radii: rendero_core::node::CornerRadii::default(),
            },
            true, // clip children like a frame
        ),
        NodeKind::Image { data, image_width, image_height } => (
            RenderShape::Image {
                width: node.width,
                height: node.height,
                data: data.clone(),
                image_width: *image_width,
                image_height: *image_height,
            },
            false, // Images don't contain children
        ),
    };

    // Add this item
    let z = *z_counter;
    *z_counter += 1;

    let my_index = items.len();
    items.push(RenderItem {
        node_id: *node_id,
        world_transform,
        world_bounds,
        style: node.style.clone(),
        shape,
        z_index: z,
        clips,
        descendant_count: 0,
        is_mask: node.is_mask,
    });

    // Recurse into children
    if let Some(children) = tree.children_of(node_id) {
        for child_id in children.iter() {
            build_scene_recursive(tree, child_id, &world_transform, viewport, items, z_counter);
        }
    }

    // Update descendant count
    items[my_index].descendant_count = items.len() - my_index - 1;
}
