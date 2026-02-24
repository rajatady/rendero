//! Canvas 2D vector rendering — draws RenderItems directly to a browser canvas.
//! GPU-accelerated via the browser's Canvas 2D API through web-sys.
//! Replaces the CPU tile-based rasterizer for on-screen rendering.

use wasm_bindgen::JsCast;
use rendero_core::node::{CornerRadii, PathCommand, TextAlign};
use rendero_core::properties::{BlendMode, Color, Effect, FillRule, Paint, StrokeCap, StrokeJoin};
use rendero_renderer::scene::{RenderItem, RenderShape, AABB};
use web_sys::CanvasRenderingContext2d;

/// Render world-space RenderItems with camera transform applied via Canvas 2D.
/// Uses a spatial grid to find visible artboards in O(viewport_cells) instead of O(total_artboards).
/// Returns number of items actually drawn (for diagnostics).
pub fn render_items_with_camera(
    ctx: &CanvasRenderingContext2d,
    items: &[RenderItem],
    spatial_grid: &std::collections::HashMap<(i32, i32), Vec<(usize, usize)>>,
    grid_cell_size: f32,
    width: f64,
    height: f64,
    cam_x: f64,
    cam_y: f64,
    zoom: f64,
) -> usize {
    ctx.clear_rect(0.0, 0.0, width, height);

    if items.is_empty() {
        return 0;
    }

    // If spatial grid is populated, use it for O(1) viewport lookups
    if !spatial_grid.is_empty() {
        let mut drawn = 0usize;

        // Draw root item (page background)
        drawn += render_item_range(ctx, items, 0, 1, width, height, cam_x, cam_y, zoom);

        // Compute world-space viewport
        let vp_left = cam_x as f32;
        let vp_top = cam_y as f32;
        let vp_right = cam_x as f32 + (width / zoom) as f32;
        let vp_bottom = cam_y as f32 + (height / zoom) as f32;

        // Find grid cells that overlap the viewport
        let col_min = (vp_left / grid_cell_size).floor() as i32;
        let col_max = (vp_right / grid_cell_size).floor() as i32;
        let row_min = (vp_top / grid_cell_size).floor() as i32;
        let row_max = (vp_bottom / grid_cell_size).floor() as i32;

        // Collect unique artboard ranges from visible cells
        // Use a small set to deduplicate (artboards can span multiple cells)
        let mut seen = std::collections::HashSet::new();
        for row in row_min..=row_max {
            for col in col_min..=col_max {
                if let Some(entries) = spatial_grid.get(&(col, row)) {
                    for &(start, end) in entries {
                        if seen.insert(start) {
                            drawn += render_item_range(
                                ctx, items, start, end,
                                width, height, cam_x, cam_y, zoom,
                            );
                        }
                    }
                }
            }
        }
        return drawn;
    }

    // Fallback: iterate all items (small documents without grid)
    render_item_range(ctx, items, 0, items.len(), width, height, cam_x, cam_y, zoom)
}

