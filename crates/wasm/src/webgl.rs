//! WebGL2 instanced rendering backend.
//!
//! Batches all visible Rects into one draw call and all Ellipses into another.
//! Each shape is an instanced quad with per-instance position, size, color, and
//! transform data. The camera is a uniform mat3 applied in the vertex shader.
//!
//! This gives 10-50x throughput over Canvas2D for data-dense scenes (10K+ shapes).
//! Text, Image, and Path shapes are skipped (fall back to Canvas2D overlay).

use rendero_renderer::scene::{RenderItem, RenderShape, AABB};
use rendero_core::properties::Paint;
use web_sys::WebGl2RenderingContext as GL;
use wasm_bindgen::JsCast;

// ── Shader sources ──────────────────────────────────────────────────────

const RECT_VS: &str = r#"#version 300 es
precision highp float;

// Per-vertex: quad corner position (0..1, 0..1)
layout(location = 0) in vec2 a_quad;

// Per-instance attributes
layout(location = 1) in vec4 a_xywh;      // x, y, width, height (world space)
layout(location = 2) in vec4 a_color;      // r, g, b, a
layout(location = 3) in vec4 a_transform;  // a, b, c, d of 2x3 affine
layout(location = 4) in vec2 a_translate;  // tx, ty of 2x3 affine
layout(location = 5) in vec4 a_radii;      // corner radii: tl, tr, br, bl

uniform vec2 u_resolution;
uniform vec3 u_camera; // cam_x, cam_y, zoom

out vec4 v_color;
out vec2 v_uv;
out vec4 v_radii;
out vec2 v_size;

void main() {
    float zoom = u_camera.z;
    vec2 cam = u_camera.xy;

    // Local position within the shape
    vec2 local = a_quad * a_xywh.zw;

    // Apply world transform (2x3 affine matrix)
    vec2 world = vec2(
        a_transform.x * local.x + a_transform.z * local.y + a_translate.x,
        a_transform.y * local.x + a_transform.w * local.y + a_translate.y
    );

    // Apply camera: screen = (world - cam) * zoom
    vec2 screen = (world - cam) * zoom;

    // Convert to clip space: [-1, 1]
    vec2 ndc = (screen / u_resolution) * 2.0 - 1.0;
    ndc.y = -ndc.y; // flip Y (screen coords: Y down, GL: Y up)

    gl_Position = vec4(ndc, 0.0, 1.0);
    v_color = a_color;
    v_uv = a_quad;
    v_radii = a_radii * zoom; // scale radii to screen pixels
    v_size = a_xywh.zw * zoom; // screen-space size
}
"#;

const RECT_FS: &str = r#"#version 300 es
precision highp float;

in vec4 v_color;
in vec2 v_uv;
in vec4 v_radii; // tl, tr, br, bl in screen pixels
in vec2 v_size;  // screen-space width, height

out vec4 fragColor;

float roundedBoxSDF(vec2 p, vec2 b, vec4 r) {
    // Select corner radius based on quadrant
    float rx = (p.x > 0.0) ? ((p.y > 0.0) ? r.z : r.y) : ((p.y > 0.0) ? r.w : r.x);
    vec2 q = abs(p) - b + rx;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - rx;
}

void main() {
    vec2 halfSize = v_size * 0.5;
    vec2 p = (v_uv - 0.5) * v_size; // centered pixel coords

    float d = roundedBoxSDF(p, halfSize, v_radii);
    float aa = fwidth(d);
    float alpha = 1.0 - smoothstep(-aa, aa, d);

    fragColor = vec4(v_color.rgb * v_color.a * alpha, v_color.a * alpha);
}
"#;

const ELLIPSE_VS: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec2 a_quad;

layout(location = 1) in vec4 a_xywh;
layout(location = 2) in vec4 a_color;
layout(location = 3) in vec4 a_transform;
layout(location = 4) in vec2 a_translate;

uniform vec2 u_resolution;
uniform vec3 u_camera;

out vec4 v_color;
out vec2 v_uv;
out vec2 v_size;

void main() {
    float zoom = u_camera.z;
    vec2 cam = u_camera.xy;

    vec2 local = a_quad * a_xywh.zw;
    vec2 world = vec2(
        a_transform.x * local.x + a_transform.z * local.y + a_translate.x,
        a_transform.y * local.x + a_transform.w * local.y + a_translate.y
    );
    vec2 screen = (world - cam) * zoom;
    vec2 ndc = (screen / u_resolution) * 2.0 - 1.0;
    ndc.y = -ndc.y;

    gl_Position = vec4(ndc, 0.0, 1.0);
    v_color = a_color;
    v_uv = a_quad;
    v_size = a_xywh.zw * zoom;
}
"#;

const ELLIPSE_FS: &str = r#"#version 300 es
precision highp float;

in vec4 v_color;
in vec2 v_uv;
in vec2 v_size;

