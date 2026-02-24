//! SVG export — converts a document tree to an SVG string.

use rendero_core::id::NodeId;
use rendero_core::node::NodeKind;
use rendero_core::properties::{Color, Paint, Transform};
use rendero_core::tree::DocumentTree;

use crate::scene::AABB;

/// Export a document tree to an SVG string.
pub fn export_svg(tree: &DocumentTree, root: &NodeId, viewport: AABB) -> String {
    let w = viewport.width();
    let h = viewport.height();

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">"#,
        w, h, viewport.min.x, viewport.min.y, w, h
    );
    svg.push('\n');

    // Traverse children of root
    if let Some(children) = tree.children_of(root) {
        for child_id in children.iter() {
            export_node(&mut svg, tree, child_id, &Transform::translate(0.0, 0.0));
        }
    }

    svg.push_str("</svg>");
    svg
}

fn export_node(svg: &mut String, tree: &DocumentTree, node_id: &NodeId, parent_transform: &Transform) {
    let Some(node) = tree.get(node_id) else { return };
    if !node.visible { return; }

    let world = parent_transform.then(&node.transform);
    let fill = first_solid_fill(&node.style.fills);

    match &node.kind {
        NodeKind::Rectangle { corner_radii } => {
            let fill_str = color_to_svg(&fill);
            let opacity = if node.style.opacity < 1.0 {
                format!(r#" opacity="{}""#, node.style.opacity)
            } else {
                String::new()
            };
            svg.push_str(&format!(
                r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="{}"{}/>"#,
                world.tx, world.ty, node.width, node.height, fill_str, opacity
            ));
            svg.push('\n');
        }

        NodeKind::Ellipse { .. } => {
            let cx = world.tx + node.width / 2.0;
            let cy = world.ty + node.height / 2.0;
            let fill_str = color_to_svg(&fill);
            let opacity = if node.style.opacity < 1.0 {
                format!(r#" opacity="{}""#, node.style.opacity)
            } else {
                String::new()
            };

            if (node.width - node.height).abs() < 0.01 {
                // Circle
                svg.push_str(&format!(
                    r#"  <circle cx="{}" cy="{}" r="{}" fill="{}"{}/>"#,
                    cx, cy, node.width / 2.0, fill_str, opacity
                ));
            } else {
                svg.push_str(&format!(
                    r#"  <ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}"{}/>"#,
                    cx, cy, node.width / 2.0, node.height / 2.0, fill_str, opacity
                ));
            }
            svg.push('\n');
        }

        NodeKind::Text { runs, .. } => {
            let fill_str = if let Some(run) = runs.first() {
                color_to_svg(&run.color)
            } else {
                color_to_svg(&fill)
            };
            let content: String = runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join("");
            let font_size = runs.first().map(|r| r.font_size).unwrap_or(16.0);
            svg.push_str(&format!(
                r#"  <text x="{}" y="{}" font-size="{}" fill="{}">{}</text>"#,
                world.tx, world.ty + font_size, font_size, fill_str,
                escape_xml(&content)
            ));
            svg.push('\n');
        }

        NodeKind::Frame { .. } => {
            // Frame = group, recurse into children
            let fill_str = color_to_svg(&fill);
            if fill.a() > 0.0 {
                svg.push_str(&format!(
                    r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                    world.tx, world.ty, node.width, node.height, fill_str
                ));
                svg.push('\n');
            }
            if let Some(children) = tree.children_of(node_id) {
                for child_id in children.iter() {
                    export_node(svg, tree, child_id, &world);
                }
            }
        }

        NodeKind::Vector { paths } => {
            for path in paths {
                let d = path_commands_to_d(&path.commands);
                let fill_str = color_to_svg(&fill);
                svg.push_str(&format!(
                    r#"  <path d="{}" fill="{}"/>"#,
                    d, fill_str
                ));
                svg.push('\n');
            }
        }

        _ => {} // Skip unsupported node types
    }
}

fn first_solid_fill(fills: &[Paint]) -> Color {
    for fill in fills {
        if let Paint::Solid(c) = fill {
            return *c;
        }
    }
    Color::new(0.0, 0.0, 0.0, 0.0)
}

fn color_to_svg(c: &Color) -> String {
    let r = (c.r() * 255.0).round() as u8;
    let g = (c.g() * 255.0).round() as u8;
    let b = (c.b() * 255.0).round() as u8;
    if c.a() < 1.0 {
        format!("rgba({},{},{},{})", r, g, b, c.a())
    } else {
        format!("rgb({},{},{})", r, g, b)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn path_commands_to_d(commands: &[rendero_core::node::PathCommand]) -> String {
    use rendero_core::node::PathCommand;
    let mut d = String::new();
    for cmd in commands {
        match cmd {
            PathCommand::MoveTo(p) => d.push_str(&format!("M{} {} ", p.x, p.y)),
            PathCommand::LineTo(p) => d.push_str(&format!("L{} {} ", p.x, p.y)),
            PathCommand::CubicTo { control1, control2, to } => {
                d.push_str(&format!("C{} {} {} {} {} {} ", control1.x, control1.y, control2.x, control2.y, to.x, to.y));
            }
            PathCommand::QuadTo { control, to } => {
                d.push_str(&format!("Q{} {} {} {} ", control.x, control.y, to.x, to.y));
            }
            PathCommand::Close => d.push_str("Z "),
        }
    }
    d.trim().to_string()
}
