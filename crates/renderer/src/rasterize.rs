//! Rasterization — converts shapes to pixels within a tile.
//!
//! All functions are pure: tile buffer in, tile buffer out.
//! No global state. No allocations in hot path where avoidable.
//! Uses inverse affine transforms for all shapes (supports rotation/scale/skew).

use rendero_core::node::{CornerRadii, PathCommand};
use rendero_core::properties::{Color, Effect, FillRule, GradientStop, Paint, PremultColor, Transform};

use crate::scene::RenderShape;
use crate::tile::{TileBuffer, TileCoord, TILE_SIZE};
use glam::Vec2;

/// A color sampler resolves paint to a premultiplied color at a local-space point.
/// For solid colors this is constant. For gradients it varies per pixel.
enum ColorSampler<'a> {
    Solid(PremultColor),
    Linear {
        stops: &'a [GradientStop],
        start: Vec2,
        /// Precomputed: (end - start) / |end - start|^2
        dir_norm: Vec2,
    },
    Radial {
        stops: &'a [GradientStop],
        center: Vec2,
        inv_radius: f32,
    },
}

impl<'a> ColorSampler<'a> {
    fn from_paint(paint: &'a Paint, opacity: f32) -> Option<Self> {
        match paint {
            Paint::Solid(c) => {
                let p = c.premultiplied();
                Some(ColorSampler::Solid(PremultColor {
                    r: p.r * opacity, g: p.g * opacity,
                    b: p.b * opacity, a: p.a * opacity,
                }))
            }
            Paint::LinearGradient { stops, start, end } => {
                if stops.is_empty() { return None; }
                let d = *end - *start;
                let len_sq = d.dot(d);
                let dir_norm = if len_sq > 1e-10 { d / len_sq } else { Vec2::ZERO };
                Some(ColorSampler::Linear { stops, start: *start, dir_norm })
            }
            Paint::RadialGradient { stops, center, radius } => {
                if stops.is_empty() || *radius <= 0.0 { return None; }
                Some(ColorSampler::Radial {
                    stops, center: *center, inv_radius: 1.0 / *radius,
                })
            }
            Paint::Image { .. } => {
                // Image fills handled by Canvas 2D renderer, not rasterizer
                None
            }
        }
    }

    #[inline]
    fn sample(&self, local_x: f32, local_y: f32) -> PremultColor {
        match self {
            ColorSampler::Solid(c) => *c,
            ColorSampler::Linear { stops, start, dir_norm } => {
                let p = Vec2::new(local_x, local_y) - *start;
                let t = p.dot(*dir_norm).clamp(0.0, 1.0);
                sample_gradient(stops, t)
            }
            ColorSampler::Radial { stops, center, inv_radius } => {
                let d = Vec2::new(local_x - center.x, local_y - center.y);
                let t = (d.length() * *inv_radius).clamp(0.0, 1.0);
                sample_gradient(stops, t)
            }
        }
    }
}

/// Sample a gradient at position t (0..1) with linear interpolation.
fn sample_gradient(stops: &[GradientStop], t: f32) -> PremultColor {
    if stops.len() == 1 {
        return stops[0].color.premultiplied();
    }

    // Find the two stops that bracket t
    let mut i = 0;
    while i + 1 < stops.len() && stops[i + 1].position < t {
        i += 1;
    }
    if i + 1 >= stops.len() {
        return stops.last().unwrap().color.premultiplied();
    }

    let s0 = &stops[i];
    let s1 = &stops[i + 1];
    let range = s1.position - s0.position;
    let frac = if range > 1e-10 { (t - s0.position) / range } else { 0.0 };

    // Lerp in linear (premultiplied) space
    let c0 = s0.color.premultiplied();
    let c1 = s1.color.premultiplied();
    PremultColor {
        r: c0.r + (c1.r - c0.r) * frac,
        g: c0.g + (c1.g - c0.g) * frac,
        b: c0.b + (c1.b - c0.b) * frac,
        a: c0.a + (c1.a - c0.a) * frac,
    }
}

/// Full style info needed for rendering an item.
pub struct RenderStyle {
    pub fills: Vec<Paint>,
    pub strokes: Vec<Paint>,
    pub stroke_weight: f32,
    pub stroke_align: rendero_core::properties::StrokeAlign,
    pub stroke_cap: rendero_core::properties::StrokeCap,
    pub stroke_join: rendero_core::properties::StrokeJoin,
    pub opacity: f32,
}