out vec4 fragColor;

void main() {
    vec2 p = v_uv * 2.0 - 1.0; // -1..1
    float d = dot(p, p); // distance squared from center
    float aa = fwidth(d);
    float alpha = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);

    fragColor = vec4(v_color.rgb * v_color.a * alpha, v_color.a * alpha);
}
"#;

// ── Point cloud shaders (simplified: no per-instance transform) ─────────

const POINT_CLOUD_VS: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec2 a_quad;
layout(location = 1) in vec4 a_xywh;   // x, y, width, height (world space, pre-positioned)
layout(location = 2) in vec4 a_color;   // r, g, b, a

uniform vec2 u_resolution;
uniform vec3 u_camera; // cam_x, cam_y, zoom

out vec4 v_color;
out vec2 v_uv;

void main() {
    float zoom = u_camera.z;
    vec2 cam = u_camera.xy;

    vec2 screen_size = a_xywh.zw * zoom;

    // LOD: sub-pixel points → clip (zero fragment work)
    if (screen_size.x < 0.5 && screen_size.y < 0.5) {
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        return;
    }

    vec2 screen_pos = (a_xywh.xy - cam) * zoom;

    // Viewport cull: off-screen → clip
    if (screen_pos.x + screen_size.x < 0.0 || screen_pos.y + screen_size.y < 0.0 ||
        screen_pos.x > u_resolution.x || screen_pos.y > u_resolution.y) {
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        return;
    }

    vec2 screen = screen_pos + a_quad * screen_size;
    vec2 ndc = (screen / u_resolution) * 2.0 - 1.0;
    ndc.y = -ndc.y;

    gl_Position = vec4(ndc, 0.0, 1.0);
    v_color = a_color;
    v_uv = a_quad;
}
"#;

const POINT_CLOUD_FS: &str = r#"#version 300 es
precision highp float;

in vec4 v_color;
in vec2 v_uv;

out vec4 fragColor;

void main() {
    vec2 p = v_uv * 2.0 - 1.0;
    float d = dot(p, p);
    float aa = fwidth(d);
    float alpha = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);
    fragColor = vec4(v_color.rgb * v_color.a * alpha, v_color.a * alpha);
}
"#;

const POINT_CLOUD_FLOATS_PER_INSTANCE: usize = 8; // x, y, w, h, r, g, b, a

// ── GPU state ───────────────────────────────────────────────────────────

/// Cached GPU state for a WebGL2 context. Created once, reused across frames.
#[allow(dead_code)] // VBOs kept alive so VAOs reference valid buffers
pub struct WebGlState {
    rect_program: web_sys::WebGlProgram,
    ellipse_program: web_sys::WebGlProgram,
    quad_vbo: web_sys::WebGlBuffer,
    rect_instance_vbo: web_sys::WebGlBuffer,
    ellipse_instance_vbo: web_sys::WebGlBuffer,
    rect_vao: web_sys::WebGlVertexArrayObject,
    ellipse_vao: web_sys::WebGlVertexArrayObject,
    // Uniform locations
    rect_u_resolution: web_sys::WebGlUniformLocation,
    rect_u_camera: web_sys::WebGlUniformLocation,
    ellipse_u_resolution: web_sys::WebGlUniformLocation,
    ellipse_u_camera: web_sys::WebGlUniformLocation,
    // Point cloud shader
    pc_program: web_sys::WebGlProgram,
    pc_u_resolution: web_sys::WebGlUniformLocation,
    pc_u_camera: web_sys::WebGlUniformLocation,
}

