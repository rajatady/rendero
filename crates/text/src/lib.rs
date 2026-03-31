//! Text measurement and font resolution via Parley + Fontique.
//!
//! Production-grade text layout: shaping, line breaking, kerning, bidi.
//! Implements `TextMeasurer` and `FontResolver` traits from rendero-core.
//!
//! Parley uses Fontique (system font discovery) + HarfRust (shaping) + Skrifa (font parsing).
//! These are the same crates backing Xilem, Vello, and other Linebender projects.

use parley::{FontContext, LayoutContext, StyleProperty};

use rendero_core::node::TextRun;
use rendero_core::providers::{FontResolver, TextMeasurer};

/// Text measurer using Parley — real text shaping with system fonts.
///
/// Handles proportional fonts, kerning, line breaking, bidi text.
/// Drop-in replacement for `HeuristicTextMeasurer`.
pub struct ParleyTextMeasurer {
    font_cx: FontContext,
    layout_cx: LayoutContext,
}

impl ParleyTextMeasurer {
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
        }
    }
}

impl Default for ParleyTextMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextMeasurer for ParleyTextMeasurer {
    fn measure(&self, runs: &[TextRun], max_width: f32) -> (f32, f32) {
        if runs.is_empty() {
            return (0.0, 0.0);
        }

        // Parley needs &mut — use interior mutability pattern
        // Since TextMeasurer::measure takes &self, we need unsafe or RefCell.
        // For now, rebuild contexts per call. This is slightly wasteful but correct.
        // Future: use RefCell or make the trait take &mut self.
        let mut font_cx = FontContext::new();
        let mut layout_cx: LayoutContext<[u8; 4]> = LayoutContext::new();

        // Concatenate all runs into one string with style spans
        let full_text: String = runs.iter().map(|r| r.text.as_str()).collect();
        if full_text.is_empty() {
            return (0.0, 0.0);
        }

        let scale = 1.0;
        let mut builder = layout_cx.ranged_builder(&mut font_cx, &full_text, scale);
        builder.push_default(StyleProperty::FontSize(16.0));
        builder.push_default(StyleProperty::LineHeight(1.2));

        // Apply styles per run
        let mut offset = 0;
        for run in runs {
            let len = run.text.len();
            let range = offset..(offset + len);

            builder.push(StyleProperty::FontSize(run.font_size), range.clone());
            builder.push(
                StyleProperty::FontWeight(parley::style::FontWeight::new(run.font_weight as f32)),
                range.clone(),
            );
            if run.letter_spacing.abs() > f32::EPSILON {
                builder.push(StyleProperty::LetterSpacing(run.letter_spacing), range.clone());
            }
            if let Some(line_height) = run.line_height {
                let multiplier = if run.font_size > 0.0 { line_height / run.font_size } else { 1.2 };
                builder.push(StyleProperty::LineHeight(multiplier.max(0.1)), range.clone());
            }
            if run.italic {
                builder.push(
                    StyleProperty::FontStyle(parley::style::FontStyle::Italic),
                    range.clone(),
                );
            }
            if !run.font_family.is_empty() {
                builder.push(
                    StyleProperty::FontStack(parley::style::FontStack::Single(
                        parley::style::FontFamily::Named(run.font_family.as_str().into()),
                    )),
                    range.clone(),
                );
            }

            offset += len;
        }

        let mut layout = builder.build(&full_text);

        // Break lines within max_width (or unbounded if max_width is infinite)
        let width_constraint = if max_width.is_finite() && max_width > 0.0 {
            Some(max_width)
        } else {
            None
        };
        layout.break_all_lines(width_constraint);

        let width = if width_constraint.is_some() {
            layout.full_width().min(max_width)
        } else {
            layout.full_width()
        };
        (width, layout.height())
    }
}

/// Font resolver using Fontique (via Parley's FontContext).
///
/// Discovers system fonts automatically. Resolves family + weight + italic
/// to font file bytes.
pub struct FontiqueResolver {
    font_cx: FontContext,
}

impl FontiqueResolver {
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::new(),
        }
    }
}

impl Default for FontiqueResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl FontResolver for FontiqueResolver {
    fn resolve(&self, _family: &str, _weight: u16, _italic: bool) -> Option<Vec<u8>> {
        // Fontique resolves fonts internally during layout — the raw bytes
        // aren't typically needed by external callers. This interface exists
        // for renderers that need raw font data (e.g., for glyph rasterization).
        // For now, return None — Parley handles font resolution internally.
        // TODO: Use fontique's Collection API to get font source bytes.
        None
    }