/// Rasterize a render item into a tile (fills + strokes).
pub fn rasterize_item_styled(
    tile: &mut TileBuffer,
    tile_coord: &TileCoord,
    shape: &RenderShape,
    style: &RenderStyle,
    world_transform: &Transform,
) {
    // Render fills first
    rasterize_item(tile, tile_coord, shape, &style.fills, style.opacity, world_transform);

    // Render strokes on top
    if style.stroke_weight > 0.0 && !style.strokes.is_empty() {
        rasterize_stroke_internal(
            tile, tile_coord, shape, &style.strokes, style.opacity,
            world_transform, style.stroke_weight, style.stroke_align,
            style.stroke_cap, style.stroke_join,
        );
    }
}

/// Rasterize strokes by expanding to filled outline path.
fn rasterize_stroke_internal(
    tile: &mut TileBuffer,
    tile_coord: &TileCoord,
    shape: &RenderShape,
    strokes: &[Paint],
    opacity: f32,
    world_transform: &Transform,
    weight: f32,
    align: rendero_core::properties::StrokeAlign,
    cap: rendero_core::properties::StrokeCap,
    join: rendero_core::properties::StrokeJoin,
) {
    let path_cmds = shape_to_path_commands(shape);
    if path_cmds.is_empty() { return; }

    let outline = crate::stroke::expand_stroke(&path_cmds, weight, align, cap, join);
    if outline.is_empty() { return; }

    let stroke_shape = RenderShape::Path {
        commands: outline,
        fill_rule: FillRule::NonZero,
    };
    rasterize_item(tile, tile_coord, &stroke_shape, strokes, opacity, world_transform);
}

/// Convert any RenderShape to path commands (for stroke expansion).
fn shape_to_path_commands(shape: &RenderShape) -> Vec<PathCommand> {
    match shape {
        RenderShape::Rect { width, height, .. } => {
            vec![
                PathCommand::MoveTo(Vec2::new(0.0, 0.0)),
                PathCommand::LineTo(Vec2::new(*width, 0.0)),
                PathCommand::LineTo(Vec2::new(*width, *height)),
                PathCommand::LineTo(Vec2::new(0.0, *height)),
                PathCommand::Close,
            ]
        }
        RenderShape::Path { commands, .. } => commands.clone(),
        RenderShape::Ellipse { width, height, .. } => {
            // Approximate ellipse as bezier path
            let rx = *width / 2.0;
            let ry = *height / 2.0;
            let cx = rx;
            let cy = ry;
            let k = 0.5522847498; // magic number for cubic bezier circle approximation
            let kx = rx * k;
            let ky = ry * k;
            vec![
                PathCommand::MoveTo(Vec2::new(cx + rx, cy)),
                PathCommand::CubicTo {
                    control1: Vec2::new(cx + rx, cy + ky),
                    control2: Vec2::new(cx + kx, cy + ry),
                    to: Vec2::new(cx, cy + ry),
                },
                PathCommand::CubicTo {
                    control1: Vec2::new(cx - kx, cy + ry),
                    control2: Vec2::new(cx - rx, cy + ky),
                    to: Vec2::new(cx - rx, cy),
                },
                PathCommand::CubicTo {
                    control1: Vec2::new(cx - rx, cy - ky),
                    control2: Vec2::new(cx - kx, cy - ry),
                    to: Vec2::new(cx, cy - ry),
                },
                PathCommand::CubicTo {
                    control1: Vec2::new(cx + kx, cy - ry),
                    control2: Vec2::new(cx + rx, cy - ky),
                    to: Vec2::new(cx + rx, cy),
                },
                PathCommand::Close,
            ]
        }
        RenderShape::Line { length } => {
            vec![
                PathCommand::MoveTo(Vec2::ZERO),
                PathCommand::LineTo(Vec2::new(*length, 0.0)),
            ]
        }
        RenderShape::Text { .. } | RenderShape::Image { .. } => Vec::new(),
    }
}

