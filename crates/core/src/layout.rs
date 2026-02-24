//! Auto-layout engine — Figma's layout system.
//!
//! Processes the document tree and resolves auto-layout constraints.
//! Each frame with auto_layout set gets its children arranged.
//!
//! The algorithm is a single-pass top-down traversal:
//! 1. For each auto-layout frame, compute children sizes
//! 2. Arrange children along primary axis (horizontal or vertical)
//! 3. Apply alignment on counter axis
//! 4. Resolve hug/fill sizing

use crate::id::NodeId;
use crate::node::NodeKind;
use crate::properties::*;
use crate::tree::DocumentTree;

/// Run auto-layout on the entire tree from root.
pub fn compute_layout(tree: &mut DocumentTree, root: &NodeId) {
    // Collect nodes to process in depth-first order
    let traversal = tree.traverse_depth_first(root);

    // Process bottom-up (children before parents) for "hug" sizing
    for node_id in traversal.iter().rev() {
        layout_node(tree, node_id);
    }
}

fn layout_node(tree: &mut DocumentTree, node_id: &NodeId) {
    // Check if this node has auto-layout
    let auto_layout = {
        let Some(node) = tree.get(node_id) else { return };
        match &node.kind {
            NodeKind::Frame { auto_layout: Some(al), .. } => al.clone(),
            _ => return,
        }
    };

    // Get children
    let child_ids: Vec<NodeId> = match tree.children_of(node_id) {
        Some(children) => children.iter().copied().collect(),
        None => return,
    };

    if child_ids.is_empty() {
        // No children — apply hug sizing (shrink to padding)
        apply_hug_empty(tree, node_id, &auto_layout);
        return;
    }

    // Gather child sizes
    let mut child_sizes: Vec<(f32, f32)> = Vec::new();
    for cid in &child_ids {
        if let Some(child) = tree.get(cid) {
            if child.visible {
                child_sizes.push((child.width, child.height));
            } else {
                child_sizes.push((0.0, 0.0)); // invisible children take no space
            }
        }
    }

    let visible_count = child_ids.iter()
        .filter(|cid| tree.get(cid).map_or(false, |n| n.visible))
        .count();

    let total_spacing = if visible_count > 1 {
        auto_layout.spacing * (visible_count as f32 - 1.0)
    } else {
        0.0
    };

    // Compute container size (for hug mode)
    let parent_node = tree.get(node_id).unwrap();
    let container_w = parent_node.width;
    let container_h = parent_node.height;

    let content_w = container_w - auto_layout.padding_left - auto_layout.padding_right;
    let content_h = container_h - auto_layout.padding_top - auto_layout.padding_bottom;

    // Layout along primary axis
    let is_horizontal = matches!(auto_layout.direction, LayoutDirection::Horizontal);

    // First pass: compute fill children's sizes
    let total_fixed: f32;
    let fill_count: usize;

    if is_horizontal {
        total_fixed = child_sizes.iter()
            .zip(child_ids.iter())
            .filter(|(_, cid)| {
                tree.get(cid).map_or(false, |n| {
                    n.visible && !matches!(n.horizontal_sizing, SizingMode::Fill)
                })
            })
            .map(|((w, _), _)| w)
            .sum::<f32>();
        fill_count = child_ids.iter()
            .filter(|cid| tree.get(cid).map_or(false, |n| {
                n.visible && matches!(n.horizontal_sizing, SizingMode::Fill)
            }))
            .count();
    } else {
        total_fixed = child_sizes.iter()
            .zip(child_ids.iter())
            .filter(|(_, cid)| {
                tree.get(cid).map_or(false, |n| {
                    n.visible && !matches!(n.vertical_sizing, SizingMode::Fill)
                })
            })
            .map(|((_, h), _)| h)
            .sum::<f32>();
        fill_count = child_ids.iter()
            .filter(|cid| tree.get(cid).map_or(false, |n| {
                n.visible && matches!(n.vertical_sizing, SizingMode::Fill)
            }))
            .count();
    }

    let available = if is_horizontal { content_w } else { content_h };
    let fill_size = if fill_count > 0 {
        ((available - total_fixed - total_spacing) / fill_count as f32).max(0.0)
    } else {
        0.0
    };

    // Second pass: position children
    let mut cursor = if is_horizontal {
        auto_layout.padding_left
    } else {
        auto_layout.padding_top
    };

    let mut first_visible = true;
    for (i, cid) in child_ids.iter().enumerate() {
        let Some(child) = tree.get(cid) else { continue };
        if !child.visible { continue; }

        if !first_visible {
            cursor += auto_layout.spacing;
        }
        first_visible = false;

        // Resolve child size
        let (child_w, child_h) = if is_horizontal {
            let w = if matches!(child.horizontal_sizing, SizingMode::Fill) {
                fill_size
            } else {
                child.width
            };
            let h = if matches!(child.vertical_sizing, SizingMode::Fill) {
                content_h
            } else {
                child.height
            };
            (w, h)
        } else {
            let w = if matches!(child.horizontal_sizing, SizingMode::Fill) {
                content_w
            } else {
                child.width
            };
            let h = if matches!(child.vertical_sizing, SizingMode::Fill) {
                fill_size
            } else {
                child.height
            };
            (w, h)
        };

        // Compute position
        let (x, y) = if is_horizontal {
            let x = cursor;
            let y = match auto_layout.align {
                LayoutAlign::Start => auto_layout.padding_top,
                LayoutAlign::Center => auto_layout.padding_top + (content_h - child_h) / 2.0,
                LayoutAlign::End => auto_layout.padding_top + content_h - child_h,
                LayoutAlign::Stretch => auto_layout.padding_top,
            };
            cursor += child_w;
            (x, y)
        } else {
            let y = cursor;
            let x = match auto_layout.align {
                LayoutAlign::Start => auto_layout.padding_left,
                LayoutAlign::Center => auto_layout.padding_left + (content_w - child_w) / 2.0,
                LayoutAlign::End => auto_layout.padding_left + content_w - child_w,
                LayoutAlign::Stretch => auto_layout.padding_left,
            };
            cursor += child_h;
            (x, y)
        };

        // Apply
        if let Some(child) = tree.get_mut(cid) {
            child.transform.tx = x;
            child.transform.ty = y;
            child.width = child_w;
            child.height = if matches!(auto_layout.align, LayoutAlign::Stretch) && is_horizontal {
                content_h
            } else {
                child_h
            };
            if !is_horizontal && matches!(auto_layout.align, LayoutAlign::Stretch) {
                child.width = content_w;
            }
        }
    }

    // Apply hug sizing to parent
    let total_children_size = cursor - if is_horizontal {
        auto_layout.padding_left
    } else {
        auto_layout.padding_top
    };

    if let Some(parent) = tree.get_mut(node_id) {
        match (&auto_layout.primary_sizing, is_horizontal) {
            (SizingMode::Hug, true) => {
                parent.width = total_children_size + auto_layout.padding_left + auto_layout.padding_right;
            }
            (SizingMode::Hug, false) => {
                parent.height = total_children_size + auto_layout.padding_top + auto_layout.padding_bottom;
            }
            _ => {}
        }
    }
}

fn apply_hug_empty(tree: &mut DocumentTree, node_id: &NodeId, al: &AutoLayout) {
    if let Some(node) = tree.get_mut(node_id) {
        if matches!(al.primary_sizing, SizingMode::Hug) {
            match al.direction {
                LayoutDirection::Horizontal => {
                    node.width = al.padding_left + al.padding_right;
                }
                LayoutDirection::Vertical => {
                    node.height = al.padding_top + al.padding_bottom;
                }
            }
        }
    }
}
