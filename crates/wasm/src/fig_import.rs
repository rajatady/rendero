//! .fig JSON importer — converts fig2json output into engine nodes.
//!
//! Reads the JSON produced by fig2json (github.com/kreako/fig2json) and creates
//! engine nodes in our document tree. Handles: frames, rectangles, ellipses,
//! text, vectors, fills, strokes, effects, corner radii, opacity, visibility.

use rendero_core::node::*;
use rendero_core::properties::*;
use rendero_core::id::NodeId;
use rendero_core::document::Document;
use rendero_core::layout::compute_layout;
use glam::Vec2;
use serde_json::Value;

/// Result of an import operation.
pub struct ImportResult {
    pub pages_imported: usize,
    pub nodes_imported: usize,
    pub errors: Vec<String>,
    pub has_image_fills: bool,
}

/// Import a fig2json JSON string into the document.
/// Creates one page per top-level child in document.children[].
/// Returns import statistics.
pub fn import_fig_json(doc: &mut Document, json_str: &str, image_base: &str) -> ImportResult {
    let mut result = ImportResult {
        pages_imported: 0,
        nodes_imported: 0,
        errors: Vec::new(),
        has_image_fills: false,
    };

    let root: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            result.errors.push(format!("JSON parse error: {}", e));
            return result;
        }
    };

    let pages = match root.get("document").and_then(|d| d.get("children")).and_then(|c| c.as_array()) {
        Some(p) => p,
        None => {
            result.errors.push("No document.children[] found".into());
            return result;
        }
    };

    for page_val in pages {
        let page_name = page_val.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Imported Page");

        let _page_id = doc.add_page(page_name);
        let page_idx = doc.pages.len() - 1;
        // The tree root is NodeId::ROOT, not page_id
        let tree_root = doc.page(page_idx).unwrap().tree.root_id();
        result.pages_imported += 1;

        // Import children into this page under the tree root
        if let Some(children) = page_val.get("children").and_then(|c| c.as_array()) {
            for child_val in children {
                let count = import_node(doc, page_idx, tree_root, child_val, image_base, &mut result);
                result.nodes_imported += count;
            }
        }

        // Apply auto-layout computation to position children correctly
        let page = doc.page_mut(page_idx).unwrap();
        let root = page.tree.root_id();
        compute_layout(&mut page.tree, &root);
    }

    result
}

/// Import from a pre-parsed serde_json::Value tree directly (no string round-trip).
/// The `document` Value should have the same structure as fig2json output:
/// {"children": [page1, page2, ...]} where each page has {"name":"...", "children":[...]}.
pub fn import_fig_value(doc: &mut Document, document: &Value, image_base: &str) -> ImportResult {
    let mut result = ImportResult {
        pages_imported: 0,
        nodes_imported: 0,
        errors: Vec::new(),
        has_image_fills: false,
    };

    let pages = match document.get("children").and_then(|c| c.as_array()) {
        Some(p) => p,
        None => {
            result.errors.push("No children[] found in document value".into());
            return result;
        }
    };

    for page_val in pages {
        let page_name = page_val.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Imported Page");

        let _page_id = doc.add_page(page_name);
        let page_idx = doc.pages.len() - 1;
        let tree_root = doc.page(page_idx).unwrap().tree.root_id();
        result.pages_imported += 1;

        if let Some(children) = page_val.get("children").and_then(|c| c.as_array()) {
            for child_val in children {
                let count = import_node(doc, page_idx, tree_root, child_val, image_base, &mut result);
                result.nodes_imported += count;
            }
        }

        // Apply auto-layout computation to position children correctly
        let page = doc.page_mut(page_idx).unwrap();
        let root = page.tree.root_id();
        compute_layout(&mut page.tree, &root);
    }

    result
}