/// Rasterize a render item into a tile.
pub fn rasterize_item(
    tile: &mut TileBuffer,
    tile_coord: &TileCoord,
    shape: &RenderShape,
    fills: &[Paint],
    opacity: f32,
    world_transform: &Transform,
) {
    // Text has its own color system (per-run), handle it before the fill check.
    if let RenderShape::Text { runs, width, height, align, vertical_align } = shape {
        crate::text::rasterize_text(
            tile, tile_coord, runs, *width, *height, *align, *vertical_align,
            world_transform, opacity,
        );
        return;
    }

    // Image has its own sampling, handle before fill check.
    if let RenderShape::Image { width, height, data, image_width, image_height } = shape {
        let tile_x = tile_coord.col * TILE_SIZE;
        let tile_y = tile_coord.row * TILE_SIZE;
        let inv = world_transform.inverse().unwrap_or(Transform {
            a: 1.0, b: 0.0, c: 0.0, d: 1.0,
            tx: -world_transform.tx, ty: -world_transform.ty,
        });
        rasterize_image(tile, tile_x, tile_y, &inv, *width, *height, data, *image_width, *image_height, opacity);
        return;
    }

    let paint = match fills.first() {
        Some(p) => p,
        None => return,
    };
    let sampler = match ColorSampler::from_paint(paint, opacity) {
        Some(s) => s,
        None => return,
    };

    let tile_x = tile_coord.col * TILE_SIZE;
    let tile_y = tile_coord.row * TILE_SIZE;

    // Precompute inverse transform for world→local mapping
    let inv = world_transform.inverse().unwrap_or(Transform {
        a: 1.0, b: 0.0, c: 0.0, d: 1.0,
        tx: -world_transform.tx, ty: -world_transform.ty,
    });

    match shape {
        RenderShape::Rect { width, height, corner_radii } => {
            rasterize_rect(tile, tile_x, tile_y, &inv, *width, *height, corner_radii, &sampler);
        }
        RenderShape::Ellipse { width, height, arc_start, arc_end, inner_radius_ratio } => {
            rasterize_ellipse(tile, tile_x, tile_y, &inv, *width, *height, &sampler);
        }
        RenderShape::Line { length } => {
            rasterize_line(tile, tile_x, tile_y, &inv, *length, &sampler);
        }
        RenderShape::Path { commands, fill_rule } => {
            rasterize_path(tile, tile_x, tile_y, &inv, commands, *fill_rule, &sampler);
        }
        RenderShape::Text { .. } => unreachable!("handled above"),
        RenderShape::Image { .. } => unreachable!("handled above"),
    }
}

/// Rasterize drop shadow effects for a shape.
/// Called BEFORE the item itself so the shadow appears behind.
pub fn rasterize_drop_shadows(
    tile: &mut TileBuffer,
    tile_coord: &TileCoord,
    shape: &RenderShape,
    effects: &[Effect],
    world_transform: &Transform,
) {
    for effect in effects {
        if let Effect::DropShadow { color, offset, blur_radius, spread } = effect {
            rasterize_one_shadow(
                tile, tile_coord, shape, world_transform,
                color, *offset, *blur_radius, *spread,
            );
        }
    }
}

fn rasterize_one_shadow(
    tile: &mut TileBuffer,
    tile_coord: &TileCoord,
    shape: &RenderShape,
    world_transform: &Transform,
    color: &Color,
    offset: Vec2,
    blur_radius: f32,
    spread: f32,
) {
    let tile_x = tile_coord.col * TILE_SIZE;
    let tile_y = tile_coord.row * TILE_SIZE;

    // Shadow transform = original + offset
    let shadow_transform = Transform {
        a: world_transform.a,
        b: world_transform.b,
        c: world_transform.c,
        d: world_transform.d,
        tx: world_transform.tx + offset.x,
        ty: world_transform.ty + offset.y,
    };

    let inv = shadow_transform.inverse().unwrap_or(Transform {
        a: 1.0, b: 0.0, c: 0.0, d: 1.0,
        tx: -shadow_transform.tx, ty: -shadow_transform.ty,
    });

    // Get shape dimensions (for signed distance computation)
    let (shape_w, shape_h) = match shape {
        RenderShape::Rect { width, height, .. } => (*width + spread * 2.0, *height + spread * 2.0),
        RenderShape::Ellipse { width, height, .. } => (*width + spread * 2.0, *height + spread * 2.0),
        _ => return, // Only support rect/ellipse shadows for now
    };

    let sigma = blur_radius / 2.0;
    let inv_2sigma2 = if sigma > 0.0 { 1.0 / (2.0 * sigma * sigma) } else { 0.0 };

    let cr = (color.r() * 255.0) as u8;
    let cg = (color.g() * 255.0) as u8;
    let cb = (color.b() * 255.0) as u8;
    let base_alpha = color.a();

    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let world_x = (tile_x + px) as f32 + 0.5;
            let world_y = (tile_y + py) as f32 + 0.5;
            let local = world_to_local(&inv, world_x, world_y);

            // Compute signed distance from shape edge (negative = inside)
            let dist = match shape {
                RenderShape::Rect { .. } => {
                    // Distance from rect [0..shape_w, 0..shape_h]
                    let dx = (local.x - shape_w * 0.5).abs() - shape_w * 0.5;
                    let dy = (local.y - shape_h * 0.5).abs() - shape_h * 0.5;
                    // Outside: euclidean distance. Inside: max(dx,dy) which is negative.
                    if dx > 0.0 && dy > 0.0 {
                        (dx * dx + dy * dy).sqrt()
                    } else {
                        dx.max(dy)
                    }
                }
                RenderShape::Ellipse { .. } => {
                    // Approximate: normalize to circle, compute distance
                    let cx = shape_w * 0.5;
                    let cy = shape_h * 0.5;
                    let nx = (local.x - cx) / cx;
                    let ny = (local.y - cy) / cy;
                    let r = (nx * nx + ny * ny).sqrt();
                    (r - 1.0) * cx.min(cy) // approximate signed distance
                }
                _ => continue,
            };

            // Gaussian falloff based on distance
            let alpha = if dist <= 0.0 {
                // Inside the shadow shape
                base_alpha
            } else if sigma > 0.0 {
                // Outside: Gaussian falloff
                base_alpha * (-dist * dist * inv_2sigma2).exp()
            } else {
                // No blur, sharp shadow
                0.0
            };

            if alpha < 0.004 { continue; } // threshold ~1/255
            let a = (alpha * 255.0) as u8;
            tile.blend_pixel(px, py, cr, cg, cb, a);
        }
    }
}