impl WebGlState {
    pub fn new(gl: &GL) -> Result<Self, String> {
        let rect_program = compile_program(gl, RECT_VS, RECT_FS)?;
        let ellipse_program = compile_program(gl, ELLIPSE_VS, ELLIPSE_FS)?;

        // Unit quad: two triangles covering [0,0] to [1,1]
        let quad_data: [f32; 12] = [
            0.0, 0.0,  1.0, 0.0,  0.0, 1.0,
            1.0, 0.0,  1.0, 1.0,  0.0, 1.0,
        ];
        let quad_vbo = create_buffer(gl, &quad_data)?;

        let rect_instance_vbo = gl.create_buffer().ok_or("Failed to create rect instance buffer")?;
        let ellipse_instance_vbo = gl.create_buffer().ok_or("Failed to create ellipse instance buffer")?;

        // ── Rect VAO ──
        let rect_vao = gl.create_vertex_array().ok_or("Failed to create rect VAO")?;
        gl.bind_vertex_array(Some(&rect_vao));
        setup_vao(gl, &quad_vbo, &rect_instance_vbo, true);
        gl.bind_vertex_array(None);

        // ── Ellipse VAO ──
        let ellipse_vao = gl.create_vertex_array().ok_or("Failed to create ellipse VAO")?;
        gl.bind_vertex_array(Some(&ellipse_vao));
        setup_vao(gl, &quad_vbo, &ellipse_instance_vbo, false);
        gl.bind_vertex_array(None);

        // ── Uniform locations ──
        let rect_u_resolution = gl.get_uniform_location(&rect_program, "u_resolution")
            .ok_or("rect: u_resolution not found")?;
        let rect_u_camera = gl.get_uniform_location(&rect_program, "u_camera")
            .ok_or("rect: u_camera not found")?;
        let ellipse_u_resolution = gl.get_uniform_location(&ellipse_program, "u_resolution")
            .ok_or("ellipse: u_resolution not found")?;
        let ellipse_u_camera = gl.get_uniform_location(&ellipse_program, "u_camera")
            .ok_or("ellipse: u_camera not found")?;

        // ── Point cloud shader ──
        let pc_program = compile_program(gl, POINT_CLOUD_VS, POINT_CLOUD_FS)?;
        let pc_u_resolution = gl.get_uniform_location(&pc_program, "u_resolution")
            .ok_or("pc: u_resolution not found")?;
        let pc_u_camera = gl.get_uniform_location(&pc_program, "u_camera")
            .ok_or("pc: u_camera not found")?;

        Ok(Self {
            rect_program,
            ellipse_program,
            quad_vbo,
            rect_instance_vbo,
            ellipse_instance_vbo,
            rect_vao,
            ellipse_vao,
            rect_u_resolution,
            rect_u_camera,
            ellipse_u_resolution,
            ellipse_u_camera,
            pc_program,
            pc_u_resolution,
            pc_u_camera,
        })
    }
}

// ── Render entry point ──────────────────────────────────────────────────

/// Render visible items using WebGL2 instanced drawing.
/// Returns the number of items drawn.
pub fn render_webgl(
    gl: &GL,
    state: &WebGlState,
    items: &[RenderItem],
    spatial_grid: &std::collections::HashMap<(i32, i32), Vec<(usize, usize)>>,
    grid_cell_size: f32,
    width: f64,
    height: f64,
    cam_x: f64,
    cam_y: f64,
    zoom: f64,
    dpr: f64,
) -> usize {
    gl.viewport(0, 0, (width * dpr) as i32, (height * dpr) as i32);
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear(GL::COLOR_BUFFER_BIT);

    gl.enable(GL::BLEND);
    gl.blend_func_separate(
        GL::ONE, GL::ONE_MINUS_SRC_ALPHA,      // premultiplied alpha RGB
        GL::ONE, GL::ONE_MINUS_SRC_ALPHA,       // alpha channel
    );

    if items.is_empty() {
        return 0;
    }

    // ── Collect visible items into batches ──
    let mut rect_instances: Vec<f32> = Vec::with_capacity(4096);
    let mut ellipse_instances: Vec<f32> = Vec::with_capacity(4096);
    let mut drawn = 0usize;

    if !spatial_grid.is_empty() {
        // Spatial grid path: only visit visible cells
        // Draw root
        drawn += collect_item_range(
            items, 0, 1,
            width, height, cam_x, cam_y, zoom,
            &mut rect_instances, &mut ellipse_instances,
        );

        let vp_left = cam_x as f32;
        let vp_top = cam_y as f32;
        let vp_right = cam_x as f32 + (width / zoom) as f32;
        let vp_bottom = cam_y as f32 + (height / zoom) as f32;

        let col_min = (vp_left / grid_cell_size).floor() as i32;
        let col_max = (vp_right / grid_cell_size).floor() as i32;
        let row_min = (vp_top / grid_cell_size).floor() as i32;
        let row_max = (vp_bottom / grid_cell_size).floor() as i32;

        let mut seen = std::collections::HashSet::new();
        for row in row_min..=row_max {
            for col in col_min..=col_max {
                if let Some(entries) = spatial_grid.get(&(col, row)) {
                    for &(start, end) in entries {
                        if seen.insert(start) {
                            drawn += collect_item_range(
                                items, start, end,
                                width, height, cam_x, cam_y, zoom,
                                &mut rect_instances, &mut ellipse_instances,
                            );
                        }
                    }
                }
            }
        }
    } else {
        drawn = collect_item_range(
            items, 0, items.len(),
            width, height, cam_x, cam_y, zoom,
            &mut rect_instances, &mut ellipse_instances,
        );
    }

    // ── Draw rect batch ──
    let rect_count = rect_instances.len() / RECT_FLOATS_PER_INSTANCE;
    if rect_count > 0 {
        gl.use_program(Some(&state.rect_program));
        gl.uniform2f(Some(&state.rect_u_resolution), width as f32, height as f32);
        gl.uniform3f(Some(&state.rect_u_camera), cam_x as f32, cam_y as f32, zoom as f32);

        gl.bind_vertex_array(Some(&state.rect_vao));
        upload_instance_data(gl, &state.rect_instance_vbo, &rect_instances);
        gl.draw_arrays_instanced(GL::TRIANGLES, 0, 6, rect_count as i32);
        gl.bind_vertex_array(None);
    }

    // ── Draw ellipse batch ──
    let ellipse_count = ellipse_instances.len() / ELLIPSE_FLOATS_PER_INSTANCE;
    if ellipse_count > 0 {
        gl.use_program(Some(&state.ellipse_program));
        gl.uniform2f(Some(&state.ellipse_u_resolution), width as f32, height as f32);
        gl.uniform3f(Some(&state.ellipse_u_camera), cam_x as f32, cam_y as f32, zoom as f32);

        gl.bind_vertex_array(Some(&state.ellipse_vao));
        upload_instance_data(gl, &state.ellipse_instance_vbo, &ellipse_instances);
        gl.draw_arrays_instanced(GL::TRIANGLES, 0, 6, ellipse_count as i32);
        gl.bind_vertex_array(None);
    }

    drawn
}

