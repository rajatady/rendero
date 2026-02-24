//! CRDT Operations — the atomic units of document mutation.
//!
//! Every change to the document is expressed as one of these operations.
//! The enum is exhaustive — the compiler forces every consumer to handle all variants.
//!
//! Operations carry their own ID (OpId) which provides:
//! - Global uniqueness (client_id + counter)
//! - Total ordering (for deterministic conflict resolution)
//! - Causal tracking (Lamport timestamps)

use rendero_core::id::{NodeId, OpId};
use rendero_core::node::{Node, NodeKind};
use rendero_core::properties::*;
use serde::{Deserialize, Serialize};

/// A CRDT operation. Exhaustive — compiler enforces all variants handled.
///
/// Each operation is self-contained: it carries all data needed to apply it.
/// No operation references external state. This makes them:
/// - Serializable (can be sent over the wire)
/// - Replayable (can be applied to any state)
/// - Commutative (order of application doesn't matter for final result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique ID for this operation. Provides total ordering.
    pub id: OpId,
    /// The actual mutation.
    pub kind: OpKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpKind {
    /// Insert a new node into the tree.
    InsertNode {
        /// The node to insert (complete, self-contained).
        node: Node,
        /// Parent to insert under.
        parent_id: NodeId,
        /// Position among siblings. Uses fractional indexing for commutativity.
        position: FractionalIndex,
    },

    /// Delete a node (and all its descendants).
    DeleteNode {
        node_id: NodeId,
    },

    /// Move a node to a new parent/position.
    MoveNode {
        node_id: NodeId,
        new_parent_id: NodeId,
        position: FractionalIndex,
    },

    /// Update a node's property.
    SetProperty {
        node_id: NodeId,
        property: PropertyUpdate,
    },

    /// Reorder a node among its siblings.
    Reorder {
        node_id: NodeId,
        position: FractionalIndex,
    },
}

/// Fractional index for ordering siblings without conflicts.
///
/// Instead of integer indices (which cause conflicts when two clients
/// insert at the same position), we use strings that can always be
/// bisected. "a" < "b" < "c", and between "a" and "b" we can always
/// create "am" or "an".
///
/// This makes InsertNode commutative: two inserts at "different fractional
/// positions" always produce a deterministic order regardless of which
/// is applied first.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FractionalIndex(pub String);

impl FractionalIndex {
    /// Generate an index at the start.
    pub fn start() -> Self {
        Self("A".into())
    }

    /// Generate an index at the end.
    pub fn end() -> Self {
        Self("z".into())
    }

    /// Generate an index between two existing indices.
    /// Always possible — string space is dense.
    pub fn between(left: &FractionalIndex, right: &FractionalIndex) -> Self {
        let l = &left.0;
        let r = &right.0;

        // Find first differing character
        let l_bytes = l.as_bytes();
        let r_bytes = r.as_bytes();
        let max_len = l_bytes.len().max(r_bytes.len());

        for i in 0..max_len {
            let lc = if i < l_bytes.len() { l_bytes[i] } else { b'A' };
            let rc = if i < r_bytes.len() { r_bytes[i] } else { b'z' };

            if rc - lc > 1 {
                // There's room between these characters
                let mid = lc + (rc - lc) / 2;
                let mut result = l[..i].to_string();
                result.push(mid as char);
                return Self(result);
            }
        }

        // Append a middle character to the left string
        Self(format!("{}N", l))
    }
}

/// A property update — what changed on a node.
/// Exhaustive enum — renderer must handle all property types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyUpdate {
    // Geometry
    Transform(Transform),
    Width(f32),
    Height(f32),

    // Visual
    Opacity(f32),
    BlendMode(BlendMode),
    Visible(bool),
    Locked(bool),
    Name(String),

    // Style arrays (replace entire array — simpler CRDT semantics)
    Fills(Vec<Paint>),
    Strokes(Vec<Paint>),
    Effects(Vec<Effect>),

    // Stroke properties
    StrokeWeight(f32),
    StrokeAlign(StrokeAlign),
    StrokeCap(StrokeCap),
    StrokeJoin(StrokeJoin),

    // Node-kind-specific updates
    CornerRadii(rendero_core::node::CornerRadii),
    ClipContent(bool),
    AutoLayout(Option<AutoLayout>),

    // Text updates
    TextRuns(Vec<rendero_core::node::TextRun>),
    TextAlign(rendero_core::node::TextAlign),
}