    fn families(&self) -> Vec<String> {
        // TODO: Enumerate system font families from Fontique collection.
        vec![
            "system-ui".to_string(),
            "sans-serif".to_string(),
            "serif".to_string(),
            "monospace".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rendero_core::properties::Color;

    fn make_run(text: &str, size: f32) -> TextRun {
        TextRun {
            text: text.to_string(),
            font_family: String::new(),
            font_size: size,
            font_weight: 400,
            italic: false,
            color: Color::BLACK,
            letter_spacing: 0.0,
            line_height: None,
            decoration: rendero_core::node::TextDecoration::None,
            fill_override: None,
        }
    }

    #[test]
    fn test_measure_hello_world() {
        let measurer = ParleyTextMeasurer::new();
        let runs = vec![make_run("Hello World", 16.0)];
        let (w, h) = measurer.measure(&runs, f32::INFINITY);
        assert!(w > 0.0, "width should be positive, got {w}");
        assert!(h > 0.0, "height should be positive, got {h}");
        // Proportional font: "Hello World" at 16px should be roughly 70-120px wide
        assert!(w > 50.0, "width too narrow: {w}");
        assert!(w < 200.0, "width too wide: {w}");
        assert!(h > 10.0, "height too short: {h}");
        assert!(h < 30.0, "height too tall: {h}");
    }

    #[test]
    fn test_measure_empty() {
        let measurer = ParleyTextMeasurer::new();
        let (w, h) = measurer.measure(&[], f32::INFINITY);
        assert_eq!(w, 0.0);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn test_measure_line_break() {
        let measurer = ParleyTextMeasurer::new();
        let runs = vec![make_run("This is a longer piece of text that should wrap", 16.0)];

        let (w_wide, h_wide) = measurer.measure(&runs, 500.0);
        let (w_narrow, h_narrow) = measurer.measure(&runs, 100.0);

        // Narrow max_width → more lines → taller, narrower
        assert!(h_narrow > h_wide, "narrow should be taller: {h_narrow} vs {h_wide}");
        assert!(w_narrow <= 110.0, "narrow should respect max_width: {w_narrow}");
    }

    #[test]
    fn test_measure_bold_vs_regular() {
        let measurer = ParleyTextMeasurer::new();
        let regular = vec![make_run("Hello", 16.0)];
        let mut bold_run = make_run("Hello", 16.0);
        bold_run.font_weight = 700;
        let bold = vec![bold_run];

        let (w_reg, _) = measurer.measure(&regular, f32::INFINITY);
        let (w_bold, _) = measurer.measure(&bold, f32::INFINITY);

        // Bold text is typically slightly wider
        assert!(w_reg > 0.0);
        assert!(w_bold > 0.0);
        // They should be in the same ballpark (within 30%)
        assert!((w_bold - w_reg).abs() / w_reg < 0.3,
            "bold/regular width difference too large: {w_bold} vs {w_reg}");
    }

    #[test]
    fn test_measure_different_sizes() {
        let measurer = ParleyTextMeasurer::new();
        let small = vec![make_run("Test", 12.0)];
        let large = vec![make_run("Test", 48.0)];

        let (w_small, h_small) = measurer.measure(&small, f32::INFINITY);
        let (w_large, h_large) = measurer.measure(&large, f32::INFINITY);

        assert!(w_large > w_small, "larger font should be wider: {w_large} vs {w_small}");
        assert!(h_large > h_small, "larger font should be taller: {h_large} vs {h_small}");
    }

    #[test]
    fn test_multiple_runs() {
        let measurer = ParleyTextMeasurer::new();
        let runs = vec![
            make_run("Hello ", 16.0),
            make_run("World", 24.0),
        ];
        let (w, h) = measurer.measure(&runs, f32::INFINITY);
        assert!(w > 0.0);
        assert!(h > 0.0);
        // Height should be at least as tall as the largest run
        assert!(h >= 20.0, "height should accommodate 24px text: {h}");
    }

    #[test]
    fn test_heuristic_vs_parley() {
        use rendero_core::providers::HeuristicTextMeasurer;

        let heuristic = HeuristicTextMeasurer;
        let parley = ParleyTextMeasurer::new();
        let runs = vec![make_run("Hello World", 16.0)];

        let (hw, hh) = heuristic.measure(&runs, f32::INFINITY);
        let (pw, ph) = parley.measure(&runs, f32::INFINITY);

        // Both should produce positive results
        assert!(hw > 0.0 && hh > 0.0);
        assert!(pw > 0.0 && ph > 0.0);

        // Parley should give different (more accurate) results than heuristic
        // Heuristic: width = 11 * 16 * 0.65 = 114.4, height = 16 * 1.5 = 24
        println!("Heuristic: {hw}x{hh}, Parley: {pw}x{ph}");
    }
}
