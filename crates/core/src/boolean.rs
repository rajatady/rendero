//! Boolean path operations: Union, Subtract, Intersect, Exclude.
//!
//! Takes child shapes and combines them into a single result path.
//! Strategy:
//! - Union: merge paths with consistent winding, NonZero fill
//! - Subtract: reverse secondary paths' winding, NonZero fill
//! - Intersect: polygon clipping (Sutherland-Hodgman)
//! - Exclude: merge paths, EvenOdd fill
//!
//! All operations first flatten curves to polylines, operate on polygons,
//! then emit the result as PathCommand sequences.

use glam::Vec2;

use crate::id::NodeId;
use crate::node::{BooleanOperation, Node, NodeKind, PathCommand, VectorPath};
use crate::properties::{FillRule, Transform};
use crate::tree::DocumentTree;

/// Result of a boolean operation.
pub struct BooleanResult {
    pub commands: Vec<PathCommand>,
    pub fill_rule: FillRule,
}

/// Compute the boolean result for a BooleanOp node.
/// Returns None if the node isn't a BooleanOp or has no children.
pub fn compute_boolean(tree: &DocumentTree, node_id: &NodeId) -> Option<BooleanResult> {
    let node = tree.get(node_id)?;
    let operation = match &node.kind {
        NodeKind::BooleanOp { operation } => *operation,
        _ => return None,
    };

    let children = tree.children_of(node_id)?;
    if children.is_empty() {
        return None;
    }

    // Convert each child to a polygon (flattened path commands in local space)
    let mut child_polys: Vec<Vec<Vec2>> = Vec::new();
    for child_id in children.iter() {
        if let Some(child) = tree.get(child_id) {
            let cmds = node_to_path_commands(child);
            let poly = flatten_to_polygon(&cmds, &child.transform);
            if poly.len() >= 3 {
                child_polys.push(poly);
            }
        }
    }

    if child_polys.is_empty() {
        return None;
    }

    match operation {
        BooleanOperation::Union => {
            let commands = polys_to_commands(&child_polys);
            Some(BooleanResult {
                commands,
                fill_rule: FillRule::NonZero,
            })
        }
        BooleanOperation::Subtract => {
            if child_polys.len() < 2 {
                let commands = polys_to_commands(&child_polys);
                return Some(BooleanResult {
                    commands,
                    fill_rule: FillRule::NonZero,
                });
            }
            // Keep first polygon, reverse all others
            let mut commands = poly_to_commands(&child_polys[0]);
            for poly in &child_polys[1..] {
                let reversed: Vec<Vec2> = poly.iter().copied().rev().collect();
                commands.extend(poly_to_commands(&reversed));
            }
            Some(BooleanResult {
                commands,
                fill_rule: FillRule::NonZero,
            })
        }
        BooleanOperation::Intersect => {
            // Sutherland-Hodgman: clip first polygon by each subsequent polygon
            let mut result = child_polys[0].clone();
            for clip_poly in &child_polys[1..] {
                result = sutherland_hodgman_clip(&result, clip_poly);
                if result.len() < 3 {
                    return Some(BooleanResult {
                        commands: Vec::new(),
                        fill_rule: FillRule::NonZero,
                    });
                }
            }
            Some(BooleanResult {
                commands: poly_to_commands(&result),
                fill_rule: FillRule::NonZero,
            })
        }
        BooleanOperation::Exclude => {
            let commands = polys_to_commands(&child_polys);
            Some(BooleanResult {
                commands,
                fill_rule: FillRule::EvenOdd,
            })
        }
    }
}

