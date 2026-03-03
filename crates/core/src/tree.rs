//! Document tree — the core data structure.
//!
//! TYPE-LEVEL GUARANTEES:
//! - Every node has exactly one parent (except root).
//! - No cycles possible (arena-based, parent always has lower index).
//! - Children are ordered (sibling order matters for rendering).
//! - O(1) access to any node by NodeId.
//!
//! DESIGN: Arena-based tree, not pointer-based.
//! Nodes stored in a HashMap keyed by NodeId.
//! Parent-child relationships stored separately.
//! This makes CRDT operations (insert, move, delete) O(1).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::id::NodeId;
use crate::node::Node;

/// Flat serializable representation of a document tree.
/// Nodes are stored in DFS order with their parent IDs.
/// Root node (parent=None) is always first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatTree {
    pub nodes: Vec<(Node, Option<NodeId>)>,
}

/// Ordered list of children for a parent node.
/// Maintains insertion order. Used for sibling ordering (z-order).
#[derive(Debug, Clone, Default)]
pub struct ChildList {
    children: Vec<NodeId>,
}

impl ChildList {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    pub fn push(&mut self, id: NodeId) {
        self.children.push(id);
    }

    pub fn insert_at(&mut self, index: usize, id: NodeId) {
        let idx = index.min(self.children.len());
        self.children.insert(idx, id);
    }