/// Import a single page from JSON string. Used for large files where the full
/// document JSON is too big to pass to WASM at once. JS splits by page and
/// calls this per-page.
pub fn import_fig_page_json(doc: &mut Document, page_json: &str, image_base: &str) -> ImportResult {
    let mut result = ImportResult {
        pages_imported: 0,
        nodes_imported: 0,
        errors: Vec::new(),
        has_image_fills: false,
    };

    let page_val: Value = match serde_json::from_str(page_json) {
        Ok(v) => v,
        Err(e) => {
            result.errors.push(format!("JSON parse error: {}", e));
            return result;
        }
    };

    let page_name = page_val.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Imported Page");

    let _page_id = doc.add_page(page_name);
    let page_idx = doc.pages.len() - 1;
    let tree_root = doc.page(page_idx).unwrap().tree.root_id();
    result.pages_imported = 1;

    if let Some(children) = page_val.get("children").and_then(|c| c.as_array()) {
        for child_val in children {
            let count = import_node(doc, page_idx, tree_root, child_val, image_base, &mut result);
            result.nodes_imported += count;
        }
    }

    // Apply auto-layout computation to position children correctly
    let page = doc.page_mut(page_idx).unwrap();
    let root = page.tree.root_id();
    compute_layout(&mut page.tree, &root);

    result
}

/// Recursively import a node and its children. Returns number of nodes created.
fn import_node(
    doc: &mut Document,
    page_idx: usize,
    parent_id: NodeId,
    val: &Value,
    image_base: &str,
    result: &mut ImportResult,
) -> usize {
    let errors = &mut result.errors;
    let name = val.get("name").and_then(|n| n.as_str()).unwrap_or("unnamed");

    // Mask nodes define clipping paths for subsequent siblings.
    // The renderer handles is_mask by using the shape as a clip path (canvas2d.rs).
    let is_mask = val.get("mask").and_then(|v| v.as_bool()).unwrap_or(false);

    // Skip invisible nodes early (but still count them)
    let visible = val.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);

    // Determine node type from properties
    let has_children = val.get("children").and_then(|c| c.as_array()).map_or(false, |a| !a.is_empty());
    let has_vector = val.get("vectorData").is_some();
    let has_text = val.get("textData").is_some();

    let (w, h) = get_size(val);
    let node_transform = get_node_transform(val);

    let id = doc.next_id();
    let mut node = if has_children {
        // Frame/group — clip based on Figma defaults
        let corner_radii = get_corner_radii(val);
        let auto_layout = get_auto_layout(val);
        // In Figma, frames clip by default, groups don't.
        // Heuristic: if node has visual properties (fill/stroke/corners/auto-layout),
        // it's a frame → default clip=true. Plain containers are groups → clip=false.
        // html.to.design exports have no `clipsContent` field (fig2json strips defaults).
        // In Figma, frames default to clip=true, groups to clip=false.
        // Without a reliable type field, we default ALL containers to clip=true.
        // This matches Figma behavior for frames (the majority case) and prevents
        // overflow artifacts from carousel arrows, navigation chevrons, etc.
        // Groups that shouldn't clip are rare and the visual impact is minimal
        // (their children are typically within bounds anyway).
        let default_clip = true;
        let explicit_clip = val.get("clipsContent").and_then(|v| v.as_bool());
        let clip_from_field = explicit_clip.unwrap_or(default_clip);
        // Also clip if any child is a mask — the mask defines a clipping region
        let has_mask_child = val.get("children").and_then(|c| c.as_array()).map_or(false, |children| {
            children.iter().any(|child| child.get("mask").and_then(|v| v.as_bool()).unwrap_or(false))
        });
        let clip_content = clip_from_field || has_mask_child;
        let mut n = Node::frame(id, name, w, h);
        n.kind = NodeKind::Frame {
            clip_content,
            auto_layout,
            corner_radii,
        };
        n
    } else if has_text {
        // Text node
        let (runs, align) = get_text_data(val);
        let mut n = Node::text(id, name, "", 16.0, Color::BLACK);
        n.kind = NodeKind::Text {
            runs,
            align,
            vertical_align: TextVerticalAlign::Top,
            resize: TextResize::Height,
        };
        n.width = w;
        n.height = h;
        n
    } else if has_vector {
        // Vector node
        let paths = get_vector_paths(val, w, h);
        let mut n = Node::frame(id, name, w, h); // start from frame, override kind
        n.kind = NodeKind::Vector { paths };
        n.width = w;
        n.height = h;
        n
    } else {
        // Check if it looks like an ellipse (has arcData) or rectangle
        if val.get("arcData").is_some() {
            Node::ellipse(id, name, w, h)
        } else {
            let mut n = Node::rectangle(id, name, w, h);
            let corner_radii = get_corner_radii(val);
            n.kind = NodeKind::Rectangle { corner_radii };
            n
        }
    };

    // Apply transform (translation + rotation + scale)
    node.transform = node_transform;

    // Apply visibility, locked, and mask flag
    node.visible = visible;
    node.locked = val.get("locked").and_then(|v| v.as_bool()).unwrap_or(false);
    node.is_mask = is_mask;

    // Apply style
    node.style.opacity = val.get("opacity").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

    // Boolean operation nodes (UNION, XOR, SUBTRACT, INTERSECT) define compound paths.
    // Their fill applies to the combined boolean shape, not as a rectangular background.
    // Since we can't compute boolean path ops yet, skip the parent fill and render children only.
    let is_boolean_op = val.get("booleanOperation").is_some();
    node.style.fills = if is_boolean_op {
        Vec::new()
    } else {
        get_fills(val, image_base)
    };

    // If no fills but has backgroundColor, use it (common for pages/frames)
    if node.style.fills.is_empty() {
        if let Some(bg) = val.get("backgroundColor").and_then(|v| v.as_str()) {
            let color = parse_hex_color(bg);
            if color.a() > 0.0 {
                node.style.fills.push(Paint::Solid(color));
            }
        }
    }

    node.style.strokes = get_strokes(val, image_base);
    node.style.stroke_weight = val.get("strokeWeight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    node.style.effects = get_effects(val);

    // Stroke cap
    if let Some(cap) = val.get("strokeCap").and_then(|v| v.get("value")).and_then(|v| v.as_str()) {
        node.style.stroke_cap = match cap {
            "ROUND" => StrokeCap::Round,
            "SQUARE" => StrokeCap::Square,
            _ => StrokeCap::None,
        };
    }

    // Dash pattern
    if let Some(dashes) = val.get("dashPattern").and_then(|v| v.as_array()) {
        let pattern: Vec<f32> = dashes.iter()
            .filter_map(|v| v.as_f64().map(|d| d as f32))
            .collect();
        if !pattern.is_empty() {
            node.style.dash_pattern = pattern;
        }
    }

    // Track if any image fills exist (for fast-path in rendering)
    if node.style.fills.iter().any(|f| matches!(f, Paint::Image { .. })) {
        result.has_image_fills = true;
    }

    // Apply blend mode
    if let Some(bm) = val.get("blendMode").and_then(|v| v.get("value")).and_then(|v| v.as_str()) {
        node.style.blend_mode = parse_blend_mode(bm);
    }

    // Insert node into tree
    if doc.add_node(page_idx, node, parent_id, usize::MAX).is_err() {
        errors.push(format!("Failed to add node: {}", name));
        return 0;
    }

    let mut count = 1;

    // Recurse into children
    if let Some(children) = val.get("children").and_then(|c| c.as_array()) {
        for child_val in children {
            count += import_node(doc, page_idx, id, child_val, image_base, result);
        }
    }

    count
}

