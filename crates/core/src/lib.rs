//! Figma Engine — Core document model and types.
//!
//! DESIGN PRINCIPLE: Make invalid states unrepresentable.
//! If it compiles, the document is structurally valid.

pub mod document;
pub mod node;
pub mod properties;
pub mod tree;
pub mod id;
pub mod hit_test;
pub mod layout;
pub mod boolean;