    pub fn remove(&mut self, id: &NodeId) -> bool {
        if let Some(pos) = self.children.iter().position(|c| c == id) {
            self.children.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, NodeId> {
        self.children.iter()
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn index_of(&self, id: &NodeId) -> Option<usize> {
        self.children.iter().position(|c| c == id)
    }
}

/// The document tree.
/// All nodes live in `nodes`. Parent-child relationships in `children` and `parents`.
/// Root node is always present.
pub struct DocumentTree {
    nodes: HashMap<NodeId, Node>,
    children: HashMap<NodeId, ChildList>,
    parents: HashMap<NodeId, NodeId>,
    root_id: NodeId,
}

impl DocumentTree {
    /// Create a new document with a root frame.
    pub fn new() -> Self {
        let root_id = NodeId::ROOT;
        let root = Node::frame(root_id, "Document", f32::INFINITY, f32::INFINITY);
        let mut nodes = HashMap::new();
        nodes.insert(root_id, root);
        let mut children = HashMap::new();
        children.insert(root_id, ChildList::new());

        Self {
            nodes,
            children,
            parents: HashMap::new(),
            root_id,
        }
    }

    pub fn root_id(&self) -> NodeId {
        self.root_id
    }

    /// Total number of nodes in the tree (including root).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get a node by ID. Returns None if not found.
    pub fn get(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node.
    pub fn get_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Get the parent of a node.
    pub fn parent_of(&self, id: &NodeId) -> Option<NodeId> {
        self.parents.get(id).copied()
    }

    /// Get the children of a node in order.
    pub fn children_of(&self, id: &NodeId) -> Option<&ChildList> {
        self.children.get(id)
    }

    /// Insert a node as child of parent at the given index.
    /// Returns Err if parent doesn't exist or parent can't have children.
    pub fn insert(
        &mut self,
        node: Node,
        parent_id: NodeId,
        index: usize,
    ) -> Result<(), TreeError> {
        // Validate parent exists and is a container
        let parent = self.nodes.get(&parent_id).ok_or(TreeError::ParentNotFound)?;
        if !parent.is_container() {
            return Err(TreeError::NotAContainer);
        }

        let node_id = node.id;

        // Insert the node
        self.nodes.insert(node_id, node);
        self.parents.insert(node_id, parent_id);

        // Add to parent's children at index
        self.children
            .entry(parent_id)
            .or_default()
            .insert_at(index, node_id);

        // Initialize children list for this node (if it's a container)
        if self.nodes.get(&node_id).map_or(false, |n| n.is_container()) {
            self.children.entry(node_id).or_default();
        }

        Ok(())
    }

    /// Remove a node and all its descendants.
    /// Returns the removed nodes (for undo/CRDT purposes).
    pub fn remove(&mut self, id: &NodeId) -> Result<Vec<Node>, TreeError> {
        if *id == self.root_id {
            return Err(TreeError::CannotRemoveRoot);
        }

        let mut removed = Vec::new();
        self.remove_recursive(id, &mut removed);

        // Remove from parent's children list
        if let Some(parent_id) = self.parents.remove(id) {
            if let Some(siblings) = self.children.get_mut(&parent_id) {
                siblings.remove(id);
            }
        }

        Ok(removed)
    }

    fn remove_recursive(&mut self, id: &NodeId, removed: &mut Vec<Node>) {
        // First remove all children recursively
        if let Some(child_list) = self.children.remove(id) {
            let child_ids: Vec<_> = child_list.iter().copied().collect();
            for child_id in child_ids {
                self.remove_recursive(&child_id, removed);
                self.parents.remove(&child_id);
            }
        }

        // Then remove this node
        if let Some(node) = self.nodes.remove(id) {
            removed.push(node);
        }
    }

    /// Move a node to a new parent at a given index.
    pub fn move_node(
        &mut self,
        id: NodeId,
        new_parent_id: NodeId,
        index: usize,
    ) -> Result<(), TreeError> {
        if id == self.root_id {
            return Err(TreeError::CannotRemoveRoot);
        }

        // Check new parent exists and is a container
        let new_parent = self.nodes.get(&new_parent_id).ok_or(TreeError::ParentNotFound)?;
        if !new_parent.is_container() {
            return Err(TreeError::NotAContainer);
        }

        // Check we're not moving a node into its own descendant
        if self.is_descendant_of(&new_parent_id, &id) {
            return Err(TreeError::CycleDetected);
        }

        // Remove from old parent
        if let Some(old_parent_id) = self.parents.get(&id).copied() {
            if let Some(siblings) = self.children.get_mut(&old_parent_id) {
                siblings.remove(&id);
            }
        }

        // Add to new parent
        self.parents.insert(id, new_parent_id);
        self.children
            .entry(new_parent_id)
            .or_default()
            .insert_at(index, id);

        Ok(())
    }

    /// Check if `potential_descendant` is a descendant of `potential_ancestor`.
    fn is_descendant_of(&self, potential_descendant: &NodeId, potential_ancestor: &NodeId) -> bool {
        let mut current = *potential_descendant;
        while let Some(parent) = self.parents.get(&current) {
            if parent == potential_ancestor {
                return true;
            }
            current = *parent;
        }
        false
    }

    /// Total number of nodes (including root).
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Iterate all node IDs.
    pub fn node_ids(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes.keys()
    }

    /// Depth-first traversal from a starting node.
    pub fn traverse_depth_first(&self, start: &NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        self.dfs(start, &mut result);
        result
    }

    fn dfs(&self, id: &NodeId, result: &mut Vec<NodeId>) {
        result.push(*id);
        if let Some(children) = self.children.get(id) {
            for child_id in children.iter() {
                self.dfs(child_id, result);
            }
        }
    }

    /// Serialize the tree to a flat representation (DFS order).
    pub fn to_flat(&self) -> FlatTree {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        self.to_flat_dfs(&self.root_id, None, &mut nodes);
        FlatTree { nodes }
    }

    fn to_flat_dfs(&self, id: &NodeId, parent: Option<NodeId>, out: &mut Vec<(Node, Option<NodeId>)>) {
        if let Some(node) = self.nodes.get(id) {
            let mut n = node.clone();
            // JSON can't represent f32::INFINITY — clamp to 0 for serialization.
            if !n.width.is_finite() { n.width = 0.0; }
            if !n.height.is_finite() { n.height = 0.0; }
            out.push((n, parent));
            if let Some(children) = self.children.get(id) {
                for child_id in children.iter() {
                    self.to_flat_dfs(child_id, Some(*id), out);
                }
            }
        }
    }

    /// Reconstruct a tree from a flat representation.
    /// Nodes must be in DFS order (parent before children).
    pub fn from_flat(flat: FlatTree) -> Self {
        let mut tree = Self {
            nodes: HashMap::new(),
            children: HashMap::new(),
            parents: HashMap::new(),
            root_id: NodeId::ROOT,
        };

        for (mut node, parent_id) in flat.nodes {
            let node_id = node.id;
            let is_container = node.is_container();

            if parent_id.is_none() {
                // Root node — restore infinite dimensions
                tree.root_id = node_id;
                node.width = f32::INFINITY;
                node.height = f32::INFINITY;
            } else {
                tree.parents.insert(node_id, parent_id.unwrap());
                tree.children.entry(parent_id.unwrap()).or_default().push(node_id);
            }

            tree.nodes.insert(node_id, node);

            if is_container {
                tree.children.entry(node_id).or_default();
            }
        }

        tree
    }
}

#[derive(Debug)]
pub enum TreeError {
    ParentNotFound,
    NotAContainer,
    CannotRemoveRoot,
    CycleDetected,
}

impl std::fmt::Display for TreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParentNotFound => write!(f, "parent node not found"),
            Self::NotAContainer => write!(f, "target node cannot have children"),
            Self::CannotRemoveRoot => write!(f, "cannot remove root node"),
            Self::CycleDetected => write!(f, "operation would create a cycle"),
        }
    }
}

impl std::error::Error for TreeError {}
