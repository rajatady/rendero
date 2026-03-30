//! Provider traits — abstract interfaces for swappable subsystems.
//!
//! Nothing in the engine depends on a concrete implementation.
//! Taffy, Parley, fontdb are implementations — not dependencies.

use crate::id::NodeId;
use crate::node::TextRun;
use crate::tree::DocumentTree;

/// Computes layout (position + size) for all nodes in a tree.
///
/// Implementations:
///   - `TaffyLayout` — full CSS flexbox via Taffy crate
///   - `LegacyLayout` — the original basic auto-layout
pub trait LayoutEngine {
    fn compute(&mut self, tree: &mut DocumentTree, root: &NodeId, viewport: (f32, f32));
}

/// Measures text dimensions for layout.
///
/// Called by layout engines to size text nodes before positioning.
/// The layout engine doesn't know how text is shaped — it asks this trait.
///
/// Implementations:
///   - `HeuristicTextMeasurer` — `width = len * fontSize * 0.65` (fast, inaccurate)
///   - Future: `ParleyTextMeasurer` — real shaping via rustybuzz
pub trait TextMeasurer {
    fn measure(&self, runs: &[TextRun], max_width: f32) -> (f32, f32);
}

/// The simplest text measurer — character count heuristic.
/// Zero dependencies. Always available. Used as default and fallback.
pub struct HeuristicTextMeasurer;

impl TextMeasurer for HeuristicTextMeasurer {
    fn measure(&self, runs: &[TextRun], _max_width: f32) -> (f32, f32) {
        if runs.is_empty() {
            return (0.0, 0.0);
        }
        let mut total_width = 0.0f32;
        let mut max_height = 0.0f32;
        for run in runs {
            total_width += run.text.len() as f32 * run.font_size * 0.65;
            max_height = max_height.max(run.font_size * 1.5);
        }
        (total_width, max_height)
    }
}
