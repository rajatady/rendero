//! Compositing — combines rendered tiles into the final image.
//!
//! Handles blend modes and opacity for overlapping elements.

use rendero_core::properties::BlendMode;

use crate::tile::TileBuffer;

/// Composite a source tile onto a destination tile using the given blend mode.
pub fn composite(dst: &mut TileBuffer, src: &TileBuffer, blend_mode: BlendMode) {
    debug_assert_eq!(dst.width, src.width);
    debug_assert_eq!(dst.height, src.height);

    let len = dst.pixels.len();
    let dst_pixels = &mut dst.pixels;
    let src_pixels = &src.pixels;

    // Fast path: Normal blend mode (source-over)
    if matches!(blend_mode, BlendMode::Normal) {
        for i in (0..len).step_by(4) {
            let sa = src_pixels[i + 3] as f32 / 255.0;
            if sa == 0.0 {
                continue;
            }
            let inv_sa = 1.0 - sa;
            dst_pixels[i] = (src_pixels[i] as f32 + dst_pixels[i] as f32 * inv_sa) as u8;
            dst_pixels[i + 1] = (src_pixels[i + 1] as f32 + dst_pixels[i + 1] as f32 * inv_sa) as u8;
            dst_pixels[i + 2] = (src_pixels[i + 2] as f32 + dst_pixels[i + 2] as f32 * inv_sa) as u8;
            dst_pixels[i + 3] = (src_pixels[i + 3] as f32 + dst_pixels[i + 3] as f32 * inv_sa) as u8;
        }
        return;
    }

    // Blend mode implementations
    for i in (0..len).step_by(4) {
        let sa = src_pixels[i + 3] as f32 / 255.0;
        if sa == 0.0 {
            continue;
        }

        let sr = src_pixels[i] as f32 / 255.0;
        let sg = src_pixels[i + 1] as f32 / 255.0;
        let sb = src_pixels[i + 2] as f32 / 255.0;
        let dr = dst_pixels[i] as f32 / 255.0;
        let dg = dst_pixels[i + 1] as f32 / 255.0;
        let db = dst_pixels[i + 2] as f32 / 255.0;

        let (br, bg, bb) = blend_channels(sr, sg, sb, dr, dg, db, blend_mode);

        let inv_sa = 1.0 - sa;
        dst_pixels[i] = ((br * sa + dr * inv_sa) * 255.0) as u8;
        dst_pixels[i + 1] = ((bg * sa + dg * inv_sa) * 255.0) as u8;
        dst_pixels[i + 2] = ((bb * sa + db * inv_sa) * 255.0) as u8;
        dst_pixels[i + 3] = ((sa + dst_pixels[i + 3] as f32 / 255.0 * inv_sa) * 255.0) as u8;
    }
}

/// Apply blend mode to RGB channels.
fn blend_channels(
    sr: f32, sg: f32, sb: f32,
    dr: f32, dg: f32, db: f32,
    mode: BlendMode,
) -> (f32, f32, f32) {
    match mode {
        BlendMode::Normal => (sr, sg, sb),
        BlendMode::Multiply => (sr * dr, sg * dg, sb * db),
        BlendMode::Screen => (
            1.0 - (1.0 - sr) * (1.0 - dr),
            1.0 - (1.0 - sg) * (1.0 - dg),
            1.0 - (1.0 - sb) * (1.0 - db),
        ),
        BlendMode::Overlay => (
            overlay_channel(dr, sr),
            overlay_channel(dg, sg),
            overlay_channel(db, sb),
        ),
        BlendMode::Darken => (sr.min(dr), sg.min(dg), sb.min(db)),
        BlendMode::Lighten => (sr.max(dr), sg.max(dg), sb.max(db)),
        BlendMode::ColorDodge => (
            dodge_channel(dr, sr),
            dodge_channel(dg, sg),
            dodge_channel(db, sb),
        ),
        BlendMode::ColorBurn => (
            burn_channel(dr, sr),
            burn_channel(dg, sg),
            burn_channel(db, sb),
        ),
        BlendMode::HardLight => (
            overlay_channel(sr, dr),
            overlay_channel(sg, dg),
            overlay_channel(sb, db),
        ),
        BlendMode::SoftLight => (
            soft_light_channel(dr, sr),
            soft_light_channel(dg, sg),
            soft_light_channel(db, sb),
        ),
        BlendMode::Difference => (
            (sr - dr).abs(),
            (sg - dg).abs(),
            (sb - db).abs(),
        ),
        BlendMode::Exclusion => (
            sr + dr - 2.0 * sr * dr,
            sg + dg - 2.0 * sg * dg,
            sb + db - 2.0 * sb * db,
        ),
        // HSL-based modes — simplified for now
        BlendMode::Hue | BlendMode::Saturation | BlendMode::ColorMode | BlendMode::Luminosity => {
            // TODO: proper HSL blend modes
            (sr, sg, sb)
        }
    }
}

#[inline]
fn overlay_channel(base: f32, blend: f32) -> f32 {
    if base < 0.5 {
        2.0 * base * blend
    } else {
        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
    }
}

#[inline]
fn dodge_channel(base: f32, blend: f32) -> f32 {
    if blend >= 1.0 { 1.0 } else { (base / (1.0 - blend)).min(1.0) }
}

#[inline]
fn burn_channel(base: f32, blend: f32) -> f32 {
    if blend <= 0.0 { 0.0 } else { 1.0 - ((1.0 - base) / blend).min(1.0) }
}

#[inline]
fn soft_light_channel(base: f32, blend: f32) -> f32 {
    if blend <= 0.5 {
        base - (1.0 - 2.0 * blend) * base * (1.0 - base)
    } else {
        let d = if base <= 0.25 {
            ((16.0 * base - 12.0) * base + 4.0) * base
        } else {
            base.sqrt()
        };
        base + (2.0 * blend - 1.0) * (d - base)
    }
}