/// Convert world pixel coordinate to local space via inverse transform.
#[inline]
pub fn world_to_local(inv: &Transform, world_x: f32, world_y: f32) -> Vec2 {
    inv.apply(Vec2::new(world_x, world_y))
}

// ─── Image ──────────────────────────────────────────────────────────────

fn rasterize_image(
    tile: &mut TileBuffer,
    tile_x: u32,
    tile_y: u32,
    inv: &Transform,
    width: f32,
    height: f32,
    data: &[u8],
    image_width: u32,
    image_height: u32,
    opacity: f32,
) {
    if data.len() < (image_width * image_height * 4) as usize { return; }
    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let world_x = (tile_x + px) as f32 + 0.5;
            let world_y = (tile_y + py) as f32 + 0.5;
            let local = world_to_local(inv, world_x, world_y);
            if local.x < 0.0 || local.x >= width || local.y < 0.0 || local.y >= height { continue; }
            // Nearest-neighbor sample from source pixels
            let u = (local.x / width * image_width as f32) as u32;
            let v = (local.y / height * image_height as f32) as u32;
            let u = u.min(image_width - 1);
            let v = v.min(image_height - 1);
            let idx = ((v * image_width + u) * 4) as usize;
            let r = data[idx];
            let g = data[idx + 1];
            let b = data[idx + 2];
            let a = (data[idx + 3] as f32 * opacity) as u8;
            if a == 0 { continue; }
            tile.blend_pixel(px, py, r, g, b, a);
        }
    }
}

// ─── Rectangle ──────────────────────────────────────────────────────────

fn rasterize_rect(
    tile: &mut TileBuffer,
    tile_x: u32,
    tile_y: u32,
    inv: &Transform,
    width: f32,
    height: f32,
    corner_radii: &CornerRadii,
    sampler: &ColorSampler,
) {
    let radii = match corner_radii {
        CornerRadii::Uniform(r) => [*r, *r, *r, *r],
        CornerRadii::PerCorner { top_left, top_right, bottom_right, bottom_left } =>
            [*top_left, *top_right, *bottom_right, *bottom_left],
    };
    let has_radii = radii.iter().any(|&r| r > 0.0);

    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let world_x = (tile_x + px) as f32 + 0.5;
            let world_y = (tile_y + py) as f32 + 0.5;
            let local = world_to_local(inv, world_x, world_y);

            if local.x >= 0.0 && local.x <= width && local.y >= 0.0 && local.y <= height {
                let color = sampler.sample(local.x, local.y);
                let (r, g, b, a) = color_to_u8(&color);
                if a == 0 { continue; }

                if has_radii {
                    let coverage = rounded_rect_coverage(local.x, local.y, width, height, &radii);
                    if coverage <= 0.0 { continue; }
                    if coverage < 1.0 {
                        tile.blend_pixel(px, py, r, g, b, (a as f32 * coverage) as u8);
                        continue;
                    }
                }
                tile.blend_pixel(px, py, r, g, b, a);
            }
        }
    }
}