// ── Per-instance data layout ────────────────────────────────────────────

// Rect: xywh(4) + color(4) + transform(4) + translate(2) + radii(4) = 18 floats
const RECT_FLOATS_PER_INSTANCE: usize = 18;
// Ellipse: xywh(4) + color(4) + transform(4) + translate(2) = 14 floats
const ELLIPSE_FLOATS_PER_INSTANCE: usize = 14;

// ── Collect visible items into instance buffers ─────────────────────────

fn collect_item_range(
    items: &[RenderItem],
    start: usize,
    end: usize,
    width: f64,
    height: f64,
    cam_x: f64,
    cam_y: f64,
    zoom: f64,
    rects: &mut Vec<f32>,
    ellipses: &mut Vec<f32>,
) -> usize {
    let mut drawn = 0usize;
    let mut i = start;

    while i < end {
        let item = &items[i];

        // Screen-space bounds
        let sx_min = (item.world_bounds.min.x as f64 - cam_x) * zoom;
        let sy_min = (item.world_bounds.min.y as f64 - cam_y) * zoom;
        let sx_max = (item.world_bounds.max.x as f64 - cam_x) * zoom;
        let sy_max = (item.world_bounds.max.y as f64 - cam_y) * zoom;
        let on_screen = sx_max >= 0.0 && sy_max >= 0.0 && sx_min <= width && sy_min <= height;
        let screen_w = sx_max - sx_min;
        let screen_h = sy_max - sy_min;

        // Hierarchical LOD: subtree < 50px → draw parent fill only, skip descendants
        if item.descendant_count > 0 && screen_w < 50.0 && screen_h < 50.0 {
            if on_screen && !item.style.fills.is_empty() {
                push_item(item, rects, ellipses);
                drawn += 1;
            }
            i += 1 + item.descendant_count;
            continue;
        }

        // Leaf LOD: < 0.5px → invisible
        if screen_w < 0.5 && screen_h < 0.5 {
            i += 1;
            continue;
        }

        if !on_screen {
            if item.descendant_count > 0 && item.clips {
                i += 1 + item.descendant_count;
                continue;
            }
            i += 1;
            continue;
        }

        // Skip mask nodes, text, image, path (unsupported in WebGL batch)
        if item.is_mask {
            i += 1;
            continue;
        }

        match &item.shape {
            RenderShape::Text { .. } | RenderShape::Image { .. } | RenderShape::Path { .. } | RenderShape::Line { .. } => {
                // These fall back to Canvas2D overlay
                i += 1;
                continue;
            }
            _ => {}
        }

        if !item.style.fills.is_empty() {
            push_item(item, rects, ellipses);
            drawn += 1;
        }

        i += 1;
    }

    drawn
}

/// Push a single item's instance data to the appropriate batch.
fn push_item(item: &RenderItem, rects: &mut Vec<f32>, ellipses: &mut Vec<f32>) {
    let t = &item.world_transform;

    // Extract first solid fill color (or white fallback)
    let (r, g, b, a) = extract_color(&item.style.fills, item.style.opacity);

    match &item.shape {
        RenderShape::Rect { width, height, corner_radii } => {
            let (tl, tr, br, bl) = match corner_radii {
                rendero_core::node::CornerRadii::Uniform(rv) => (*rv, *rv, *rv, *rv),
                rendero_core::node::CornerRadii::PerCorner { top_left, top_right, bottom_right, bottom_left } =>
                    (*top_left, *top_right, *bottom_right, *bottom_left),
            };
            // xywh
            rects.extend_from_slice(&[0.0, 0.0, *width, *height]);
            // color
            rects.extend_from_slice(&[r, g, b, a]);
            // transform a,b,c,d
            rects.extend_from_slice(&[t.a, t.b, t.c, t.d]);
            // translate tx,ty
            rects.extend_from_slice(&[t.tx, t.ty]);
            // radii
            rects.extend_from_slice(&[tl, tr, br, bl]);
        }
        RenderShape::Ellipse { width, height, .. } => {
            // xywh
            ellipses.extend_from_slice(&[0.0, 0.0, *width, *height]);
            // color
            ellipses.extend_from_slice(&[r, g, b, a]);
            // transform a,b,c,d
            ellipses.extend_from_slice(&[t.a, t.b, t.c, t.d]);
            // translate tx,ty
            ellipses.extend_from_slice(&[t.tx, t.ty]);
        }
        _ => {} // Line, Path, Text, Image handled by Canvas2D
    }
}