/// Render a contiguous range of items [start..end) with full LOD/culling.
fn render_item_range(
    ctx: &CanvasRenderingContext2d,
    items: &[RenderItem],
    start: usize,
    end: usize,
    width: f64,
    height: f64,
    cam_x: f64,
    cam_y: f64,
    zoom: f64,
) -> usize {
    let mut drawn = 0usize;
    let mut clip_stack: Vec<usize> = Vec::new();
    let mut i = start;

    while i < end {
        let item = &items[i];

        // Pop any expired clip regions
        while let Some(clip_end) = clip_stack.last() {
            if i >= *clip_end {
                clip_stack.pop();
                ctx.restore();
            } else {
                break;
            }
        }

        // Screen-space bounds check
        let sx_min = (item.world_bounds.min.x as f64 - cam_x) * zoom;
        let sy_min = (item.world_bounds.min.y as f64 - cam_y) * zoom;
        let sx_max = (item.world_bounds.max.x as f64 - cam_x) * zoom;
        let sy_max = (item.world_bounds.max.y as f64 - cam_y) * zoom;
        let on_screen = sx_max >= 0.0 && sy_max >= 0.0 && sx_min <= width && sy_min <= height;
        let screen_w = sx_max - sx_min;
        let screen_h = sy_max - sy_min;

        // Hierarchical LOD: if a subtree is small on screen,
        // draw just the frame background and skip ALL descendants.
        if item.descendant_count > 0 && screen_w < 50.0 && screen_h < 50.0 {
            if on_screen && !item.style.fills.is_empty() {
                drawn += 1;
                ctx.save();
                let t = &item.world_transform;
                let _ = ctx.set_transform(
                    t.a as f64 * zoom, t.b as f64 * zoom,
                    t.c as f64 * zoom, t.d as f64 * zoom,
                    (t.tx as f64 - cam_x) * zoom, (t.ty as f64 - cam_y) * zoom,
                );
                set_fill_style(ctx, item.style.fills.first().unwrap(), &item.shape);
                draw_shape(ctx, &item.shape, false);
                ctx.restore();
            }
            i += 1 + item.descendant_count;
            continue;
        }

        // LOD: skip leaf shapes smaller than 0.5px on screen (sub-pixel = invisible).
        if screen_w < 0.5 && screen_h < 0.5 && !item.clips {
            i += 1;
            continue;
        }

        if !on_screen {
            // Off-screen container with clipping: skip subtree
            if item.descendant_count > 0 && item.clips {
                i += 1 + item.descendant_count;
                continue;
            }
            i += 1;
            continue;
        }

        drawn += 1;

        // Mask node: use its shape as a clip path for subsequent siblings.
        // Don't draw the mask itself — it only defines the clipping region.
        if item.is_mask {
            ctx.save();
            let t = &item.world_transform;
            let _ = ctx.set_transform(
                t.a as f64 * zoom, t.b as f64 * zoom,
                t.c as f64 * zoom, t.d as f64 * zoom,
                (t.tx as f64 - cam_x) * zoom, (t.ty as f64 - cam_y) * zoom,
            );
            build_clip_path(ctx, &item.shape);
            ctx.clip();
            // Clip stays active until parent's descendant range ends.
            // Find the enclosing parent clip end, or use the section end.
            let clip_end = clip_stack.last().copied().unwrap_or(end);
            clip_stack.push(clip_end);
            i += 1;
            continue;
        }

        ctx.save();

        // Apply world transform + camera in one setTransform call
        let t = &item.world_transform;
        let _ = ctx.set_transform(
            t.a as f64 * zoom, t.b as f64 * zoom,
            t.c as f64 * zoom, t.d as f64 * zoom,
            (t.tx as f64 - cam_x) * zoom, (t.ty as f64 - cam_y) * zoom,
        );

        if item.style.opacity < 1.0 {
            ctx.set_global_alpha(item.style.opacity as f64);
        }

        if !matches!(item.style.blend_mode, BlendMode::Normal) {
            ctx.set_global_composite_operation(blend_mode_to_composite(&item.style.blend_mode))
                .unwrap_or(());
        }

        apply_effects(ctx, &item.style.effects);

        if matches!(&item.shape, RenderShape::Text { .. } | RenderShape::Image { .. }) {
            draw_shape(ctx, &item.shape, false);
        } else {
            for paint in &item.style.fills {
                set_fill_style(ctx, paint, &item.shape);
                draw_shape(ctx, &item.shape, false);
            }
        }

        if !item.style.strokes.is_empty() && item.style.stroke_weight > 0.0 {
            ctx.set_line_width(item.style.stroke_weight as f64);
            ctx.set_line_cap(match item.style.stroke_cap {
                StrokeCap::None => "butt",
                StrokeCap::Round => "round",
                StrokeCap::Square => "square",
            });
            ctx.set_line_join(match item.style.stroke_join {
                StrokeJoin::Miter => "miter",
                StrokeJoin::Round => "round",
                StrokeJoin::Bevel => "bevel",
            });
            if !item.style.dash_pattern.is_empty() {
                let dashes = js_sys::Array::new_with_length(item.style.dash_pattern.len() as u32);
                for (j, &d) in item.style.dash_pattern.iter().enumerate() {
                    dashes.set(j as u32, wasm_bindgen::JsValue::from_f64(d as f64));
                }
                let _ = ctx.set_line_dash(&dashes);
            }
            for paint in &item.style.strokes {
                set_stroke_style(ctx, paint, &item.shape);
                draw_shape(ctx, &item.shape, true);
            }
        }

        // Draw inner shadows after fills/strokes
        apply_inner_shadows(ctx, &item.style.effects, &item.shape);

        clear_effects(ctx);
        ctx.restore();

        // Set up clip region for children
        let shape_finite = match &item.shape {
            RenderShape::Rect { width, height, .. } => width.is_finite() && height.is_finite(),
            RenderShape::Ellipse { width, height, .. } => width.is_finite() && height.is_finite(),
            _ => true,
        };
        if item.clips && item.descendant_count > 0 && shape_finite {
            ctx.save();
            let t = &item.world_transform;
            let _ = ctx.set_transform(
                t.a as f64 * zoom, t.b as f64 * zoom,
                t.c as f64 * zoom, t.d as f64 * zoom,
                (t.tx as f64 - cam_x) * zoom, (t.ty as f64 - cam_y) * zoom,
            );
            build_clip_path(ctx, &item.shape);
            ctx.clip();
            clip_stack.push(i + 1 + item.descendant_count);
        }

        i += 1;
    }

    for _ in &clip_stack {
        ctx.restore();
    }

    drawn
}