/// Convert a node's shape to path commands in local (node) space.
pub fn node_to_path_commands(node: &Node) -> Vec<PathCommand> {
    match &node.kind {
        NodeKind::Rectangle { .. } | NodeKind::Frame { .. } | NodeKind::Component | NodeKind::Instance { .. } => {
            let w = node.width;
            let h = node.height;
            vec![
                PathCommand::MoveTo(Vec2::new(0.0, 0.0)),
                PathCommand::LineTo(Vec2::new(w, 0.0)),
                PathCommand::LineTo(Vec2::new(w, h)),
                PathCommand::LineTo(Vec2::new(0.0, h)),
                PathCommand::Close,
            ]
        }
        NodeKind::Ellipse { .. } => {
            let rx = node.width / 2.0;
            let ry = node.height / 2.0;
            let cx = rx;
            let cy = ry;
            let k = 0.5522847498_f32;
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
        NodeKind::Line => {
            vec![
                PathCommand::MoveTo(Vec2::ZERO),
                PathCommand::LineTo(Vec2::new(node.width, 0.0)),
            ]
        }
        NodeKind::Vector { paths } => {
            paths.iter().flat_map(|p| p.commands.iter().cloned()).collect()
        }
        NodeKind::Polygon { point_count, inner_radius_ratio } => {
            polygon_path(*point_count, *inner_radius_ratio, node.width, node.height)
        }
        NodeKind::Text { .. } => Vec::new(),
        NodeKind::BooleanOp { .. } => Vec::new(), // Nested booleans resolved separately
        NodeKind::Image { .. } => Vec::new(), // Raster — no path representation
    }
}

/// Generate path commands for a polygon/star shape.
fn polygon_path(point_count: u32, inner_ratio: f32, width: f32, height: f32) -> Vec<PathCommand> {
    let n = point_count.max(3) as usize;
    let cx = width / 2.0;
    let cy = height / 2.0;
    let rx = width / 2.0;
    let ry = height / 2.0;

    let mut cmds = Vec::new();
    let is_star = inner_ratio > 0.0;
    let total_points = if is_star { n * 2 } else { n };

    for i in 0..total_points {
        let angle = std::f32::consts::TAU * (i as f32 / total_points as f32)
            - std::f32::consts::FRAC_PI_2;
        let (r_x, r_y) = if is_star && i % 2 == 1 {
            (rx * inner_ratio, ry * inner_ratio)
        } else {
            (rx, ry)
        };
        let pt = Vec2::new(cx + r_x * angle.cos(), cy + r_y * angle.sin());
        if i == 0 {
            cmds.push(PathCommand::MoveTo(pt));
        } else {
            cmds.push(PathCommand::LineTo(pt));
        }
    }
    cmds.push(PathCommand::Close);
    cmds
}

/// Flatten path commands to a polygon (list of points), applying a transform.
/// Curves are subdivided into line segments.
fn flatten_to_polygon(commands: &[PathCommand], transform: &Transform) -> Vec<Vec2> {
    let mut points = Vec::new();
    let mut current = Vec2::ZERO;

    for cmd in commands {
        match cmd {
            PathCommand::MoveTo(to) => {
                current = *to;
                points.push(transform.apply(current));
            }
            PathCommand::LineTo(to) => {
                current = *to;
                points.push(transform.apply(current));
            }
            PathCommand::CubicTo { control1, control2, to } => {
                flatten_cubic(current, *control1, *control2, *to, transform, &mut points);
                current = *to;
            }
            PathCommand::QuadTo { control, to } => {
                flatten_quad(current, *control, *to, transform, &mut points);
                current = *to;
            }
            PathCommand::Close => {
                // Close connects back to start — polygon is implicitly closed
            }
        }
    }

    points
}

/// Flatten a cubic bezier to line segments using de Casteljau subdivision.
fn flatten_cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, transform: &Transform, out: &mut Vec<Vec2>) {
    const TOLERANCE_SQ: f32 = 0.5;
    const MAX_DEPTH: u32 = 8;

    fn subdivide(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, transform: &Transform, out: &mut Vec<Vec2>, depth: u32) {
        if depth >= MAX_DEPTH {
            out.push(transform.apply(p3));
            return;
        }

        // Flatness test: distance of control points from the line p0→p3
        let d = p3 - p0;
        let len_sq = d.dot(d);
        if len_sq < 1e-10 {
            out.push(transform.apply(p3));
            return;
        }

        let d1 = ((p1 - p0).dot(d.perp())).abs();
        let d2 = ((p2 - p0).dot(d.perp())).abs();
        let flatness = (d1 + d2) * (d1 + d2) / len_sq;

        if flatness <= TOLERANCE_SQ {
            out.push(transform.apply(p3));
        } else {
            let m01 = (p0 + p1) * 0.5;
            let m12 = (p1 + p2) * 0.5;
            let m23 = (p2 + p3) * 0.5;
            let m012 = (m01 + m12) * 0.5;
            let m123 = (m12 + m23) * 0.5;
            let mid = (m012 + m123) * 0.5;

            subdivide(p0, m01, m012, mid, transform, out, depth + 1);
            subdivide(mid, m123, m23, p3, transform, out, depth + 1);
        }
    }

    subdivide(p0, p1, p2, p3, transform, out, 0);
}

/// Flatten a quadratic bezier to line segments.
fn flatten_quad(p0: Vec2, p1: Vec2, p2: Vec2, transform: &Transform, out: &mut Vec<Vec2>) {
    // Convert quad to cubic: C1 = P0 + 2/3*(P1-P0), C2 = P2 + 2/3*(P1-P2)
    let c1 = p0 + (p1 - p0) * (2.0 / 3.0);
    let c2 = p2 + (p1 - p2) * (2.0 / 3.0);
    flatten_cubic(p0, c1, c2, p2, transform, out);
}

/// Convert a polygon (Vec<Vec2>) to path commands.
fn poly_to_commands(poly: &[Vec2]) -> Vec<PathCommand> {
    if poly.is_empty() {
        return Vec::new();
    }
    let mut cmds = Vec::with_capacity(poly.len() + 2);
    cmds.push(PathCommand::MoveTo(poly[0]));
    for pt in &poly[1..] {
        cmds.push(PathCommand::LineTo(*pt));
    }
    cmds.push(PathCommand::Close);
    cmds
}

