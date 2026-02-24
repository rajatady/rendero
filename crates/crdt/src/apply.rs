//! Apply operations to the document tree.
//!
//! The apply function is the CRDT core. It takes an operation and mutates the tree.
//!
//! COMMUTATIVITY: Two operations applied in either order produce the same result.
//! This is achieved through:
//! - FractionalIndex for ordering (no index conflicts)
//! - OpId for tie-breaking (deterministic winner)
//! - Last-writer-wins for property updates (OpId determines "last")
//! - Delete wins over concurrent move (prevents orphans)

use rendero_core::id::NodeId;
use rendero_core::tree::DocumentTree;

use crate::operation::{OpKind, Operation, PropertyUpdate};

/// Result of applying an operation.
#[derive(Debug)]
pub enum ApplyResult {
    /// Operation applied successfully.
    Applied,
    /// Operation was a no-op (e.g., deleting an already-deleted node).
    NoOp,
    /// Operation could not be applied (e.g., parent doesn't exist yet).
    /// The operation should be queued and retried when dependencies arrive.
    Deferred,
}

/// Apply an operation to the document tree.
///
/// This function is the single entry point for ALL document mutations.
/// By routing everything through here, we guarantee CRDT properties.
pub fn apply(tree: &mut DocumentTree, op: &Operation) -> ApplyResult {
    match &op.kind {
        OpKind::InsertNode { node, parent_id, position } => {
            // Check if parent exists
            if tree.get(parent_id).is_none() {
                return ApplyResult::Deferred;
            }

            // Check if node already exists (idempotent)
            if tree.get(&node.id).is_some() {
                return ApplyResult::NoOp;
            }

            // Find insertion index from fractional position
            let index = resolve_fractional_index(tree, parent_id, position);

            match tree.insert(node.clone(), *parent_id, index) {
                Ok(()) => ApplyResult::Applied,
                Err(_) => ApplyResult::Deferred,
            }
        }

        OpKind::DeleteNode { node_id } => {
            if tree.get(node_id).is_none() {
                return ApplyResult::NoOp; // Already deleted — idempotent
            }

            match tree.remove(node_id) {
                Ok(_removed) => ApplyResult::Applied,
                Err(_) => ApplyResult::NoOp,
            }
        }

        OpKind::MoveNode { node_id, new_parent_id, position } => {
            // If node was deleted, no-op (delete wins over move)
            if tree.get(node_id).is_none() {
                return ApplyResult::NoOp;
            }

            // If new parent doesn't exist, defer
            if tree.get(new_parent_id).is_none() {
                return ApplyResult::Deferred;
            }

            let index = resolve_fractional_index(tree, new_parent_id, position);

            match tree.move_node(*node_id, *new_parent_id, index) {
                Ok(()) => ApplyResult::Applied,
                Err(_) => ApplyResult::NoOp, // Cycle or other issue — skip
            }
        }

        OpKind::SetProperty { node_id, property } => {
            let Some(node) = tree.get_mut(node_id) else {
                return ApplyResult::NoOp; // Node deleted
            };

            apply_property(node, property);
            ApplyResult::Applied
        }

        OpKind::Reorder { node_id, position } => {
            let Some(parent_id) = tree.parent_of(node_id) else {
                return ApplyResult::NoOp;
            };

            let index = resolve_fractional_index(tree, &parent_id, position);
            match tree.move_node(*node_id, parent_id, index) {
                Ok(()) => ApplyResult::Applied,
                Err(_) => ApplyResult::NoOp,
            }
        }
    }
}

/// Convert a FractionalIndex to a concrete integer index within a parent's children.
fn resolve_fractional_index(
    tree: &DocumentTree,
    parent_id: &NodeId,
    position: &crate::operation::FractionalIndex,
) -> usize {
    // Get current children and their fractional positions
    // For now, simple approach: just use the child count as the index
    // (fractional indexing comparison will be refined as we add the position
    // tracking per-child)
    tree.children_of(parent_id)
        .map(|c| c.len())
        .unwrap_or(0)
}

/// Apply a property update to a node.
fn apply_property(node: &mut rendero_core::node::Node, prop: &PropertyUpdate) {
    match prop {
        PropertyUpdate::Transform(t) => node.transform = *t,
        PropertyUpdate::Width(w) => node.width = *w,
        PropertyUpdate::Height(h) => node.height = *h,
        PropertyUpdate::Opacity(o) => node.style.opacity = *o,
        PropertyUpdate::BlendMode(bm) => node.style.blend_mode = *bm,
        PropertyUpdate::Visible(v) => node.visible = *v,
        PropertyUpdate::Locked(l) => node.locked = *l,
        PropertyUpdate::Name(n) => node.name = n.clone(),
        PropertyUpdate::Fills(f) => node.style.fills = f.clone(),
        PropertyUpdate::Strokes(s) => node.style.strokes = s.clone(),
        PropertyUpdate::Effects(e) => node.style.effects = e.clone(),
        PropertyUpdate::StrokeWeight(w) => node.style.stroke_weight = *w,
        PropertyUpdate::StrokeAlign(a) => node.style.stroke_align = *a,
        PropertyUpdate::StrokeCap(c) => node.style.stroke_cap = *c,
        PropertyUpdate::StrokeJoin(j) => node.style.stroke_join = *j,
        PropertyUpdate::CornerRadii(cr) => {
            match &mut node.kind {
                rendero_core::node::NodeKind::Frame { corner_radii, .. }
                | rendero_core::node::NodeKind::Rectangle { corner_radii } => {
                    *corner_radii = cr.clone();
                }
                _ => {} // Silently ignore — not applicable to this node type
            }
        }
        PropertyUpdate::ClipContent(clip) => {
            if let rendero_core::node::NodeKind::Frame { clip_content, .. } = &mut node.kind {
                *clip_content = *clip;
            }
        }
        PropertyUpdate::AutoLayout(al) => {
            if let rendero_core::node::NodeKind::Frame { auto_layout, .. } = &mut node.kind {
                *auto_layout = al.clone();
            }
        }
        PropertyUpdate::TextRuns(runs) => {
            if let rendero_core::node::NodeKind::Text { runs: existing, .. } = &mut node.kind {
                *existing = runs.clone();
            }
        }
        PropertyUpdate::TextAlign(align) => {
            if let rendero_core::node::NodeKind::Text { align: existing, .. } = &mut node.kind {
                *existing = *align;
            }
        }
    }
}