/// Draw the selection overlay for a selected node.
pub fn draw_selection(
    ctx: &CanvasRenderingContext2d,
    x: f64, y: f64, w: f64, h: f64,
) {
    ctx.save();
    let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    // Selection border
    ctx.set_stroke_style_str("#4285f4");
    ctx.set_line_width(2.0);
    ctx.stroke_rect(x, y, w, h);

    // Corner handles
    let handle_size = 8.0;
    let half = handle_size / 2.0;
    ctx.set_fill_style_str("#ffffff");
    ctx.set_stroke_style_str("#4285f4");
    ctx.set_line_width(1.5);
    let corners = [
        (x - half, y - half),
        (x + w - half, y - half),
        (x - half, y + h - half),
        (x + w - half, y + h - half),
    ];
    for (cx, cy) in corners {
        ctx.fill_rect(cx, cy, handle_size, handle_size);
        ctx.stroke_rect(cx, cy, handle_size, handle_size);
    }

    // Edge midpoint handles
    let mid_handles = [
        (x + w / 2.0 - half, y - half),           // top
        (x + w / 2.0 - half, y + h - half),        // bottom
        (x - half, y + h / 2.0 - half),            // left
        (x + w - half, y + h / 2.0 - half),        // right
    ];
    for (mx, my) in mid_handles {
        ctx.fill_rect(mx, my, handle_size, handle_size);
        ctx.stroke_rect(mx, my, handle_size, handle_size);
    }

    ctx.restore();
}

fn color_to_css(c: &Color) -> String {
    format!(
        "rgba({},{},{},{})",
        (c.r() * 255.0) as u8,
        (c.g() * 255.0) as u8,
        (c.b() * 255.0) as u8,
        c.a()
    )
}

fn shape_dimensions(shape: &RenderShape) -> (f64, f64) {
    match shape {
        RenderShape::Rect { width, height, .. } => (*width as f64, *height as f64),
        RenderShape::Ellipse { width, height, .. } => (*width as f64, *height as f64),
        RenderShape::Line { length, .. } => (*length as f64, 1.0),
        _ => (1.0, 1.0),
    }
}

fn set_fill_style(ctx: &CanvasRenderingContext2d, paint: &Paint, shape: &RenderShape) {
    match paint {
        Paint::Solid(c) => {
            ctx.set_fill_style_str(&color_to_css(c));
        }
        Paint::LinearGradient { stops, start, end } => {
            let (w, h) = shape_dimensions(shape);
            let gradient = ctx.create_linear_gradient(
                start.x as f64 * w, start.y as f64 * h,
                end.x as f64 * w, end.y as f64 * h,
            );
            for stop in stops {
                let _ = gradient.add_color_stop(
                    stop.position,
                    &color_to_css(&stop.color),
                );
            }
            ctx.set_fill_style_canvas_gradient(&gradient);
        }
        Paint::RadialGradient { stops, center, radius } => {
            let (w, h) = shape_dimensions(shape);
            if let Ok(gradient) = ctx.create_radial_gradient(
                center.x as f64 * w, center.y as f64 * h, 0.0,
                center.x as f64 * w, center.y as f64 * h, *radius as f64 * w.max(h),
            ) {
                for stop in stops {
                    let _ = gradient.add_color_stop(
                        stop.position,
                        &color_to_css(&stop.color),
                    );
                }
                ctx.set_fill_style_canvas_gradient(&gradient);
            }
        }
        Paint::Image { opacity, .. } => {
            let a = (*opacity * 255.0) as u8;
            ctx.set_fill_style_str(&format!("rgba(200,200,200,{})", a as f32 / 255.0));
        }
    }
}

