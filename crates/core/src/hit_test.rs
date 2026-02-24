//! Hit testing — which node is at a given point?
//!
//! Used for selection, hover, and click handling.
//! Returns nodes in z-order (topmost first).

use glam::Vec2;

use crate::id::NodeId;
use crate::node::NodeKind;
use crate::properties::Transform;
use crate::tree::DocumentTree;

/// Result of a hit test.
#[derive(Debug, Clone)]
pub struct HitResult {
    pub node_id: NodeId,
    pub depth: u32,
}

/// Find all nodes at a world-space point, topmost first.
pub fn hit_test(tree: &DocumentTree, root: &NodeId, point: Vec2) -> Vec<HitResult> {
    let mut results = Vec::new();
    hit_test_recursive(tree, root, point, &Transform::IDENTITY, 0, &mut results);
    results.reverse(); // Topmost (last drawn) first
    results
}

fn hit_test_recursive(
    tree: &DocumentTree,
    node_id: &NodeId,
    point: Vec2,
    parent_transform: &Transform,
    depth: u32,
    results: &mut Vec<HitResult>,
) {
    let Some(node) = tree.get(node_id) else { return };
    if !node.visible { return; }

    let world_transform = node.transform.then(parent_transform);
    let local = world_transform.apply_inverse(point);

    // Check if point is inside this node's bounds
    let inside = local.x >= 0.0 && local.x <= node.width
        && local.y >= 0.0 && local.y <= node.height;

    if inside {
        // More precise check based on node kind
        let precise_hit = match &node.kind {
            NodeKind::Ellipse { .. } => {
                let cx = node.width / 2.0;
                let cy = node.height / 2.0;
                let dx = (local.x - cx) / (node.width / 2.0);
                let dy = (local.y - cy) / (node.height / 2.0);
                dx * dx + dy * dy <= 1.0
            }
            _ => true, // Rectangles, frames, etc. use the bounding box
        };

        if precise_hit {
            // Don't add containers with no fill (they're transparent)
            let has_visual = !node.style.fills.is_empty()
                || !node.style.strokes.is_empty()
                || !matches!(node.kind, NodeKind::Frame { .. } | NodeKind::Component);

            if has_visual {
                results.push(HitResult { node_id: *node_id, depth });
            }
        }
    }

    // Recurse into children — but skip if this is a clipping frame and point is outside.
    // For 100K artboards × 18 children, this turns O(1.8M) into O(100K).
    let clips = matches!(&node.kind, NodeKind::Frame { clip_content: true, .. });
    let skip_children = clips && !inside;
    if !skip_children {
        if let Some(children) = tree.children_of(node_id) {
            for child_id in children.iter() {
                hit_test_recursive(tree, child_id, point, &world_transform, depth + 1, results);
            }
        }
    }
}

/// Find the topmost node at a point. Most common operation.
pub fn hit_test_top(tree: &DocumentTree, root: &NodeId, point: Vec2) -> Option<NodeId> {
    hit_test(tree, root, point).first().map(|r| r.node_id)
}
