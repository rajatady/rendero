//! Provider traits — abstract interfaces for swappable subsystems.
//!
//! Nothing in the engine depends on a concrete implementation.
//! Taffy, Parley, fontdb, lightningcss are implementations — not dependencies.

use serde::{Deserialize, Serialize};

use crate::id::NodeId;
use crate::node::TextRun;
use crate::properties::{LayoutAlign, LayoutDirection};
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

// ═══════════════════════════════════════════════════════════════
// CSS Parsing
// ═══════════════════════════════════════════════════════════════

/// CSS length unit — unresolved.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CssUnit {
    Px,
    Rem,
    Em,
    Vh,
    Vw,
    Percent,
}

/// CSS display value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CssDisplay {
    Flex,
    InlineFlex,
    Block,
    Inline,
    None,
}

/// CSS overflow value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CssOverflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

/// CSS position value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CssPosition {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

/// A gradient stop: color + position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssGradientStop {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub position: f32,
}

/// Parsed CSS value — engine-agnostic output of CssParser.
///
/// Both WASM and native consume this same type.
/// Kept flat (no nesting) for easy serialization to JS.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CssValue {
    /// RGBA color, components 0.0–1.0.
    Color { r: f32, g: f32, b: f32, a: f32 },

    /// Resolved length in pixels.
    Length(f32),

    /// Percentage (0.0–1.0), not yet resolved to px.
    Percentage(f32),

    /// Unresolved dimension (value + unit). Caller resolves against context.
    Dimension { value: f32, unit: CssUnit },

    /// Numeric value (flex-grow, opacity, font-weight, etc.)
    Number(f32),

    /// String keyword (font-family, text-align, etc.)
    Keyword(String),

    // ─── Layout ───

    Display(CssDisplay),
    Position(CssPosition),
    Overflow(CssOverflow),

    FlexDirection(LayoutDirection),
    FlexWrap(bool),
    AlignItems(LayoutAlign),
    JustifyContent(LayoutAlign),

    // ─── Compound ───

    Padding { top: f32, right: f32, bottom: f32, left: f32 },
    Margin { top: f32, right: f32, bottom: f32, left: f32 },
    BorderRadius { tl: f32, tr: f32, br: f32, bl: f32 },

    LinearGradient {
        start_x: f32, start_y: f32,
        end_x: f32, end_y: f32,
        stops: Vec<CssGradientStop>,
    },

    BoxShadow {
        ox: f32, oy: f32,
        blur: f32, spread: f32,
        r: f32, g: f32, b: f32, a: f32,
    },

    /// Property not recognized or value is `none`/`initial`.
    None,
}

/// Resolves a font family name to loaded font data.
///
/// Implementations:
///   - `FontiqueResolver` (rendero-text) — system font discovery via Fontique
///   - `EmbeddedFontResolver` (rendero-renderer) — single embedded font (RobotoMono)
///   - Future: `CoreTextResolver` — macOS native, `DirectWriteResolver` — Windows native
pub trait FontResolver: Send + Sync {
    /// Resolve a font family name + weight + italic to font file bytes.
    /// Returns None if no matching font is found.
    fn resolve(&self, family: &str, weight: u16, italic: bool) -> Option<Vec<u8>>;

    /// List available font families.
    fn families(&self) -> Vec<String>;
}

/// Rasterizes individual glyphs to pixel bitmaps.
///
/// Implementations:
///   - `FontdueRasterizer` (rendero-renderer) — CPU glyph rasterization via fontdue
///   - Future: `SwashRasterizer` — swash-based, `GpuRasterizer` — GPU text
pub trait GlyphRasterizer {
    /// Rasterize a single glyph. Returns (width, height, bitmap) where bitmap is alpha-only.
    fn rasterize(&self, font_data: &[u8], glyph_id: u16, size: f32) -> Option<(u32, u32, Vec<u8>)>;

    /// Get metrics for a character: (advance_width, height).
    fn measure_char(&self, font_data: &[u8], ch: char, size: f32) -> (f32, f32);
}

/// Parses CSS property values into engine-consumable types.
///
/// Implementations:
///   - `LightningCssParser` (rendero-css) — full CSS spec via lightningcss
///   - Future: `SimpleCssParser` — regex fallback for minimal targets
pub trait CssParser {
    /// Parse a single CSS property + value pair.
    ///
    /// `viewport` is (width, height) in pixels for resolving vh/vw/%.
    fn parse_property(&self, property: &str, value: &str, viewport: (f32, f32)) -> CssValue;

    /// Parse a full CSS declaration block (e.g. "display: flex; gap: 10px;").
    ///
    /// Returns a list of (property_name, parsed_value) pairs.
    /// Shorthands are expanded (e.g. "padding: 10px 20px" → 4 entries).
    fn parse_declarations(&self, css: &str, viewport: (f32, f32)) -> Vec<(String, CssValue)>;
}
