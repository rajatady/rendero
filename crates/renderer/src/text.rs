//! Text layout and rasterization.
//!
//! Uses fontdue for glyph rasterization from an embedded TTF font.
//! Handles multi-run styled text with word wrapping and alignment.

use rendero_core::node::{TextAlign, TextRun, TextVerticalAlign};
use rendero_core::properties::{PremultColor, Transform};

use crate::tile::{TileBuffer, TileCoord, TILE_SIZE};

use fontdue::Font;
use glam::Vec2;
use std::sync::OnceLock;

/// Embedded font — Roboto Mono, compiled into the binary.
static DEFAULT_FONT: &[u8] = include_bytes!("../assets/RobotoMono.ttf");

static PARSED_FONT: OnceLock<Font> = OnceLock::new();

fn font() -> &'static Font {
    PARSED_FONT.get_or_init(|| {
        Font::from_bytes(DEFAULT_FONT, fontdue::FontSettings::default())
            .expect("embedded font is valid TTF")
    })
}

/// A positioned glyph ready for rasterization.
struct PlacedGlyph {
    bitmap: Vec<u8>,
    x: f32,
    y: f32,
    width: usize,
    height: usize,
    color: PremultColor,
}

struct Line {
    start: usize, // index into glyphs vec
    end: usize,
    width: f32,
    height: f32,
}

/// Lay out and rasterize styled text into a tile.
pub fn rasterize_text(
    tile: &mut TileBuffer,
    tile_coord: &TileCoord,
    runs: &[TextRun],
    text_width: f32,
    text_height: f32,
    align: TextAlign,
    vertical_align: TextVerticalAlign,
    world_transform: &Transform,
    opacity: f32,
) {
    if runs.is_empty() {
        return;
    }

    let font = font();

    // Phase 1: rasterize all glyphs and compute positions
    let mut glyphs: Vec<PlacedGlyph> = Vec::new();
    let mut lines: Vec<Line> = Vec::new();
    let mut cursor_x: f32 = 0.0;
    let mut line_start: usize = 0;
    let mut current_line_height: f32 = 0.0;

    for run in runs {
        let size = run.font_size;
        let color = run.color.premultiplied();
        let line_metrics = font.horizontal_line_metrics(size);
        let ascent = line_metrics.map(|m| m.ascent).unwrap_or(size * 0.8);
        let run_line_height = run.line_height.unwrap_or_else(|| {
            line_metrics.map(|m| m.new_line_size).unwrap_or(size * 1.2)
        });
        current_line_height = current_line_height.max(run_line_height);

        for ch in run.text.chars() {
            if ch == '\n' {
                lines.push(Line {
                    start: line_start,
                    end: glyphs.len(),
                    width: cursor_x,
                    height: current_line_height,
                });
                line_start = glyphs.len();
                cursor_x = 0.0;
                current_line_height = run_line_height;
                continue;
            }

            let (metrics, bitmap) = font.rasterize(ch, size);
            let advance = metrics.advance_width + run.letter_spacing;

            // Word wrap
            if cursor_x + advance > text_width && cursor_x > 0.0 && !ch.is_whitespace() {
                lines.push(Line {
                    start: line_start,
                    end: glyphs.len(),
                    width: cursor_x,
                    height: current_line_height,
                });
                line_start = glyphs.len();
                cursor_x = 0.0;
                current_line_height = run_line_height;
            }

            let gx = cursor_x + metrics.xmin as f32;
            let gy = ascent - metrics.ymin as f32 - metrics.height as f32;

            glyphs.push(PlacedGlyph {
                bitmap,
                x: gx,
                y: gy,
                width: metrics.width,
                height: metrics.height,
                color,
            });

            cursor_x += advance;
        }
    }

    // Final line
    if line_start < glyphs.len() || lines.is_empty() {
        lines.push(Line {
            start: line_start,
            end: glyphs.len(),
            width: cursor_x,
            height: current_line_height,
        });
    }

    // Phase 2: apply alignment offsets
    let total_height: f32 = lines.iter().map(|l| l.height).sum();
    let vert_offset = match vertical_align {
        TextVerticalAlign::Top => 0.0,
        TextVerticalAlign::Center => (text_height - total_height).max(0.0) / 2.0,
        TextVerticalAlign::Bottom => (text_height - total_height).max(0.0),
    };

    let mut line_y = vert_offset;
    for line in &lines {
        let horiz_offset = match align {
            TextAlign::Left | TextAlign::Justified => 0.0,
            TextAlign::Center => (text_width - line.width).max(0.0) / 2.0,
            TextAlign::Right => (text_width - line.width).max(0.0),
        };

        for glyph in &mut glyphs[line.start..line.end] {
            glyph.x += horiz_offset;
            glyph.y += line_y;
        }

        line_y += line.height;
    }

    // Phase 3: rasterize glyphs into tile
    let tile_x = (tile_coord.col * TILE_SIZE) as f32;
    let tile_y = (tile_coord.row * TILE_SIZE) as f32;
    let inv = world_transform.inverse().unwrap_or(Transform {
        a: 1.0, b: 0.0, c: 0.0, d: 1.0,
        tx: -world_transform.tx, ty: -world_transform.ty,
    });

    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let world_pt = Vec2::new(tile_x + px as f32 + 0.5, tile_y + py as f32 + 0.5);
            let local = inv.apply(world_pt);

            // Check each glyph (could optimize with spatial index, but correctness first)
            for glyph in &glyphs {
                if glyph.width == 0 || glyph.height == 0 {
                    continue;
                }

                let gx = local.x - glyph.x;
                let gy = local.y - glyph.y;

                if gx >= 0.0 && gx < glyph.width as f32
                    && gy >= 0.0 && gy < glyph.height as f32
                {
                    let gi = gy as usize * glyph.width + gx as usize;
                    if gi < glyph.bitmap.len() {
                        let alpha = glyph.bitmap[gi] as f32 / 255.0 * opacity;
                        if alpha > 0.001 {
                            let sr = (glyph.color.r * alpha * 255.0) as u8;
                            let sg = (glyph.color.g * alpha * 255.0) as u8;
                            let sb = (glyph.color.b * alpha * 255.0) as u8;
                            let sa = (glyph.color.a * alpha * 255.0) as u8;
                            tile.blend_pixel(px, py, sr, sg, sb, sa);
                        }
                    }
                }
            }
        }
    }
}