fn rounded_rect_coverage(x: f32, y: f32, w: f32, h: f32, radii: &[f32; 4]) -> f32 {
    // radii: [top_left, top_right, bottom_right, bottom_left]
    let r = if x < w / 2.0 {
        if y < h / 2.0 { radii[0] } else { radii[3] }
    } else {
        if y < h / 2.0 { radii[1] } else { radii[2] }
    };
    let r = r.min(w / 2.0).min(h / 2.0);
    if r <= 0.0 { return 1.0; }

    let corner_x = if x < r { r } else if x > w - r { w - r } else { x };
    let corner_y = if y < r { r } else if y > h - r { h - r } else { y };

    if (x < r || x > w - r) && (y < r || y > h - r) {
        let dx = x - corner_x;
        let dy = y - corner_y;
        let dist = (dx * dx + dy * dy).sqrt();
        (r - dist).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

// ─── Ellipse ────────────────────────────────────────────────────────────

fn rasterize_ellipse(
    tile: &mut TileBuffer,
    tile_x: u32,
    tile_y: u32,
    inv: &Transform,
    width: f32,
    height: f32,
    sampler: &ColorSampler,
) {
    let cx = width / 2.0;
    let cy = height / 2.0;
    let rx = width / 2.0;
    let ry = height / 2.0;

    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let local = world_to_local(inv, (tile_x + px) as f32 + 0.5, (tile_y + py) as f32 + 0.5);
            let dx = (local.x - cx) / rx;
            let dy = (local.y - cy) / ry;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= 1.0 {
                let color = sampler.sample(local.x, local.y);
                let (r, g, b, a) = color_to_u8(&color);
                if a == 0 { continue; }

                let dist = dist_sq.sqrt();
                if dist > 0.95 {
                    let coverage = ((1.0 - dist) / 0.05).clamp(0.0, 1.0);
                    tile.blend_pixel(px, py, r, g, b, (a as f32 * coverage) as u8);
                } else {
                    tile.blend_pixel(px, py, r, g, b, a);
                }
            }
        }
    }
}

// ─── Line ───────────────────────────────────────────────────────────────

fn rasterize_line(
    tile: &mut TileBuffer,
    tile_x: u32,
    tile_y: u32,
    inv: &Transform,
    length: f32,
    sampler: &ColorSampler,
) {
    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let local = world_to_local(inv, (tile_x + px) as f32 + 0.5, (tile_y + py) as f32 + 0.5);
            if local.x >= 0.0 && local.x <= length && local.y.abs() < 1.0 {
                let color = sampler.sample(local.x, local.y);
                let (r, g, b, a) = color_to_u8(&color);
                if a == 0 { continue; }
                let coverage = (1.0 - local.y.abs()).clamp(0.0, 1.0);
                tile.blend_pixel(px, py, r, g, b, (a as f32 * coverage) as u8);
            }
        }
    }
}

// ─── Path (scanline fill) ───────────────────────────────────────────────

fn rasterize_path(
    tile: &mut TileBuffer,
    tile_x: u32,
    tile_y: u32,
    inv: &Transform,
    commands: &[PathCommand],
    fill_rule: FillRule,
    sampler: &ColorSampler,
) {
    // Flatten bezier curves to line segments for scanline fill
    let segments = flatten_path(commands);
    if segments.is_empty() { return; }

    for py in 0..TILE_SIZE {
        for px in 0..TILE_SIZE {
            let local = world_to_local(inv, (tile_x + px) as f32 + 0.5, (tile_y + py) as f32 + 0.5);

            let winding = compute_winding(&segments, local);
            let inside = match fill_rule {
                FillRule::NonZero => winding != 0,
                FillRule::EvenOdd => (winding & 1) != 0,
            };

            if inside {
                let color = sampler.sample(local.x, local.y);
                let (r, g, b, a) = color_to_u8(&color);
                if a == 0 { continue; }

                // Anti-aliasing: compute signed distance to nearest edge
                let dist = signed_distance_to_path(&segments, local);
                if dist < 1.0 {
                    let coverage = dist.clamp(0.0, 1.0);
                    tile.blend_pixel(px, py, r, g, b, (a as f32 * coverage) as u8);
                } else {
                    tile.blend_pixel(px, py, r, g, b, a);
                }
            }
        }
    }
}

/// A line segment for scanline processing.
#[derive(Debug, Clone, Copy)]
struct Segment {
    pub from: Vec2,
    pub to: Vec2,
}

