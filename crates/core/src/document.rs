//! Top-level document — wraps the tree with metadata and page management.

use serde::{Deserialize, Serialize};

use crate::id::{ClockGen, NodeId};
use crate::node::Node;
use crate::tree::{DocumentTree, TreeError};

/// A page in the document.
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub tree: DocumentTree,
}

/// The top-level document.
pub struct Document {
    pub name: String,
    pub pages: Vec<Page>,
    pub clock: ClockGen,
}

impl Document {
    /// Create a new document with one empty page.
    pub fn new(name: impl Into<String>, client_id: u32) -> Self {
        let mut clock = ClockGen::new(client_id);
        let page_id = clock.next_node_id();
        let page = Page {
            id: page_id,
            name: "Page 1".into(),
            tree: DocumentTree::new(),
        };
        Self {
            name: name.into(),
            pages: vec![page],
            clock,
        }
    }

    /// Add a new page.
    pub fn add_page(&mut self, name: impl Into<String>) -> NodeId {
        let page_id = self.clock.next_node_id();
        self.pages.push(Page {
            id: page_id,
            name: name.into(),
            tree: DocumentTree::new(),
        });
        page_id
    }

    /// Get a page by index.
    pub fn page(&self, index: usize) -> Option<&Page> {
        self.pages.get(index)
    }

    /// Get a mutable page by index.
    pub fn page_mut(&mut self, index: usize) -> Option<&mut Page> {
        self.pages.get_mut(index)
    }

    /// Add a node to a specific page.
    pub fn add_node(
        &mut self,
        page_index: usize,
        node: Node,
        parent_id: NodeId,
        child_index: usize,
    ) -> Result<(), TreeError> {
        let page = self.pages.get_mut(page_index).ok_or(TreeError::ParentNotFound)?;
        page.tree.insert(node, parent_id, child_index)
    }

    /// Generate a new unique node ID.
    pub fn next_id(&mut self) -> NodeId {
        self.clock.next_node_id()
    }
}