fn set_stroke_style(ctx: &CanvasRenderingContext2d, paint: &Paint, shape: &RenderShape) {
    match paint {
        Paint::Solid(c) => {
            ctx.set_stroke_style_str(&color_to_css(c));
        }
        Paint::LinearGradient { stops, start, end } => {
            let (w, h) = shape_dimensions(shape);
            let gradient = ctx.create_linear_gradient(
                start.x as f64 * w, start.y as f64 * h,
                end.x as f64 * w, end.y as f64 * h,
            );
            for stop in stops {
                let _ = gradient.add_color_stop(
                    stop.position,
                    &color_to_css(&stop.color),
                );
            }
            ctx.set_stroke_style_canvas_gradient(&gradient);
        }
        Paint::RadialGradient { stops, center, radius } => {
            let (w, h) = shape_dimensions(shape);
            if let Ok(gradient) = ctx.create_radial_gradient(
                center.x as f64 * w, center.y as f64 * h, 0.0,
                center.x as f64 * w, center.y as f64 * h, *radius as f64 * w.max(h),
            ) {
                for stop in stops {
                    let _ = gradient.add_color_stop(
                        stop.position,
                        &color_to_css(&stop.color),
                    );
                }
                ctx.set_stroke_style_canvas_gradient(&gradient);
            }
        }
        Paint::Image { .. } => {
            // Image strokes not supported
        }
    }
}

fn apply_effects(ctx: &CanvasRenderingContext2d, effects: &[Effect]) {
    for effect in effects {
        match effect {
            Effect::DropShadow { color, offset, blur_radius, .. } => {
                ctx.set_shadow_color(&color_to_css(color));
                ctx.set_shadow_blur(*blur_radius as f64);
                ctx.set_shadow_offset_x(offset.x as f64);
                ctx.set_shadow_offset_y(offset.y as f64);
            }
            Effect::LayerBlur { radius } | Effect::BackgroundBlur { radius } => {
                let _ = js_sys::Reflect::set(
                    ctx.as_ref(),
                    &wasm_bindgen::JsValue::from_str("filter"),
                    &wasm_bindgen::JsValue::from_str(&format!("blur({}px)", radius)),
                );
            }
            _ => {}
        }
    }
}

fn blend_mode_to_composite(mode: &BlendMode) -> &'static str {
    match mode {
        BlendMode::Normal => "source-over",
        BlendMode::Multiply => "multiply",
        BlendMode::Screen => "screen",
        BlendMode::Overlay => "overlay",
        BlendMode::Darken => "darken",
        BlendMode::Lighten => "lighten",
        BlendMode::ColorDodge => "color-dodge",
        BlendMode::ColorBurn => "color-burn",
        BlendMode::HardLight => "hard-light",
        BlendMode::SoftLight => "soft-light",
        BlendMode::Difference => "difference",
        BlendMode::Exclusion => "exclusion",
        BlendMode::Hue => "hue",
        BlendMode::Saturation => "saturation",
        BlendMode::ColorMode => "color",
        BlendMode::Luminosity => "luminosity",
    }
}

fn clear_effects(ctx: &CanvasRenderingContext2d) {
    ctx.set_shadow_color("transparent");
    ctx.set_shadow_blur(0.0);
    ctx.set_shadow_offset_x(0.0);
    ctx.set_shadow_offset_y(0.0);
    let _ = js_sys::Reflect::set(
        ctx.as_ref(),
        &wasm_bindgen::JsValue::from_str("filter"),
        &wasm_bindgen::JsValue::from_str("none"),
    );
}