// --- Property extractors ---

fn get_size(val: &Value) -> (f32, f32) {
    let size = val.get("size");
    let w = size.and_then(|s| s.get("x")).and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
    let h = size.and_then(|s| s.get("y")).and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
    (w, h)
}

fn get_node_transform(val: &Value) -> Transform {
    let t = match val.get("transform") {
        Some(t) => t,
        None => return Transform::IDENTITY,
    };

    let x = t.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let y = t.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let rotation = t.get("rotation").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let scale_x = t.get("scaleX").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let scale_y = t.get("scaleY").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

    if rotation == 0.0 && scale_x == 1.0 && scale_y == 1.0 {
        return Transform::translate(x, y);
    }

    // Build affine: translate * rotate * scale
    let rad = rotation.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    Transform {
        a: cos * scale_x,
        b: sin * scale_x,
        c: -sin * scale_y,
        d: cos * scale_y,
        tx: x,
        ty: y,
    }
}

fn get_corner_radii(val: &Value) -> CornerRadii {
    // Check per-corner first
    let tl = val.get("rectangleTopLeftCornerRadius").and_then(|v| v.as_f64());
    let tr = val.get("rectangleTopRightCornerRadius").and_then(|v| v.as_f64());
    let bl = val.get("rectangleBottomLeftCornerRadius").and_then(|v| v.as_f64());
    let br = val.get("rectangleBottomRightCornerRadius").and_then(|v| v.as_f64());

    if tl.is_some() || tr.is_some() || bl.is_some() || br.is_some() {
        // At least one per-corner value exists
        let uniform = val.get("cornerRadius").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        CornerRadii::PerCorner {
            top_left: tl.unwrap_or(uniform as f64) as f32,
            top_right: tr.unwrap_or(uniform as f64) as f32,
            bottom_right: br.unwrap_or(uniform as f64) as f32,
            bottom_left: bl.unwrap_or(uniform as f64) as f32,
        }
    } else if let Some(r) = val.get("cornerRadius").and_then(|v| v.as_f64()) {
        CornerRadii::Uniform(r as f32)
    } else {
        CornerRadii::Uniform(0.0)
    }
}

