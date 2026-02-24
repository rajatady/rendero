//! Stroke rendering — converts a path + stroke properties into a filled outline.
//!
//! Approach: expand the path into a stroke outline (a closed polygon),
//! then rasterize it as a filled path via the normal path rasterizer.

use rendero_core::node::PathCommand;
use rendero_core::properties::{StrokeAlign, StrokeCap, StrokeJoin};
use glam::Vec2;

/// Expand a path into a stroke outline.
/// Returns path commands for a closed filled shape representing the stroke.
pub fn expand_stroke(
    commands: &[PathCommand],
    weight: f32,
    align: StrokeAlign,
    cap: StrokeCap,
    _join: StrokeJoin,
) -> Vec<PathCommand> {
    let points = flatten_to_points(commands);
    if points.len() < 2 {
        return Vec::new();
    }

    let (outer_w, inner_w) = match align {
        StrokeAlign::Center => (weight / 2.0, weight / 2.0),
        StrokeAlign::Inside => (0.0, weight),
        StrokeAlign::Outside => (weight, 0.0),
    };

    let is_closed = points.len() > 2 && (points.first().unwrap() - points.last().unwrap()).length() < 0.01;
    let normals = compute_normals(&points);

    // Build outer and inner offset curves
    let outer: Vec<Vec2> = points.iter().zip(normals.iter())
        .map(|(p, n)| *p + *n * outer_w)
        .collect();
    let inner: Vec<Vec2> = points.iter().zip(normals.iter())
        .map(|(p, n)| *p - *n * inner_w)
        .collect();

    let mut result = Vec::new();

    // Outer edge forward
    if let Some(&first) = outer.first() {
        result.push(PathCommand::MoveTo(first));
        for &p in outer.iter().skip(1) {
            result.push(PathCommand::LineTo(p));
        }
    }

    // End cap
    if !is_closed {
        let last_idx = points.len() - 1;
        match cap {
            StrokeCap::Round => {
                let center = points[last_idx];
                let n = normals[last_idx];
                let steps = 8;
                for s in 1..=steps {
                    let angle = std::f32::consts::PI * s as f32 / steps as f32;
                    let (sin, cos) = angle.sin_cos();
                    let p = center + Vec2::new(
                        n.x * cos - n.y * sin,
                        n.x * sin + n.y * cos,
                    ) * outer_w;
                    result.push(PathCommand::LineTo(p));
                }
            }
            StrokeCap::Square => {
                let dir = if last_idx > 0 {
                    (points[last_idx] - points[last_idx - 1]).normalize_or_zero()
                } else { Vec2::X };
                let ext = dir * outer_w;
                result.push(PathCommand::LineTo(outer[last_idx] + ext));
                result.push(PathCommand::LineTo(inner[last_idx] + ext));
            }
            StrokeCap::None => {
                result.push(PathCommand::LineTo(inner[last_idx]));
            }
        }
    }

    // Inner edge backward
    for &p in inner.iter().rev() {
        result.push(PathCommand::LineTo(p));
    }

    // Start cap
    if !is_closed {
        match cap {
            StrokeCap::Round => {
                let center = points[0];
                let n = normals[0];
                let steps = 8;
                for s in 1..steps {
                    let angle = std::f32::consts::PI + std::f32::consts::PI * s as f32 / steps as f32;
                    let (sin, cos) = angle.sin_cos();
                    let p = center + Vec2::new(
                        n.x * cos - n.y * sin,
                        n.x * sin + n.y * cos,
                    ) * outer_w;
                    result.push(PathCommand::LineTo(p));
                }
            }
            StrokeCap::Square => {
                let dir = if points.len() > 1 {
                    (points[1] - points[0]).normalize_or_zero()
                } else { Vec2::X };
                let ext = dir * outer_w;
                result.push(PathCommand::LineTo(inner[0] - ext));
                result.push(PathCommand::LineTo(outer[0] - ext));
            }
            StrokeCap::None => {}
        }
    }

    result.push(PathCommand::Close);
    result
}

fn flatten_to_points(commands: &[PathCommand]) -> Vec<Vec2> {
    let mut points = Vec::new();
    let mut current = Vec2::ZERO;

    for cmd in commands {
        match cmd {
            PathCommand::MoveTo(p) => {
                if !points.is_empty() { break; }
                current = *p;
                points.push(current);
            }
            PathCommand::LineTo(p) => {
                current = *p;
                points.push(current);
            }
            PathCommand::CubicTo { control1, control2, to } => {
                flatten_cubic(&mut points, current, *control1, *control2, *to, 0);
                current = *to;
            }
            PathCommand::QuadTo { control, to } => {
                let c1 = current + (2.0 / 3.0) * (*control - current);
                let c2 = *to + (2.0 / 3.0) * (*control - *to);
                flatten_cubic(&mut points, current, c1, c2, *to, 0);
                current = *to;
            }
            PathCommand::Close => {
                if let Some(&first) = points.first() {
                    if (current - first).length() > 0.01 {
                        points.push(first);
                    }
                }
                break;
            }
        }
    }
    points
}

fn flatten_cubic(pts: &mut Vec<Vec2>, p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, depth: u8) {
    if depth >= 6 { pts.push(p3); return; }
    let d1 = pt_line_dist(p1, p0, p3);
    let d2 = pt_line_dist(p2, p0, p3);
    if d1 + d2 <= 0.5 { pts.push(p3); return; }
    let m01 = (p0 + p1) * 0.5;
    let m12 = (p1 + p2) * 0.5;
    let m23 = (p2 + p3) * 0.5;
    let m012 = (m01 + m12) * 0.5;
    let m123 = (m12 + m23) * 0.5;
    let mid = (m012 + m123) * 0.5;
    flatten_cubic(pts, p0, m01, m012, mid, depth + 1);
    flatten_cubic(pts, mid, m123, m23, p3, depth + 1);
}

fn pt_line_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-10 { return (p - a).length(); }
    ((p.x - a.x) * ab.y - (p.y - a.y) * ab.x).abs() / len_sq.sqrt()
}

fn compute_normals(points: &[Vec2]) -> Vec<Vec2> {
    let n = points.len();
    let mut normals = vec![Vec2::ZERO; n];
    for i in 0..n {
        let prev = if i > 0 { points[i - 1] } else { points[i] };
        let next = if i < n - 1 { points[i + 1] } else { points[i] };
        let dir = (next - prev).normalize_or_zero();
        normals[i] = Vec2::new(-dir.y, dir.x);
    }
    normals
}
