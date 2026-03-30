//! Auto-layout engine — provider-based architecture.
//!
//! The default `compute_layout()` uses TaffyLayout (CSS flexbox).
//! Use `compute_layout_with()` to swap in any LayoutEngine implementation.
//!
//! Implementations:
//!   - `TaffyLayout` (crate::taffy_layout) — full CSS flexbox via Taffy
//!   - `LegacyLayout` (this file) — the original basic auto-layout

use crate::id::NodeId;
use crate::node::NodeKind;
use crate::properties::*;
use crate::providers::{LayoutEngine, HeuristicTextMeasurer};
use crate::tree::DocumentTree;

/// Default entry point — uses TaffyLayout with heuristic text measurement.
/// Drop-in replacement: same signature as before, zero breaking changes.
pub fn compute_layout(tree: &mut DocumentTree, root: &NodeId) {
    let mut engine = crate::taffy_layout::TaffyLayout::new(HeuristicTextMeasurer);
    engine.compute(tree, root, (0.0, 0.0));
}

/// Layout with a custom engine — for testing, fallback, or mixing implementations.
pub fn compute_layout_with(
    engine: &mut dyn LayoutEngine,
    tree: &mut DocumentTree,
    root: &NodeId,
    viewport: (f32, f32),
) {
    engine.compute(tree, root, viewport);
}

// ═══════════════════════════════════════════════════════════════
// LegacyLayout — the original basic auto-layout, wrapped as a provider
// ═══════════════════════════════════════════════════════════════

/// The original single-pass layout engine. Kept as a fallback provider.
pub struct LegacyLayout;

impl LayoutEngine for LegacyLayout {
    fn compute(&mut self, tree: &mut DocumentTree, root: &NodeId, _viewport: (f32, f32)) {
        let traversal = tree.traverse_depth_first(root);
        for node_id in traversal.iter().rev() {
            layout_node(tree, node_id);
        }
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
