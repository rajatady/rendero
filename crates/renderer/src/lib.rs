//! Figma Renderer — tile-based software renderer.
//!
//! DESIGN: The renderer is a pipeline of typed stages:
//!   Document → SceneGraph → TileGrid → Pixels
//!
//! Each stage has a well-defined input and output type.
//! The pipeline is type-enforced: you cannot skip stages or feed
//! wrong data. If it compiles, the pipeline is structurally correct.
//!
//! PERFORMANCE STRATEGY:
//! - Tile-based: only re-render tiles that changed (dirty tracking)
//! - Cache-friendly: tiles fit in L1 cache (64x64 pixels = 16KB)
//! - Parallelizable: each tile is independent (no shared mutable state)
//! - SIMD-ready: pixel operations on contiguous memory

pub mod scene;
pub mod tile;
pub mod rasterize;
pub mod stroke;
pub mod text;
pub mod composite;
pub mod pipeline;
pub mod svg;
pub mod verify;