fn get_auto_layout(val: &Value) -> Option<AutoLayout> {
    let stack_mode = val.get("stackMode").and_then(|v| v.get("value")).and_then(|v| v.as_str())?;
    let direction = match stack_mode {
        "HORIZONTAL" => LayoutDirection::Horizontal,
        "VERTICAL" => LayoutDirection::Vertical,
        _ => return None,
    };
    let spacing = val.get("stackSpacing").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let h_pad = val.get("stackHorizontalPadding").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let v_pad = val.get("stackVerticalPadding").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let pad_bottom = val.get("stackPaddingBottom").and_then(|v| v.as_f64()).unwrap_or(v_pad as f64) as f32;
    let pad_right = val.get("stackPaddingRight").and_then(|v| v.as_f64()).unwrap_or(h_pad as f64) as f32;

    Some(AutoLayout {
        direction,
        spacing,
        padding_top: v_pad,
        padding_right: pad_right,
        padding_bottom: pad_bottom,
        padding_left: h_pad,
        primary_sizing: SizingMode::Fixed,
        counter_sizing: SizingMode::Fixed,
        align: LayoutAlign::Start,
    })
}

/// Parse hex color string (#RRGGBB or #RRGGBBAA) to Color
fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let len = hex.len();
    if len < 6 {
        return Color::BLACK;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    let a = if len >= 8 {
        u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
    } else {
        1.0
    };
    Color::new(r, g, b, a)
}

fn get_fills(val: &Value, image_base: &str) -> Vec<Paint> {
    let mut fills = Vec::new();
    if let Some(paints) = val.get("fillPaints").and_then(|v| v.as_array()) {
        for paint in paints {
            if let Some(fill) = parse_paint(paint, image_base) {
                fills.push(fill);
            }
        }
    }
    fills
}

fn get_strokes(val: &Value, image_base: &str) -> Vec<Paint> {
    let mut strokes = Vec::new();
    if let Some(paints) = val.get("strokePaints").and_then(|v| v.as_array()) {
        for paint in paints {
            if let Some(stroke) = parse_paint(paint, image_base) {
                strokes.push(stroke);
            }
        }
    }
    strokes
}