/// Draw inner shadows by clipping to the shape, then drawing a large rect outside
/// that casts its shadow inward through the clip boundary.
fn apply_inner_shadows(ctx: &CanvasRenderingContext2d, effects: &[Effect], shape: &RenderShape) {
    for effect in effects {
        if let Effect::InnerShadow { color, offset, blur_radius, spread } = effect {
            ctx.save();

            // Build clip path from shape
            match shape {
                RenderShape::Rect { width, height, corner_radii } => {
                    let w = *width as f64;
                    let h = *height as f64;
                    let (tl, tr, br, bl) = match corner_radii {
                        CornerRadii::Uniform(r) => {
                            let r = *r as f64;
                            (r, r, r, r)
                        }
                        CornerRadii::PerCorner { top_left, top_right, bottom_right, bottom_left } =>
                            (*top_left as f64, *top_right as f64, *bottom_right as f64, *bottom_left as f64),
                    };
                    ctx.begin_path();
                    ctx.move_to(tl, 0.0);
                    ctx.line_to(w - tr, 0.0);
                    ctx.quadratic_curve_to(w, 0.0, w, tr);
                    ctx.line_to(w, h - br);
                    ctx.quadratic_curve_to(w, h, w - br, h);
                    ctx.line_to(bl, h);
                    ctx.quadratic_curve_to(0.0, h, 0.0, h - bl);
                    ctx.line_to(0.0, tl);
                    ctx.quadratic_curve_to(0.0, 0.0, tl, 0.0);
                    ctx.close_path();
                }
                RenderShape::Ellipse { width, height, .. } => {
                    let rx = *width as f64 / 2.0;
                    let ry = *height as f64 / 2.0;
                    ctx.begin_path();
                    let _ = ctx.ellipse(rx, ry, rx, ry, 0.0, 0.0, std::f64::consts::TAU);
                    ctx.close_path();
                }
                _ => {
                    ctx.restore();
                    continue;
                }
            }
            ctx.clip();

            // Set shadow properties
            ctx.set_shadow_color(&color_to_css(color));
            ctx.set_shadow_blur((*blur_radius + *spread) as f64);
            ctx.set_shadow_offset_x(offset.x as f64);
            ctx.set_shadow_offset_y(offset.y as f64);

            // Fill a large rect around (but not overlapping) the shape.
            // The shadow from this rect bleeds inward through the clip.
            ctx.set_fill_style_str(&color_to_css(color));
            let big = 10000.0;
            ctx.fill_rect(-big, -big, big * 3.0, big * 3.0);

            ctx.restore();
        }
    }
}

fn draw_shape(ctx: &CanvasRenderingContext2d, shape: &RenderShape, stroke_only: bool) {
    match shape {
        RenderShape::Rect { width, height, corner_radii } => {
            draw_rect(ctx, *width as f64, *height as f64, corner_radii, stroke_only);
        }
        RenderShape::Ellipse { width, height, arc_start, arc_end, inner_radius_ratio } => {
            draw_ellipse(ctx, *width as f64, *height as f64, *arc_start, *arc_end, *inner_radius_ratio, stroke_only);
        }
        RenderShape::Path { commands, fill_rule } => {
            draw_path(ctx, commands, *fill_rule, stroke_only);
        }
        RenderShape::Text { runs, width, height: _, align, .. } => {
            if !stroke_only {
                draw_text(ctx, runs, *align, *width as f64);
            }
        }
        RenderShape::Image { width, height, data, image_width, image_height } => {
            if !stroke_only {
                draw_image(ctx, *width as f64, *height as f64, data, *image_width, *image_height);
            }
        }
        RenderShape::Line { length } => {
            draw_line(ctx, *length as f64, stroke_only);
        }
    }
}

fn draw_rect(ctx: &CanvasRenderingContext2d, w: f64, h: f64, radii: &CornerRadii, stroke_only: bool) {
    match radii {
        CornerRadii::Uniform(r) if *r <= 0.0 => {
            if stroke_only { ctx.stroke_rect(0.0, 0.0, w, h); }
            else { ctx.fill_rect(0.0, 0.0, w, h); }
        }
        _ => {
            let (tl, tr, br, bl) = match radii {
                CornerRadii::Uniform(r) => (*r as f64, *r as f64, *r as f64, *r as f64),
                CornerRadii::PerCorner { top_left, top_right, bottom_right, bottom_left } =>
                    (*top_left as f64, *top_right as f64, *bottom_right as f64, *bottom_left as f64),
            };
            ctx.begin_path();
            ctx.move_to(tl, 0.0);
            ctx.line_to(w - tr, 0.0);
            ctx.quadratic_curve_to(w, 0.0, w, tr);
            ctx.line_to(w, h - br);
            ctx.quadratic_curve_to(w, h, w - br, h);
            ctx.line_to(bl, h);
            ctx.quadratic_curve_to(0.0, h, 0.0, h - bl);
            ctx.line_to(0.0, tl);
            ctx.quadratic_curve_to(0.0, 0.0, tl, 0.0);
            ctx.close_path();
            if stroke_only { ctx.stroke(); }
            else { ctx.fill(); }
        }
    }
}