/// Extract a solid color from the first fill, applying opacity.
fn extract_color(fills: &[Paint], opacity: f32) -> (f32, f32, f32, f32) {
    for paint in fills {
        match paint {
            Paint::Solid(c) => {
                return (c.r(), c.g(), c.b(), c.a() * opacity);
            }
            Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } => {
                // Use first stop color as approximation
                if let Some(stop) = stops.first() {
                    return (stop.color.r(), stop.color.g(), stop.color.b(), stop.color.a() * opacity);
                }
            }
            _ => {}
        }
    }
    (1.0, 1.0, 1.0, opacity) // fallback white
}

// ── WebGL helpers ───────────────────────────────────────────────────────

fn compile_shader(gl: &GL, shader_type: u32, source: &str) -> Result<web_sys::WebGlShader, String> {
    let shader = gl.create_shader(shader_type).ok_or("Failed to create shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl.get_shader_parameter(&shader, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(shader)
    } else {
        let log = gl.get_shader_info_log(&shader).unwrap_or_default();
        gl.delete_shader(Some(&shader));
        Err(format!("Shader compile error: {}", log))
    }
}

fn compile_program(gl: &GL, vs_src: &str, fs_src: &str) -> Result<web_sys::WebGlProgram, String> {
    let vs = compile_shader(gl, GL::VERTEX_SHADER, vs_src)?;
    let fs = compile_shader(gl, GL::FRAGMENT_SHADER, fs_src)?;
    let program = gl.create_program().ok_or("Failed to create program")?;
    gl.attach_shader(&program, &vs);
    gl.attach_shader(&program, &fs);
    gl.link_program(&program);

    gl.delete_shader(Some(&vs));
    gl.delete_shader(Some(&fs));

    if gl.get_program_parameter(&program, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(program)
    } else {
        let log = gl.get_program_info_log(&program).unwrap_or_default();
        gl.delete_program(Some(&program));
        Err(format!("Program link error: {}", log))
    }
}

fn create_buffer(gl: &GL, data: &[f32]) -> Result<web_sys::WebGlBuffer, String> {
    let buffer = gl.create_buffer().ok_or("Failed to create buffer")?;
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));
    unsafe {
        let view = js_sys::Float32Array::view(data);
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
    }
    Ok(buffer)
}

fn upload_instance_data(gl: &GL, buffer: &web_sys::WebGlBuffer, data: &[f32]) {
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(buffer));
    unsafe {
        let view = js_sys::Float32Array::view(data);
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::DYNAMIC_DRAW);
    }
}

/// Set up VAO attribute pointers for a quad VBO + instance VBO.
/// `has_radii`: true for rects (18 floats/instance), false for ellipses (14 floats/instance).
fn setup_vao(gl: &GL, quad_vbo: &web_sys::WebGlBuffer, instance_vbo: &web_sys::WebGlBuffer, has_radii: bool) {
    let stride = if has_radii {
        RECT_FLOATS_PER_INSTANCE as i32 * 4
    } else {
        ELLIPSE_FLOATS_PER_INSTANCE as i32 * 4
    };

    // Attribute 0: quad vertex position (per-vertex)
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(quad_vbo));
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
    // divisor 0 = per-vertex (default)

    // Per-instance attributes
    gl.bind_buffer(GL::ARRAY_BUFFER, Some(instance_vbo));

    // Attribute 1: xywh (4 floats) — offset 0
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_with_i32(1, 4, GL::FLOAT, false, stride, 0);
    gl.vertex_attrib_divisor(1, 1);

    // Attribute 2: color (4 floats) — offset 16
    gl.enable_vertex_attrib_array(2);
    gl.vertex_attrib_pointer_with_i32(2, 4, GL::FLOAT, false, stride, 16);
    gl.vertex_attrib_divisor(2, 1);

    // Attribute 3: transform a,b,c,d (4 floats) — offset 32
    gl.enable_vertex_attrib_array(3);
    gl.vertex_attrib_pointer_with_i32(3, 4, GL::FLOAT, false, stride, 32);
    gl.vertex_attrib_divisor(3, 1);

    // Attribute 4: translate tx,ty (2 floats) — offset 48
    gl.enable_vertex_attrib_array(4);
    gl.vertex_attrib_pointer_with_i32(4, 2, GL::FLOAT, false, stride, 48);
    gl.vertex_attrib_divisor(4, 1);

    if has_radii {
        // Attribute 5: corner radii (4 floats) — offset 56
        gl.enable_vertex_attrib_array(5);
        gl.vertex_attrib_pointer_with_i32(5, 4, GL::FLOAT, false, stride, 56);
        gl.vertex_attrib_divisor(5, 1);
    }
}