fn parse_paint(paint: &Value, image_base: &str) -> Option<Paint> {
    // Solid color
    if let Some(color_hex) = paint.get("color").and_then(|v| v.as_str()) {
        let mut color = parse_hex_color(color_hex);
        // Apply paint-level opacity if present
        if let Some(opacity) = paint.get("opacity").and_then(|v| v.as_f64()) {
            color = Color::new(color.r(), color.g(), color.b(), color.a() * opacity as f32);
        }
        return Some(Paint::Solid(color));
    }

    // Gradient with stops
    if let Some(stops_arr) = paint.get("stops").and_then(|v| v.as_array()) {
        let mut stops: Vec<GradientStop> = stops_arr.iter().filter_map(|s| {
            let pos = s.get("position").and_then(|v| v.as_f64())? as f32;
            let color_hex = s.get("color").and_then(|v| v.as_str())?;
            let color = parse_hex_color(color_hex);
            Some(GradientStop::new(pos, color))
        }).collect();

        // Apply paint-level opacity to all stops
        if let Some(opacity) = paint.get("opacity").and_then(|v| v.as_f64()) {
            let op = opacity as f32;
            for stop in &mut stops {
                let c = stop.color;
                stop.color = Color::new(c.r(), c.g(), c.b(), c.a() * op);
            }
        }

        if !stops.is_empty() {
            // Compute start/end from gradient transform
            let (start, end) = parse_gradient_transform(paint);
            return Some(Paint::LinearGradient { stops, start, end });
        }
    }

    // Image fill — store full path for Canvas 2D renderer to load
    if let Some(image) = paint.get("image") {
        if let Some(raw_filename) = image.get("filename").and_then(|f| f.as_str()) {
            let opacity = paint.get("opacity").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
            // fig2json sometimes omits .png extension on image hashes
            let filename = if raw_filename.contains('.') {
                raw_filename.to_string()
            } else {
                format!("{}.png", raw_filename)
            };
            // Build full path: <image_base>/<filename> (e.g. "Coffee Shop-extracted/images/abc.png")
            let full_path = if image_base.is_empty() {
                filename
            } else {
                format!("{}/{}", image_base, filename)
            };
            return Some(Paint::Image {
                path: full_path,
                scale_mode: ImageScaleMode::Fill,
                opacity,
            });
        }
    }

    None
}

/// Parse gradient transform from fig2json format into start/end points.
/// Transform has: x, y (normalized origin), rotation (degrees), scaleX, scaleY.
/// The gradient line goes from the transform origin in the direction of rotation,
/// scaled by scaleX (length of gradient line in normalized [0,1] space).
fn parse_gradient_transform(paint: &Value) -> (Vec2, Vec2) {
    let t = match paint.get("transform") {
        Some(t) => t,
        None => return (Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)),
    };

    let x = t.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let y = t.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let rotation = t.get("rotation").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let scale_x = t.get("scaleX").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

    // Convert rotation degrees to radians, compute direction scaled by gradient length
    let rad = rotation.to_radians();
    let dx = rad.cos() * scale_x;
    let dy = rad.sin() * scale_x;

    // Start at (x, y), end at (x + dx, y + dy) in normalized [0,1] space
    let start = Vec2::new(x, y);
    let end = Vec2::new(x + dx, y + dy);

    (start, end)
}

fn get_effects(val: &Value) -> Vec<Effect> {
    let mut effects = Vec::new();
    if let Some(effs) = val.get("effects").and_then(|v| v.as_array()) {
        for eff in effs {
            let color = eff.get("color").and_then(|v| v.as_str())
                .map(parse_hex_color)
                .unwrap_or(Color::new(0.0, 0.0, 0.0, 0.25));
            let offset_x = eff.get("offset").and_then(|o| o.get("x")).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let offset_y = eff.get("offset").and_then(|o| o.get("y")).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let radius = eff.get("radius").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let spread = eff.get("spread").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

            effects.push(Effect::DropShadow {
                color,
                offset: Vec2::new(offset_x, offset_y),
                blur_radius: radius,
                spread,
            });
        }
    }
    effects
}