/// Convert multiple polygons to path commands (multi-contour path).
fn polys_to_commands(polys: &[Vec<Vec2>]) -> Vec<PathCommand> {
    let mut cmds = Vec::new();
    for poly in polys {
        cmds.extend(poly_to_commands(poly));
    }
    cmds
}

/// Sutherland-Hodgman polygon clipping algorithm.
/// Clips `subject` polygon against `clip` polygon.
/// Both polygons are assumed to be closed (implicitly).
fn sutherland_hodgman_clip(subject: &[Vec2], clip: &[Vec2]) -> Vec<Vec2> {
    if subject.is_empty() || clip.is_empty() {
        return Vec::new();
    }

    let mut output = subject.to_vec();

    let clip_len = clip.len();
    for i in 0..clip_len {
        if output.is_empty() {
            return Vec::new();
        }

        let edge_start = clip[i];
        let edge_end = clip[(i + 1) % clip_len];

        let mut input = output;
        output = Vec::new();

        let input_len = input.len();
        for j in 0..input_len {
            let current = input[j];
            let previous = input[(j + input_len - 1) % input_len];

            let curr_inside = is_inside(current, edge_start, edge_end);
            let prev_inside = is_inside(previous, edge_start, edge_end);

            if curr_inside {
                if !prev_inside {
                    // Entering: add intersection
                    if let Some(inter) = line_intersection(previous, current, edge_start, edge_end) {
                        output.push(inter);
                    }
                }
                output.push(current);
            } else if prev_inside {
                // Leaving: add intersection
                if let Some(inter) = line_intersection(previous, current, edge_start, edge_end) {
                    output.push(inter);
                }
            }
        }
    }

    output
}

/// Test if point is on the inside (left) of the directed edge from a to b.
#[inline]
fn is_inside(point: Vec2, edge_a: Vec2, edge_b: Vec2) -> bool {
    (edge_b.x - edge_a.x) * (point.y - edge_a.y)
        - (edge_b.y - edge_a.y) * (point.x - edge_a.x)
        >= 0.0
}

/// Compute intersection of line segments (p1→p2) and (p3→p4).
fn line_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let d1 = p2 - p1;
    let d2 = p4 - p3;
    let cross = d1.x * d2.y - d1.y * d2.x;

    if cross.abs() < 1e-10 {
        return None; // Parallel
    }

    let d3 = p3 - p1;
    let t = (d3.x * d2.y - d3.y * d2.x) / cross;

    Some(p1 + d1 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_path_triangle() {
        let cmds = polygon_path(3, 0.0, 100.0, 100.0);
        // Triangle: MoveTo + 2 LineTo + Close = 4 commands
        assert_eq!(cmds.len(), 4);
    }

    #[test]
    fn test_polygon_path_star() {
        let cmds = polygon_path(5, 0.5, 100.0, 100.0);
        // 5-point star: MoveTo + 9 LineTo + Close = 11
        assert_eq!(cmds.len(), 11);
    }

    #[test]
    fn test_sutherland_hodgman_overlap() {
        // Two overlapping squares
        let a = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let b = vec![
            Vec2::new(5.0, 5.0),
            Vec2::new(15.0, 5.0),
            Vec2::new(15.0, 15.0),
            Vec2::new(5.0, 15.0),
        ];
        let result = sutherland_hodgman_clip(&a, &b);
        // Result should be the intersection square: (5,5)-(10,10)
        assert!(result.len() >= 3);
        for pt in &result {
            assert!(pt.x >= 4.99 && pt.x <= 10.01);
            assert!(pt.y >= 4.99 && pt.y <= 10.01);
        }
    }

    #[test]
    fn test_sutherland_hodgman_no_overlap() {
        let a = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 0.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(0.0, 5.0),
        ];
        let b = vec![
            Vec2::new(10.0, 10.0),
            Vec2::new(15.0, 10.0),
            Vec2::new(15.0, 15.0),
            Vec2::new(10.0, 15.0),
        ];
        let result = sutherland_hodgman_clip(&a, &b);
        assert!(result.len() < 3);
    }

    #[test]
    fn test_flatten_rect_identity() {
        let cmds = vec![
            PathCommand::MoveTo(Vec2::new(0.0, 0.0)),
            PathCommand::LineTo(Vec2::new(10.0, 0.0)),
            PathCommand::LineTo(Vec2::new(10.0, 10.0)),
            PathCommand::LineTo(Vec2::new(0.0, 10.0)),
            PathCommand::Close,
        ];
        let poly = flatten_to_polygon(&cmds, &Transform::IDENTITY);
        assert_eq!(poly.len(), 4);
    }
}