// ── Point cloud: GPU-direct rendering bypassing the scene graph ────────
//
// Architecture: spatial chunking + LOD decimation.
//
// On creation, points are counting-sorted into a spatial grid. The sorted data
// goes into a single GPU buffer (STATIC_DRAW). Each grid cell is a contiguous
// range within that buffer. Per frame, only cells overlapping the viewport are
// drawn — each as a separate drawArraysInstanced call with an attribute pointer
// offset. A decimated LOD buffer (every 16th point) is used at very low zoom.
//
// Memory: N × 32 bytes GPU (main) + N/16 × 32 bytes GPU (LOD) + ~0 WASM steady state.
// Draw calls per frame: ~visible_cells (typically 50-300) or 1 (LOD mode).

/// Metadata for one spatial grid cell: a contiguous range in the sorted buffer.
struct CellRange {
    offset: u32,  // start index in points (not floats)
    count: u32,   // number of points in this cell
}

/// A spatially-chunked, GPU-resident point cloud.
pub struct PointCloud {
    pub total_points: u32,
    // Main data: single VBO with points sorted by cell
    vao: web_sys::WebGlVertexArrayObject,
    vbo: web_sys::WebGlBuffer,
    uploaded: bool,
    pending_data: Option<Vec<f32>>,
    // Spatial grid
    cells: Vec<CellRange>,
    grid_cols: u32,
    grid_rows: u32,
    cell_w: f32,
    cell_h: f32,
    origin_x: f32,
    origin_y: f32,
    // LOD1: every 16th point
    lod_vao: web_sys::WebGlVertexArrayObject,
    lod_vbo: web_sys::WebGlBuffer,
    lod_count: u32,
    lod_uploaded: bool,
    lod_pending: Option<Vec<f32>>,
    // LOD2: every 256th point (for 50M+ datasets)
    lod2_vao: web_sys::WebGlVertexArrayObject,
    lod2_vbo: web_sys::WebGlBuffer,
    lod2_count: u32,
    lod2_uploaded: bool,
    lod2_pending: Option<Vec<f32>>,
}

