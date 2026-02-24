//! Build tree structure from flat nodeChanges array.

use crate::error::{FigError, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Build a hierarchical tree from flat nodeChanges.
/// Returns the root node (guid "0:0") with children attached.
/// Takes ownership of the vec to avoid cloning (critical for large files like apple.fig 97MB).
pub fn build_tree(node_changes: Vec<JsonValue>) -> Result<JsonValue> {
    let mut nodes: HashMap<String, JsonValue> = HashMap::with_capacity(node_changes.len());
    let mut parent_to_children: HashMap<String, Vec<(String, String)>> = HashMap::new();

    // First pass: collect parent→child relationships (read-only)
    for node in &node_changes {
        if let Some(parent_index) = node.get("parentIndex") {
            let parent_guid = format_parent_guid(parent_index)?;
            let child_guid = format_guid(node)?;
            let position = parent_index
                .get("position")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            parent_to_children
                .entry(parent_guid)
                .or_default()
                .push((position, child_guid));
        }
    }

    // Second pass: take ownership of nodes (no clone!)
    for node in node_changes {
        let guid = format_guid(&node)?;
        nodes.insert(guid, node);
    }

    // Sort children by position
    for children in parent_to_children.values_mut() {
        children.sort_by(|a, b| a.0.cmp(&b.0));
    }

    build_node_tree("0:0", &mut nodes, &parent_to_children)
}

fn build_node_tree(
    guid: &str,
    nodes: &mut HashMap<String, JsonValue>,
    parent_to_children: &HashMap<String, Vec<(String, String)>>,
) -> Result<JsonValue> {
    let mut node = nodes
        .remove(guid)
        .ok_or_else(|| FigError::TreeError(format!("Node {} not found", guid)))?;

    if let Some(obj) = node.as_object_mut() {
        obj.remove("parentIndex");

        if let Some(child_entries) = parent_to_children.get(guid) {
            let mut children = Vec::new();
            for (_position, child_guid) in child_entries {
                let child_node = build_node_tree(child_guid, &mut *nodes, parent_to_children)?;
                children.push(child_node);
            }
            if !children.is_empty() {
                obj.insert("children".to_string(), JsonValue::Array(children));
            }
        }
    }

    Ok(node)
}

fn format_guid(node: &JsonValue) -> Result<String> {
    let guid_obj = node
        .get("guid")
        .and_then(|v| v.as_object())
        .ok_or_else(|| FigError::TreeError("Node missing guid field".into()))?;

    let session_id = guid_obj
        .get("sessionID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::TreeError("Invalid sessionID in guid".into()))?;

    let local_id = guid_obj
        .get("localID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::TreeError("Invalid localID in guid".into()))?;

    Ok(format!("{}:{}", session_id, local_id))
}

fn format_parent_guid(parent_index: &JsonValue) -> Result<String> {
    let guid_obj = parent_index
        .get("guid")
        .and_then(|v| v.as_object())
        .ok_or_else(|| FigError::TreeError("parentIndex missing guid field".into()))?;

    let session_id = guid_obj
        .get("sessionID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::TreeError("Invalid sessionID in parentIndex".into()))?;

    let local_id = guid_obj
        .get("localID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::TreeError("Invalid localID in parentIndex".into()))?;

    Ok(format!("{}:{}", session_id, local_id))
}