/// Flatten all path commands into line segments.
/// Bezier curves are approximated by subdividing until flat enough.
pub fn flatten_path(commands: &[PathCommand]) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current = Vec2::ZERO;
    let mut subpath_start = Vec2::ZERO;

    for cmd in commands {
        match cmd {
            PathCommand::MoveTo(p) => {
                current = *p;
                subpath_start = *p;
            }
            PathCommand::LineTo(p) => {
                segments.push(Segment { from: current, to: *p });
                current = *p;
            }
            PathCommand::CubicTo { control1, control2, to } => {
                flatten_cubic(&mut segments, current, *control1, *control2, *to, 0);
                current = *to;
            }
            PathCommand::QuadTo { control, to } => {
                // Convert quadratic to cubic
                let c1 = current + (2.0 / 3.0) * (*control - current);
                let c2 = *to + (2.0 / 3.0) * (*control - *to);
                flatten_cubic(&mut segments, current, c1, c2, *to, 0);
                current = *to;
            }
            PathCommand::Close => {
                if current != subpath_start {
                    segments.push(Segment { from: current, to: subpath_start });
                }
                current = subpath_start;
            }
        }
    }

    segments
}

/// Subdivide a cubic bezier into line segments.
/// Uses de Casteljau subdivision until segments are flat enough.
fn flatten_cubic(segments: &mut Vec<Segment>, p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, depth: u8) {
    const MAX_DEPTH: u8 = 8;
    const TOLERANCE: f32 = 0.25; // pixels

    if depth >= MAX_DEPTH {
        segments.push(Segment { from: p0, to: p3 });
        return;
    }

    // Flatness test: are control points close enough to the line p0→p3?
    let d1 = point_to_line_distance(p1, p0, p3);
    let d2 = point_to_line_distance(p2, p0, p3);

    if d1 + d2 <= TOLERANCE {
        segments.push(Segment { from: p0, to: p3 });
        return;
    }

    // de Casteljau subdivision at t=0.5
    let m01 = (p0 + p1) * 0.5;
    let m12 = (p1 + p2) * 0.5;
    let m23 = (p2 + p3) * 0.5;
    let m012 = (m01 + m12) * 0.5;
    let m123 = (m12 + m23) * 0.5;
    let mid = (m012 + m123) * 0.5;

    flatten_cubic(segments, p0, m01, m012, mid, depth + 1);
    flatten_cubic(segments, mid, m123, m23, p3, depth + 1);
}

/// Distance from point to line segment.
fn point_to_line_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-10 {
        return (p - a).length();
    }
    let cross = (p.x - a.x) * ab.y - (p.y - a.y) * ab.x;
    cross.abs() / len_sq.sqrt()
}

/// Compute winding number at a point using ray casting (horizontal ray to +x).
fn compute_winding(segments: &[Segment], point: Vec2) -> i32 {
    let mut winding = 0i32;

    for seg in segments {
        let y0 = seg.from.y;
        let y1 = seg.to.y;

        // Skip horizontal segments
        if (y1 - y0).abs() < 1e-10 {
            continue;
        }

        // Does this segment cross the horizontal ray from point to +inf?
        if (y0 <= point.y && y1 > point.y) || (y1 <= point.y && y0 > point.y) {
            // Compute x intersection
            let t = (point.y - y0) / (y1 - y0);
            let x_intersect = seg.from.x + t * (seg.to.x - seg.from.x);

            if point.x < x_intersect {
                // Ray crosses: determine direction for winding
                if y1 > y0 {
                    winding += 1; // Upward crossing
                } else {
                    winding -= 1; // Downward crossing
                }
            }
        }
    }

    winding
}

/// Minimum distance from a point to the path (for anti-aliasing).
fn signed_distance_to_path(segments: &[Segment], point: Vec2) -> f32 {
    let mut min_dist = f32::INFINITY;

    for seg in segments {
        let ab = seg.to - seg.from;
        let ap = point - seg.from;
        let len_sq = ab.length_squared();

        let t = if len_sq < 1e-10 {
            0.0
        } else {
            (ap.dot(ab) / len_sq).clamp(0.0, 1.0)
        };

        let closest = seg.from + ab * t;
        let dist = (point - closest).length();
        min_dist = min_dist.min(dist);
    }

    min_dist
}

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline]
fn color_to_u8(c: &PremultColor) -> (u8, u8, u8, u8) {
    (
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
        (c.a * 255.0) as u8,
    )
}