impl PointCloud {
    /// Create a point cloud from packed data: [x, y, w, h, r, g, b, a] × N.
    /// Sorts points into a spatial grid and builds a decimated LOD buffer.
    pub fn new(gl: &GL, state: &WebGlState, data: Vec<f32>) -> Result<Self, String> {
        let stride = POINT_CLOUD_FLOATS_PER_INSTANCE;
        let n = data.len() / stride;
        if n == 0 { return Err("Empty point cloud data".into()); }

        // ── 1. Find world bounds ──
        let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
        let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
        for i in 0..n {
            let x = data[i * stride];
            let y = data[i * stride + 1];
            min_x = min_x.min(x); min_y = min_y.min(y);
            max_x = max_x.max(x); max_y = max_y.max(y);
        }
        let world_w = (max_x - min_x).max(1.0);
        let world_h = (max_y - min_y).max(1.0);

        // ── 2. Choose grid dimensions (target ~20K points per cell) ──
        let target_per_cell = 20_000.0f32;
        let target_cells = (n as f32 / target_per_cell).max(4.0);
        let aspect = world_w / world_h;
        let rows = (target_cells / aspect).sqrt().max(1.0).ceil() as u32;
        let cols = (rows as f32 * aspect).max(1.0).ceil() as u32;
        let cell_w = world_w / cols as f32;
        let cell_h = world_h / rows as f32;
        let total_cells = (cols * rows) as usize;

        // ── 3. Counting sort ──
        let mut counts = vec![0u32; total_cells];
        for i in 0..n {
            let x = data[i * stride];
            let y = data[i * stride + 1];
            let col = ((x - min_x) / cell_w).floor().min(cols as f32 - 1.0).max(0.0) as u32;
            let row = ((y - min_y) / cell_h).floor().min(rows as f32 - 1.0).max(0.0) as u32;
            counts[(row * cols + col) as usize] += 1;
        }

        // Prefix sums → write offsets
        let mut offsets = vec![0u32; total_cells + 1];
        for i in 0..total_cells {
            offsets[i + 1] = offsets[i] + counts[i];
        }

        // Place points into sorted array
        let mut sorted = vec![0.0f32; data.len()];
        let mut write_pos = offsets[..total_cells].to_vec();
        for i in 0..n {
            let base = i * stride;
            let x = data[base];
            let y = data[base + 1];
            let col = ((x - min_x) / cell_w).floor().min(cols as f32 - 1.0).max(0.0) as u32;
            let row = ((y - min_y) / cell_h).floor().min(rows as f32 - 1.0).max(0.0) as u32;
            let cell = (row * cols + col) as usize;
            let dest = write_pos[cell] as usize * stride;
            sorted[dest..dest + stride].copy_from_slice(&data[base..base + stride]);
            write_pos[cell] += 1;
        }
        drop(data); // free unsorted data

        // Build cell ranges
        let cells: Vec<CellRange> = (0..total_cells).map(|i| CellRange {
            offset: offsets[i],
            count: counts[i],
        }).collect();

        // ── 4. Build LOD levels ──
        // LOD1: every 16th point
        let lod_data: Vec<f32> = (0..n).step_by(16)
            .flat_map(|i| sorted[i * stride..(i + 1) * stride].iter().copied())
            .collect();
        let lod_count = (lod_data.len() / stride) as u32;

        // LOD2: every 256th point (for very large datasets)
        let lod2_data: Vec<f32> = (0..n).step_by(256)
            .flat_map(|i| sorted[i * stride..(i + 1) * stride].iter().copied())
            .collect();
        let lod2_count = (lod2_data.len() / stride) as u32;

        // ── 5. Create GPU objects ──
        let byte_stride = (stride * 4) as i32;

        // Helper closure to set up a VAO with quad + instance attributes
        let setup_lod_vao = |gl: &GL, label: &str| -> Result<(web_sys::WebGlVertexArrayObject, web_sys::WebGlBuffer), String> {
            let vao = gl.create_vertex_array().ok_or(format!("PC: {} VAO", label))?;
            let vbo = gl.create_buffer().ok_or(format!("PC: {} VBO", label))?;
            gl.bind_vertex_array(Some(&vao));
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&state.quad_vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_with_i32(1, 4, GL::FLOAT, false, byte_stride, 0);
            gl.vertex_attrib_divisor(1, 1);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_with_i32(2, 4, GL::FLOAT, false, byte_stride, 16);
            gl.vertex_attrib_divisor(2, 1);
            gl.bind_vertex_array(None);
            Ok((vao, vbo))
        };

        // Main VAO (same layout but will repoint per-cell during render)
        let vao = gl.create_vertex_array().ok_or("PC: VAO")?;
        let vbo = gl.create_buffer().ok_or("PC: VBO")?;
        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&state.quad_vbo));
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 0, 0);
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 4, GL::FLOAT, false, byte_stride, 0);
        gl.vertex_attrib_divisor(1, 1);
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_with_i32(2, 4, GL::FLOAT, false, byte_stride, 16);
        gl.vertex_attrib_divisor(2, 1);
        gl.bind_vertex_array(None);

        let (lod_vao, lod_vbo) = setup_lod_vao(gl, "LOD1")?;
        let (lod2_vao, lod2_vbo) = setup_lod_vao(gl, "LOD2")?;

        Ok(Self {
            total_points: n as u32,
            vao, vbo, uploaded: false, pending_data: Some(sorted),
            cells, grid_cols: cols, grid_rows: rows,
            cell_w, cell_h, origin_x: min_x, origin_y: min_y,
            lod_vao, lod_vbo, lod_count, lod_uploaded: false, lod_pending: Some(lod_data),
            lod2_vao, lod2_vbo, lod2_count, lod2_uploaded: false, lod2_pending: Some(lod2_data),
        })
    }

    fn upload_main(&mut self, gl: &GL) {
        if self.uploaded { return; }
        if let Some(data) = self.pending_data.take() {
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
            unsafe {
                let view = js_sys::Float32Array::view(&data);
                gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
            }
            self.uploaded = true;
        }
    }

    fn upload_lod(&mut self, gl: &GL) {
        if self.lod_uploaded { return; }
        if let Some(data) = self.lod_pending.take() {
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.lod_vbo));
            unsafe {
                let view = js_sys::Float32Array::view(&data);
                gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
            }
            self.lod_uploaded = true;
        }
    }

    fn upload_lod2(&mut self, gl: &GL) {
        if self.lod2_uploaded { return; }
        if let Some(data) = self.lod2_pending.take() {
            gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.lod2_vbo));
            unsafe {
                let view = js_sys::Float32Array::view(&data);
                gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
            }
            self.lod2_uploaded = true;
        }
    }

    /// Delete GPU resources.
    pub fn delete(&self, gl: &GL) {
        gl.delete_buffer(Some(&self.vbo));
        gl.delete_vertex_array(Some(&self.vao));
        gl.delete_buffer(Some(&self.lod_vbo));
        gl.delete_vertex_array(Some(&self.lod_vao));
        gl.delete_buffer(Some(&self.lod2_vbo));
        gl.delete_vertex_array(Some(&self.lod2_vao));
    }
}