fn draw_ellipse(
    ctx: &CanvasRenderingContext2d,
    w: f64, h: f64,
    arc_start: f32, arc_end: f32,
    inner_radius_ratio: f32,
    stroke_only: bool,
) {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let rx = w / 2.0;
    let ry = h / 2.0;

    ctx.begin_path();
    let _ = ctx.ellipse(cx, cy, rx, ry, 0.0, arc_start as f64, arc_end as f64);

    if inner_radius_ratio > 0.0 {
        let irx = rx * inner_radius_ratio as f64;
        let iry = ry * inner_radius_ratio as f64;
        let _ = ctx.ellipse(cx, cy, irx, iry, 0.0, arc_end as f64, arc_start as f64);
    }

    if stroke_only { ctx.stroke(); }
    else { ctx.fill(); }
}

fn draw_path(
    ctx: &CanvasRenderingContext2d,
    commands: &[PathCommand],
    fill_rule: FillRule,
    stroke_only: bool,
) {
    ctx.begin_path();
    for cmd in commands {
        match cmd {
            PathCommand::MoveTo(p) => ctx.move_to(p.x as f64, p.y as f64),
            PathCommand::LineTo(p) => ctx.line_to(p.x as f64, p.y as f64),
            PathCommand::CubicTo { control1, control2, to } => {
                ctx.bezier_curve_to(
                    control1.x as f64, control1.y as f64,
                    control2.x as f64, control2.y as f64,
                    to.x as f64, to.y as f64,
                );
            }
            PathCommand::QuadTo { control, to } => {
                ctx.quadratic_curve_to(
                    control.x as f64, control.y as f64,
                    to.x as f64, to.y as f64,
                );
            }
            PathCommand::Close => ctx.close_path(),
        }
    }
    if stroke_only {
        ctx.stroke();
    } else {
        match fill_rule {
            FillRule::EvenOdd => {
                let _ = ctx.fill_with_canvas_winding_rule(
                    web_sys::CanvasWindingRule::Evenodd,
                );
            }
            FillRule::NonZero => {
                ctx.fill();
            }
        }
    }
}

fn draw_text(
    ctx: &CanvasRenderingContext2d,
    runs: &[rendero_core::node::TextRun],
    align: TextAlign,
    width: f64,
) {
    let text_align = match align {
        TextAlign::Left => "left",
        TextAlign::Center => "center",
        TextAlign::Right => "right",
        TextAlign::Justified => "left",
    };
    ctx.set_text_align(text_align);
    ctx.set_text_baseline("top");

    let x_start = match align {
        TextAlign::Center => width / 2.0,
        TextAlign::Right => width,
        _ => 0.0,
    };

    let mut y_offset = 0.0;
    for run in runs {
        // Build CSS font string with numeric weight for full granularity
        let style = if run.italic { "italic " } else { "" };
        let font = format!("{}{} {:.0}px '{}', 'SF Pro Display', 'SF Pro Text', system-ui, -apple-system, 'Helvetica Neue', sans-serif",
            style, run.font_weight, run.font_size, run.font_family);
        ctx.set_font(&font);
        ctx.set_fill_style_str(&color_to_css(&run.color));

        // Line height: explicit value or 1.2× font size
        let line_h = run.line_height.unwrap_or(run.font_size * 1.2) as f64;

        // Split on explicit newlines and render each line
        for (line_idx, line) in run.text.split('\n').enumerate() {
            if line_idx > 0 {
                y_offset += line_h;
            }
            if !line.is_empty() {
                // Word-wrap within the container width
                let words: Vec<&str> = line.split(' ').collect();
                let mut current_line = String::new();
                let mut first_word_in_line = true;

                for word in &words {
                    let test_line = if first_word_in_line {
                        word.to_string()
                    } else {
                        format!("{} {}", current_line, word)
                    };

                    let measured = ctx.measure_text(&test_line).map(|m| m.width()).unwrap_or(0.0);

                    if !first_word_in_line && measured > width && !current_line.is_empty() {
                        // Emit the current line
                        let _ = ctx.fill_text(&current_line, x_start, y_offset);
                        y_offset += line_h;
                        current_line = word.to_string();
                    } else {
                        current_line = test_line;
                    }
                    first_word_in_line = false;
                }

                // Emit the last line
                if !current_line.is_empty() {
                    let _ = ctx.fill_text(&current_line, x_start, y_offset);
                }
            }
        }
        y_offset += line_h;
    }
}

