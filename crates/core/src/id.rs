//! Unique identifiers for document elements.
//!
//! Every node, every property, every operation gets a globally unique ID.
//! IDs are the backbone of CRDT operations — they must be:
//! - Globally unique (no collisions across clients)
//! - Orderable (for deterministic conflict resolution)
//! - Cheap to create and compare
//!
//! TYPE-LEVEL GUARANTEE: NodeId and PropertyId are distinct types.
//! You cannot accidentally pass a PropertyId where a NodeId is expected.

use serde::{Deserialize, Serialize};

/// Lamport-like logical timestamp for ordering.
/// Combined with client_id, this gives total ordering across all clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LogicalClock {
    pub counter: u64,
    pub client_id: u32,
}

/// Unique identifier for a node in the document tree.
/// Wrapping in a newtype prevents mixing with other ID types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub LogicalClock);

/// Unique identifier for an operation (used by CRDT).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpId(pub LogicalClock);

impl NodeId {
    pub fn new(counter: u64, client_id: u32) -> Self {
        Self(LogicalClock { counter, client_id })
    }

    /// The root node has a well-known ID (0, 0).
    pub const ROOT: Self = Self(LogicalClock { counter: 0, client_id: 0 });
}

impl OpId {
    pub fn new(counter: u64, client_id: u32) -> Self {
        Self(LogicalClock { counter, client_id })
    }
}

/// Clock generator for a single client.
/// Monotonically increasing — cannot go backwards.
pub struct ClockGen {
    client_id: u32,
    counter: u64,
}

impl ClockGen {
    pub fn new(client_id: u32) -> Self {
        Self { client_id, counter: 0 }
    }

    /// Generate next ID. Always advances — cannot produce duplicates.
    pub fn next_node_id(&mut self) -> NodeId {
        self.counter += 1;
        NodeId::new(self.counter, self.client_id)
    }

    pub fn next_op_id(&mut self) -> OpId {
        self.counter += 1;
        OpId::new(self.counter, self.client_id)
    }

    /// Merge with a remote clock to maintain causal ordering.
    pub fn merge(&mut self, remote_counter: u64) {
        self.counter = self.counter.max(remote_counter);
    }

    /// Current counter value (for serialization).
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Client ID (for serialization).
    pub fn client_id(&self) -> u32 {
        self.client_id
    }

    /// Restore from saved state.
    pub fn from_parts(client_id: u32, counter: u64) -> Self {
        Self { client_id, counter }
    }
}