/// Render all point clouds with spatial culling and LOD. Returns total points drawn.
pub fn render_point_clouds(
    gl: &GL,
    state: &WebGlState,
    clouds: &mut [PointCloud],
    width: f64,
    height: f64,
    cam_x: f64,
    cam_y: f64,
    zoom: f64,
    dpr: f64,
) -> usize {
    if clouds.is_empty() { return 0; }

    gl.use_program(Some(&state.pc_program));
    gl.uniform2f(Some(&state.pc_u_resolution), width as f32, height as f32);
    gl.uniform3f(Some(&state.pc_u_camera), cam_x as f32, cam_y as f32, zoom as f32);

    let stride = POINT_CLOUD_FLOATS_PER_INSTANCE;
    let byte_stride = (stride * 4) as i32;

    let vp_left = cam_x as f32;
    let vp_top = cam_y as f32;
    let vp_right = cam_x as f32 + width as f32 / zoom as f32;
    let vp_bottom = cam_y as f32 + height as f32 / zoom as f32;

    let mut total = 0usize;

    // ── Pass 1: estimate aggregate visible points across all clouds ──
    let mut aggregate_visible = 0.0f32;
    for cloud in clouds.iter() {
        let col_min = ((vp_left - cloud.origin_x) / cloud.cell_w).floor().max(0.0) as u32;
        let col_max = ((vp_right - cloud.origin_x) / cloud.cell_w).ceil().min(cloud.grid_cols as f32) as u32;
        let row_min = ((vp_top - cloud.origin_y) / cloud.cell_h).floor().max(0.0) as u32;
        let row_max = ((vp_bottom - cloud.origin_y) / cloud.cell_h).ceil().min(cloud.grid_rows as f32) as u32;
        let visible_cells = (col_max.saturating_sub(col_min)) as f32
            * (row_max.saturating_sub(row_min)) as f32;
        let total_cells = (cloud.grid_cols * cloud.grid_rows) as f32;
        let visible_frac = (visible_cells / total_cells.max(1.0)).min(1.0);
        aggregate_visible += cloud.total_points as f32 * visible_frac;
    }

    // Choose LOD level: target <700K drawn points for 60+ FPS
    // 0 = full resolution (cell-culled), 1 = every 16th, 2 = every 256th
    // LOD2 when LOD1 would still exceed 700K (aggregate/16 > 700K → aggregate > 11.2M)
    // LOD1 when full resolution exceeds 700K
    let lod_level = if aggregate_visible > 11_200_000.0 { 2u8 }
                    else if aggregate_visible > 700_000.0 { 1u8 }
                    else { 0u8 };

    // ── Pass 2: render each cloud at chosen LOD ──
    for cloud in clouds.iter_mut() {
        if lod_level == 2 {
            cloud.upload_lod2(gl);
            gl.bind_vertex_array(Some(&cloud.lod2_vao));
            gl.draw_arrays_instanced(GL::TRIANGLES, 0, 6, cloud.lod2_count as i32);
            total += cloud.lod2_count as usize;
            continue;
        }

        if lod_level == 1 {
            cloud.upload_lod(gl);
            gl.bind_vertex_array(Some(&cloud.lod_vao));
            gl.draw_arrays_instanced(GL::TRIANGLES, 0, 6, cloud.lod_count as i32);
            total += cloud.lod_count as usize;
            continue;
        }

        // Full resolution: upload main buffer and draw visible cells
        cloud.upload_main(gl);

        let col_min = ((vp_left - cloud.origin_x) / cloud.cell_w).floor().max(0.0) as u32;
        let col_max = ((vp_right - cloud.origin_x) / cloud.cell_w).ceil().min(cloud.grid_cols as f32) as u32;
        let row_min = ((vp_top - cloud.origin_y) / cloud.cell_h).floor().max(0.0) as u32;
        let row_max = ((vp_bottom - cloud.origin_y) / cloud.cell_h).ceil().min(cloud.grid_rows as f32) as u32;

        gl.bind_vertex_array(Some(&cloud.vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&cloud.vbo));

        for row in row_min..row_max {
            for col in col_min..col_max {
                let cell_idx = (row * cloud.grid_cols + col) as usize;
                if cell_idx >= cloud.cells.len() { continue; }
                let cell = &cloud.cells[cell_idx];
                if cell.count == 0 { continue; }

                let byte_offset = cell.offset as i32 * byte_stride;
                gl.vertex_attrib_pointer_with_i32(1, 4, GL::FLOAT, false, byte_stride, byte_offset);
                gl.vertex_attrib_pointer_with_i32(2, 4, GL::FLOAT, false, byte_stride, byte_offset + 16);
                gl.draw_arrays_instanced(GL::TRIANGLES, 0, 6, cell.count as i32);
                total += cell.count as usize;
            }
        }
    }

    gl.bind_vertex_array(None);
    total
}