fn get_text_data(val: &Value) -> (Vec<TextRun>, TextAlign) {
    let characters = val.get("textData")
        .and_then(|td| td.get("characters"))
        .and_then(|c| c.as_str())
        .unwrap_or("");

    let font_size = val.get("fontSize").and_then(|v| v.as_f64()).unwrap_or(16.0) as f32;
    let font_family = val.get("fontName")
        .and_then(|fn_val| fn_val.get("family"))
        .and_then(|f| f.as_str())
        .unwrap_or("Inter")
        .to_string();
    let font_style = val.get("fontName")
        .and_then(|fn_val| fn_val.get("style"))
        .and_then(|s| s.as_str())
        .unwrap_or("Regular");
    let font_weight = if font_style.contains("Black") { 900 }
        else if font_style.contains("ExtraBold") || font_style.contains("UltraBold") { 800 }
        else if font_style.contains("Bold") { 700 }
        else if font_style.contains("SemiBold") || font_style.contains("Semibold") || font_style.contains("DemiBold") { 600 }
        else if font_style.contains("Medium") { 500 }
        else if font_style.contains("Light") && font_style.contains("Extra") { 200 }
        else if font_style.contains("Light") { 300 }
        else if font_style.contains("Thin") || font_style.contains("Hairline") { 100 }
        else { 400 };
    let italic = font_style.contains("Italic");

    let letter_spacing = val.get("letterSpacing").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let line_height = val.get("lineHeight").and_then(|v| v.as_str()).and_then(|s| {
        s.trim_end_matches("px").parse::<f32>().ok()
    });

    // Text color — from fills or default black
    let color = get_fills(val, "").first().and_then(|f| match f {
        Paint::Solid(c) => Some(*c),
        _ => None,
    }).unwrap_or(Color::BLACK);

    let align = match val.get("textAlignHorizontal").and_then(|v| v.get("value")).and_then(|v| v.as_str()) {
        Some("CENTER") => TextAlign::Center,
        Some("RIGHT") => TextAlign::Right,
        Some("JUSTIFIED") => TextAlign::Justified,
        _ => TextAlign::Left,
    };

    let decoration = match val.get("textDecoration").and_then(|v| v.as_str()) {
        Some("UNDERLINE") => TextDecoration::Underline,
        Some("STRIKETHROUGH") => TextDecoration::Strikethrough,
        _ => TextDecoration::None,
    };

    let runs = vec![TextRun {
        text: characters.to_string(),
        font_family,
        font_size,
        font_weight,
        italic,
        color,
        letter_spacing,
        line_height,
        decoration,
        fill_override: None,
    }];

    (runs, align)
}

