//! Operation history with undo/redo.
//!
//! APPROACH: Store operations paired with their inverse.
//! Undo = apply the inverse. Redo = apply the original again.
//! This avoids needing to snapshot the full document state.

use rendero_core::id::OpId;
use crate::operation::{OpKind, Operation};

/// A recorded operation with its inverse for undo.
#[derive(Debug, Clone)]
struct HistoryEntry {
    op: Operation,
    inverse: Option<Operation>,
    is_local: bool,
}

pub struct History {
    entries: Vec<HistoryEntry>,
    /// Indices into entries for local ops that can be undone.
    undo_stack: Vec<usize>,
    /// Undone entries that can be redone.
    redo_stack: Vec<HistoryEntry>,
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Record an operation with its computed inverse.
    pub fn push(&mut self, op: Operation, inverse: Option<Operation>, is_local: bool) {
        let idx = self.entries.len();
        self.entries.push(HistoryEntry { op, inverse, is_local });
        if is_local {
            self.undo_stack.push(idx);
            self.redo_stack.clear();
        }
    }

    /// Get the next undo operation (the inverse of the last local op).
    /// Returns the inverse operation to apply.
    pub fn pop_undo(&mut self) -> Option<Operation> {
        let idx = self.undo_stack.pop()?;
        let entry = self.entries[idx].clone();
        self.redo_stack.push(entry.clone());
        entry.inverse
    }

    /// Get the next redo operation (re-apply the last undone op).
    pub fn pop_redo(&mut self) -> Option<Operation> {
        let entry = self.redo_stack.pop()?;
        let idx = self.entries.len();
        // Re-push to entries and undo stack
        self.entries.push(entry.clone());
        self.undo_stack.push(idx);
        Some(entry.op)
    }

    /// Can we undo?
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Can we redo?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get all operations after a given OpId (for sync).
    pub fn ops_after(&self, after: Option<OpId>) -> Vec<&Operation> {
        match after {
            None => self.entries.iter().map(|e| &e.op).collect(),
            Some(after_id) => {
                let pos = self.entries.iter().position(|e| e.op.id == after_id);
                match pos {
                    Some(idx) => self.entries[idx + 1..].iter().map(|e| &e.op).collect(),
                    None => self.entries.iter().map(|e| &e.op).collect(),
                }
            }
        }
    }

    pub fn last_op_id(&self) -> Option<OpId> {
        self.entries.last().map(|e| e.op.id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Compute the inverse of an operation given the current tree state.
/// Must be called BEFORE the operation is applied.
pub fn compute_inverse(
    op: &Operation,
    tree: &rendero_core::tree::DocumentTree,
    next_op_id: OpId,
) -> Option<Operation> {
    let inverse_kind = match &op.kind {
        OpKind::InsertNode { node, .. } => {
            // Inverse of insert is delete
            OpKind::DeleteNode { node_id: node.id }
        }
        OpKind::DeleteNode { node_id } => {
            // Inverse of delete is re-insert
            // We need the node data and its parent
            let node = tree.get(node_id)?.clone();
            let parent_id = tree.parent_of(node_id)?;
            let position = crate::operation::FractionalIndex::end();
            OpKind::InsertNode { node, parent_id, position }
        }
        OpKind::MoveNode { node_id, .. } => {
            // Inverse is moving back to original parent/position
            let parent_id = tree.parent_of(node_id)?;
            let position = crate::operation::FractionalIndex::end();
            OpKind::MoveNode {
                node_id: *node_id,
                new_parent_id: parent_id,
                position,
            }
        }
        OpKind::SetProperty { node_id, property } => {
            // Inverse is setting the old value
            let node = tree.get(node_id)?;
            let old_property = get_current_property(node, property);
            OpKind::SetProperty {
                node_id: *node_id,
                property: old_property,
            }
        }
        OpKind::Reorder { node_id, .. } => {
            // Just reorder back
            OpKind::Reorder {
                node_id: *node_id,
                position: crate::operation::FractionalIndex::end(),
            }
        }
    };

    Some(Operation {
        id: next_op_id,
        kind: inverse_kind,
    })
}

/// Extract the current value of a property from a node (for undo).
fn get_current_property(
    node: &rendero_core::node::Node,
    property: &crate::operation::PropertyUpdate,
) -> crate::operation::PropertyUpdate {
    use crate::operation::PropertyUpdate;

    match property {
        PropertyUpdate::Transform(_) => PropertyUpdate::Transform(node.transform),
        PropertyUpdate::Width(_) => PropertyUpdate::Width(node.width),
        PropertyUpdate::Height(_) => PropertyUpdate::Height(node.height),
        PropertyUpdate::Opacity(_) => PropertyUpdate::Opacity(node.style.opacity),
        PropertyUpdate::BlendMode(_) => PropertyUpdate::BlendMode(node.style.blend_mode),
        PropertyUpdate::Visible(_) => PropertyUpdate::Visible(node.visible),
        PropertyUpdate::Locked(_) => PropertyUpdate::Locked(node.locked),
        PropertyUpdate::Name(_) => PropertyUpdate::Name(node.name.clone()),
        PropertyUpdate::Fills(_) => PropertyUpdate::Fills(node.style.fills.clone()),
        PropertyUpdate::Strokes(_) => PropertyUpdate::Strokes(node.style.strokes.clone()),
        PropertyUpdate::Effects(_) => PropertyUpdate::Effects(node.style.effects.clone()),
        PropertyUpdate::StrokeWeight(_) => PropertyUpdate::StrokeWeight(node.style.stroke_weight),
        PropertyUpdate::StrokeAlign(_) => PropertyUpdate::StrokeAlign(node.style.stroke_align),
        PropertyUpdate::StrokeCap(_) => PropertyUpdate::StrokeCap(node.style.stroke_cap),
        PropertyUpdate::StrokeJoin(_) => PropertyUpdate::StrokeJoin(node.style.stroke_join),
        PropertyUpdate::CornerRadii(_) => {
            match &node.kind {
                rendero_core::node::NodeKind::Frame { corner_radii, .. }
                | rendero_core::node::NodeKind::Rectangle { corner_radii } => {
                    PropertyUpdate::CornerRadii(*corner_radii)
                }
                _ => PropertyUpdate::CornerRadii(rendero_core::node::CornerRadii::default()),
            }
        }
        PropertyUpdate::ClipContent(_) => {
            if let rendero_core::node::NodeKind::Frame { clip_content, .. } = &node.kind {
                PropertyUpdate::ClipContent(*clip_content)
            } else {
                PropertyUpdate::ClipContent(false)
            }
        }
        PropertyUpdate::AutoLayout(_) => {
            if let rendero_core::node::NodeKind::Frame { auto_layout, .. } = &node.kind {
                PropertyUpdate::AutoLayout(auto_layout.clone())
            } else {
                PropertyUpdate::AutoLayout(None)
            }
        }
        PropertyUpdate::TextRuns(_) => {
            if let rendero_core::node::NodeKind::Text { runs, .. } = &node.kind {
                PropertyUpdate::TextRuns(runs.clone())
            } else {
                PropertyUpdate::TextRuns(Vec::new())
            }
        }
        PropertyUpdate::TextAlign(_) => {
            if let rendero_core::node::NodeKind::Text { align, .. } = &node.kind {
                PropertyUpdate::TextAlign(*align)
            } else {
                PropertyUpdate::TextAlign(rendero_core::node::TextAlign::Left)
            }
        }
    }
}
