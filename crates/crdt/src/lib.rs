//! Figma CRDT — operation-based CRDT for collaborative document editing.
//!
//! DESIGN: Every edit to the document is expressed as an Operation.
//! Operations are:
//! - Commutative: apply(a, apply(b, state)) == apply(b, apply(a, state))
//! - Idempotent: apply(a, apply(a, state)) == apply(a, state)
//!
//! TYPE-LEVEL ENFORCEMENT:
//! - Operations are an exhaustive enum — every handler must handle all ops.
//! - OpId provides total ordering for deterministic conflict resolution.
//! - The apply function takes &mut DocumentTree — it cannot partially apply.
//!   It either succeeds completely or returns an error.

pub mod operation;
pub mod apply;
pub mod history;