/// Convert fig vectorData (vectorNetwork) into our VectorPath format.
/// vectorNetwork has: vertices[{x,y}], segments[{start:{vertex,dx,dy}, end:{vertex,dx,dy}}], regions[{loops:[{segments:[idx]}]}]
fn get_vector_paths(val: &Value, norm_w: f32, norm_h: f32) -> Vec<VectorPath> {
    let vd = match val.get("vectorData") {
        Some(v) => v,
        None => return Vec::new(),
    };

    let vn = match vd.get("vectorNetwork") {
        Some(v) => v,
        None => return Vec::new(),
    };

    let norm_size = vd.get("normalizedSize");
    let ns_x = norm_size.and_then(|s| s.get("x")).and_then(|v| v.as_f64()).unwrap_or(norm_w as f64) as f32;
    let ns_y = norm_size.and_then(|s| s.get("y")).and_then(|v| v.as_f64()).unwrap_or(norm_h as f64) as f32;

    // Scale factor: vectorNetwork coords are in normalizedSize space, we need node space
    let sx = if ns_x > 0.0 { norm_w / ns_x } else { 1.0 };
    let sy = if ns_y > 0.0 { norm_h / ns_y } else { 1.0 };

    let vertices: Vec<(f32, f32)> = vn.get("vertices").and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|v| {
            let x = v.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let y = v.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            (x * sx, y * sy)
        }).collect())
        .unwrap_or_default();

    // Segments: each has start{vertex, dx, dy} and end{vertex, dx, dy}
    // dx/dy are bezier control point offsets from the vertex
    struct Segment {
        start_vertex: usize,
        start_dx: f32,
        start_dy: f32,
        end_vertex: usize,
        end_dx: f32,
        end_dy: f32,
    }

    let segments: Vec<Segment> = vn.get("segments").and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|s| {
            let start = s.get("start").unwrap_or(s);
            let end = s.get("end").unwrap_or(s);
            Segment {
                start_vertex: start.get("vertex").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                start_dx: start.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 * sx,
                start_dy: start.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 * sy,
                end_vertex: end.get("vertex").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                end_dx: end.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 * sx,
                end_dy: end.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32 * sy,
            }
        }).collect())
        .unwrap_or_default();

    // Regions define closed paths via loops of segment indices
    let regions = vn.get("regions").and_then(|v| v.as_array());

    let mut paths = Vec::new();

    if let Some(regions) = regions {
        for region in regions {
            let loops = match region.get("loops").and_then(|v| v.as_array()) {
                Some(l) => l,
                None => continue,
            };

            for loop_val in loops {
                let seg_indices: Vec<usize> = loop_val.get("segments")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
                    .unwrap_or_default();

                if seg_indices.is_empty() { continue; }

                let mut commands = Vec::new();

                for (i, &seg_idx) in seg_indices.iter().enumerate() {
                    if seg_idx >= segments.len() { continue; }
                    let seg = &segments[seg_idx];

                    if seg.start_vertex >= vertices.len() || seg.end_vertex >= vertices.len() {
                        continue;
                    }

                    let (sx_v, sy_v) = vertices[seg.start_vertex];
                    let (ex_v, ey_v) = vertices[seg.end_vertex];

                    if i == 0 {
                        commands.push(PathCommand::MoveTo(Vec2::new(sx_v, sy_v)));
                    }

                    let has_curves = seg.start_dx != 0.0 || seg.start_dy != 0.0
                        || seg.end_dx != 0.0 || seg.end_dy != 0.0;

                    if has_curves {
                        // Cubic bezier: control1 = start_vertex + start_tangent, control2 = end_vertex + end_tangent
                        commands.push(PathCommand::CubicTo {
                            control1: Vec2::new(sx_v + seg.start_dx, sy_v + seg.start_dy),
                            control2: Vec2::new(ex_v + seg.end_dx, ey_v + seg.end_dy),
                            to: Vec2::new(ex_v, ey_v),
                        });
                    } else {
                        commands.push(PathCommand::LineTo(Vec2::new(ex_v, ey_v)));
                    }
                }

                commands.push(PathCommand::Close);

                paths.push(VectorPath {
                    commands,
                    fill_rule: FillRule::NonZero,
                });
            }
        }
    } else if !segments.is_empty() {
        // No regions — just draw all segments as one open path
        let mut commands = Vec::new();
        let mut last_vertex = usize::MAX;

        for seg in &segments {
            if seg.start_vertex >= vertices.len() || seg.end_vertex >= vertices.len() {
                continue;
            }
            let (sx_v, sy_v) = vertices[seg.start_vertex];
            let (ex_v, ey_v) = vertices[seg.end_vertex];

            if seg.start_vertex != last_vertex {
                commands.push(PathCommand::MoveTo(Vec2::new(sx_v, sy_v)));
            }

            let has_curves = seg.start_dx != 0.0 || seg.start_dy != 0.0
                || seg.end_dx != 0.0 || seg.end_dy != 0.0;

            if has_curves {
                commands.push(PathCommand::CubicTo {
                    control1: Vec2::new(sx_v + seg.start_dx, sy_v + seg.start_dy),
                    control2: Vec2::new(ex_v + seg.end_dx, ey_v + seg.end_dy),
                    to: Vec2::new(ex_v, ey_v),
                });
            } else {
                commands.push(PathCommand::LineTo(Vec2::new(ex_v, ey_v)));
            }

            last_vertex = seg.end_vertex;
        }

        if !commands.is_empty() {
            paths.push(VectorPath {
                commands,
                fill_rule: FillRule::NonZero,
            });
        }
    }

    paths
}

fn parse_blend_mode(s: &str) -> BlendMode {
    match s {
        "MULTIPLY" => BlendMode::Multiply,
        "SCREEN" => BlendMode::Screen,
        "OVERLAY" => BlendMode::Overlay,
        "DARKEN" => BlendMode::Darken,
        "LIGHTEN" => BlendMode::Lighten,
        "COLOR_DODGE" => BlendMode::ColorDodge,
        "COLOR_BURN" => BlendMode::ColorBurn,
        "HARD_LIGHT" => BlendMode::HardLight,
        "SOFT_LIGHT" => BlendMode::SoftLight,
        "DIFFERENCE" => BlendMode::Difference,
        "EXCLUSION" => BlendMode::Exclusion,
        "HUE" => BlendMode::Hue,
        "SATURATION" => BlendMode::Saturation,
        "COLOR" => BlendMode::ColorMode,
        "LUMINOSITY" => BlendMode::Luminosity,
        _ => BlendMode::Normal,
    }
}
