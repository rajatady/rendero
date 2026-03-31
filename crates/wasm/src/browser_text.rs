//! Browser-backed text measurer for WASM.
//!
//! DELIBERATE EXCEPTION: This is the ONLY place in the engine where measurement
//! is delegated to the browser instead of using our own Rust-side implementation.
//!
//! WHY: On WASM, there is no access to system font files. The browser has fonts
//! loaded (system fonts, @font-face web fonts, Google Fonts) but provides no API
//! to extract raw font bytes. Parley/Fontique cannot resolve fonts without bytes.
//! The browser's canvas.measureText() uses the real loaded fonts and gives exact
//! measurements that match the DOM oracle.
//!
//! THIS PATTERN WILL NOT BE FOLLOWED ELSEWHERE. The goal is single-pass rendering
//! across all platforms. Font rendering on WASM will eventually use the same
//! Parley/fontdb pipeline as native, once we implement font loading (bundled fonts,
//! web font fetching, or wasm-compatible font resolution). At that point, this
//! module becomes dead code and is removed.
//!
//! On native: ParleyTextMeasurer (system fonts via Fontique) — the correct path.
//! On WASM: BrowserTextMeasurer (browser canvas) — temporary bridge.
//!
//! WHY THIS LIVES IN rendero-wasm, NOT rendero-text:
//! This measurer depends on wasm-bindgen and web-sys (browser-only APIs).
//! rendero-text is a platform-agnostic crate used by both native and WASM.
//! Putting browser dependencies there would either pollute the native build
//! or require cfg-gated feature flags. Since rendero-wasm already owns the
//! browser integration surface, this adapter lives here alongside it.

use rendero_core::node::TextRun;
use rendero_core::providers::{HeuristicTextMeasurer, TextMeasurer};
use wasm_bindgen::JsCast;

/// Text measurer that delegates to the browser's canvas.measureText().
///
/// See module-level docs for why this exists and why it's temporary.
pub struct BrowserTextMeasurer {
    ctx: web_sys::CanvasRenderingContext2d,
}

impl BrowserTextMeasurer {
    pub fn new() -> Option<Self> {
        let document = web_sys::window()?.document()?;
        let canvas = document.create_element("canvas").ok()?;
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into().ok()?;
        let ctx = canvas
            .get_context("2d")
            .ok()??
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()?;
        Some(Self { ctx })
    }

    /// Create a BrowserTextMeasurer, or fall back to heuristic if browser
    /// APIs are unavailable (e.g. during headless testing without DOM).
    pub fn unwrap_or_fallback() -> WasmTextMeasurer {
        match Self::new() {
            Some(m) => WasmTextMeasurer::Browser(m),
            None => WasmTextMeasurer::Heuristic,
        }
    }
}

/// Either browser-backed or heuristic text measurement.
/// This exists only to handle the Option from BrowserTextMeasurer::new().
pub enum WasmTextMeasurer {
    Browser(BrowserTextMeasurer),
    Heuristic,
}

impl TextMeasurer for WasmTextMeasurer {
    fn measure(&self, runs: &[TextRun], max_width: f32) -> (f32, f32) {
        match self {
            WasmTextMeasurer::Browser(m) => m.measure(runs, max_width),
            WasmTextMeasurer::Heuristic => HeuristicTextMeasurer.measure(runs, max_width),
        }
    }
}

impl TextMeasurer for BrowserTextMeasurer {
    fn measure(&self, runs: &[TextRun], max_width: f32) -> (f32, f32) {
        if runs.is_empty() {
            return (0.0, 0.0);
        }

        // Build font string and full text from runs.
        // For mixed-style runs, we measure each run separately and sum widths.
        // Height is the max line height across all runs.
        let wrap = max_width.is_finite() && max_width > 0.0;
        let mut total_width = 0.0f32;
        let mut max_line_height = 0.0f32;
        let mut line_width = 0.0f32;
        let mut total_height = 0.0f32;
        let mut line_count = 0u32;

        for run in runs {
            let weight = run.font_weight;
            let size = run.font_size;
            let family = if run.font_family.is_empty() {
                "-apple-system, system-ui, sans-serif"
            } else {
                &run.font_family
            };

            let font_str = format!("{weight} {size}px {family}");
            self.ctx.set_font(&font_str);

            let run_line_height = run.line_height.unwrap_or(size * 1.2).max(size);
            max_line_height = max_line_height.max(run_line_height);

            // Measure word by word for wrapping support
            if wrap {
                for word in run.text.split_inclusive(|c: char| c.is_whitespace() || c == '\n') {
                    if word.contains('\n') {
                        // Hard line break
                        total_width = total_width.max(line_width);
                        total_height += max_line_height;
                        line_width = 0.0;
                        line_count += 1;
                        // Measure non-newline part if any
                        let part = word.replace('\n', "");
                        if !part.is_empty() {
                            if let Ok(metrics) = self.ctx.measure_text(&part) {
                                line_width += metrics.width() as f32;
                            }
                        }
                        continue;
                    }

                    let word_width = self.ctx.measure_text(word)
                        .map(|m| m.width() as f32)
                        .unwrap_or(0.0);

                    if line_width > 0.0 && line_width + word_width > max_width {
                        total_width = total_width.max(line_width);
                        total_height += max_line_height;
                        line_width = 0.0;
                        line_count += 1;
                    }
                    line_width += word_width;
                }
            } else {
                // No wrapping — measure the whole run at once
                if let Ok(metrics) = self.ctx.measure_text(&run.text) {
                    line_width += metrics.width() as f32;
                }
            }
        }

        // Final line
        total_width = total_width.max(line_width);
        if line_width > 0.0 || line_count == 0 {
            total_height += max_line_height;
        }

        (total_width.ceil(), total_height.ceil())
    }
}