fn draw_image(
    ctx: &CanvasRenderingContext2d,
    w: f64, h: f64,
    data: &[u8],
    image_width: u32, image_height: u32,
) {
    let clamped = wasm_bindgen::Clamped(data);
    let img_data = match web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        clamped, image_width, image_height,
    ) {
        Ok(d) => d,
        Err(_) => return,
    };

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let tmp = match document.create_element("canvas") {
        Ok(el) => el,
        Err(_) => return,
    };
    let tmp_canvas: web_sys::HtmlCanvasElement = match tmp.dyn_into() {
        Ok(c) => c,
        Err(_) => return,
    };
    tmp_canvas.set_width(image_width);
    tmp_canvas.set_height(image_height);
    let tmp_ctx = match tmp_canvas.get_context("2d") {
        Ok(Some(c)) => match c.dyn_into::<CanvasRenderingContext2d>() {
            Ok(ctx) => ctx,
            Err(_) => return,
        },
        _ => return,
    };
    let _ = tmp_ctx.put_image_data(&img_data, 0.0, 0.0);

    let _ = ctx.draw_image_with_html_canvas_element_and_dw_and_dh(
        &tmp_canvas, 0.0, 0.0, w, h,
    );
}

fn draw_line(ctx: &CanvasRenderingContext2d, length: f64, _stroke_only: bool) {
    ctx.begin_path();
    ctx.move_to(0.0, 0.0);
    ctx.line_to(length, 0.0);
    ctx.stroke();
}

/// Build a clip path matching the shape (used for frame clipping).
fn build_clip_path(ctx: &CanvasRenderingContext2d, shape: &RenderShape) {
    match shape {
        RenderShape::Rect { width, height, corner_radii } => {
            let w = *width as f64;
            let h = *height as f64;
            ctx.begin_path();
            match corner_radii {
                CornerRadii::Uniform(r) if *r <= 0.0 => {
                    ctx.rect(0.0, 0.0, w, h);
                }
                _ => {
                    let (tl, tr, br, bl) = match corner_radii {
                        CornerRadii::Uniform(r) => (*r as f64, *r as f64, *r as f64, *r as f64),
                        CornerRadii::PerCorner { top_left, top_right, bottom_right, bottom_left } =>
                            (*top_left as f64, *top_right as f64, *bottom_right as f64, *bottom_left as f64),
                    };
                    ctx.move_to(tl, 0.0);
                    ctx.line_to(w - tr, 0.0);
                    ctx.quadratic_curve_to(w, 0.0, w, tr);
                    ctx.line_to(w, h - br);
                    ctx.quadratic_curve_to(w, h, w - br, h);
                    ctx.line_to(bl, h);
                    ctx.quadratic_curve_to(0.0, h, 0.0, h - bl);
                    ctx.line_to(0.0, tl);
                    ctx.quadratic_curve_to(0.0, 0.0, tl, 0.0);
                    ctx.close_path();
                }
            }
        }
        RenderShape::Ellipse { width, height, .. } => {
            let cx = *width as f64 / 2.0;
            let cy = *height as f64 / 2.0;
            ctx.begin_path();
            let _ = ctx.ellipse(cx, cy, cx, cy, 0.0, 0.0, std::f64::consts::TAU);
        }
        RenderShape::Path { commands, .. } => {
            ctx.begin_path();
            for cmd in commands {
                match cmd {
                    PathCommand::MoveTo(p) => ctx.move_to(p.x as f64, p.y as f64),
                    PathCommand::LineTo(p) => ctx.line_to(p.x as f64, p.y as f64),
                    PathCommand::CubicTo { control1, control2, to } => {
                        ctx.bezier_curve_to(
                            control1.x as f64, control1.y as f64,
                            control2.x as f64, control2.y as f64,
                            to.x as f64, to.y as f64,
                        );
                    }
                    PathCommand::QuadTo { control, to } => {
                        ctx.quadratic_curve_to(
                            control.x as f64, control.y as f64,
                            to.x as f64, to.y as f64,
                        );
                    }
                    PathCommand::Close => ctx.close_path(),
                }
            }
        }
        _ => {
            // Line, Text, Image — use bounding rect as fallback clip
            ctx.begin_path();
            ctx.rect(0.0, 0.0, 10000.0, 10000.0);
        }
    }
}
