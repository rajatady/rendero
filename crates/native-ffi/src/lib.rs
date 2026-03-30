//! Native FFI bridge — C-ABI exports for macOS/iOS/Linux.
//!
//! Same engine as the WASM version, but accessible via `extern "C"` functions
//! instead of `wasm_bindgen`. Swift/Kotlin/C++ call these through the C bridge.
//!
//! The DOM shim's `engine-native.js` calls these via JavaScriptCore.

use std::ffi::{CStr, c_char, c_void};

use rendero_core::document::Document;
use rendero_core::id::NodeId;
use rendero_core::node::{Node, NodeKind};
use rendero_core::properties::*;
use rendero_renderer::pipeline;
use rendero_renderer::scene::{AABB, RenderItem};
use glam::Vec2;

/// The native engine — same as CanvasEngine in WASM but without wasm_bindgen.
struct NativeEngine {
    document: Document,
    selected: Vec<NodeId>,
    viewport_width: u32,
    viewport_height: u32,
    cam_x: f32,
    cam_y: f32,
    cam_zoom: f32,
    insert_parent: Option<NodeId>,
    current_page: usize,
    scene_cache: Option<Vec<RenderItem>>,
}

impl NativeEngine {
    fn new(name: &str, client_id: u32) -> Self {
        Self {
            document: Document::new(name, client_id),
            selected: Vec::new(),
            viewport_width: 800,
            viewport_height: 600,
            cam_x: 0.0,
            cam_y: 0.0,
            cam_zoom: 1.0,
            insert_parent: None,
            current_page: 0,
            scene_cache: None,
        }
    }

    fn effective_parent(&self) -> NodeId {
        self.insert_parent.unwrap_or_else(|| {
            self.document.page(self.current_page).unwrap().tree.root_id()
        })
    }

    fn invalidate_cache(&mut self) {
        self.scene_cache = None;
    }

    fn node_world_pos(&self, node_id: &NodeId) -> (f32, f32) {
        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return (0.0, 0.0),
        };
        let mut wx = 0.0f32;
        let mut wy = 0.0f32;
        let mut cur = *node_id;
        loop {
            let node = match page.tree.get(&cur) {
                Some(n) => n,
                None => break,
            };
            wx += node.transform.tx;
            wy += node.transform.ty;
            match page.tree.parent_of(&cur) {
                Some(p) => cur = p,
                None => break,
            }
        }
        (wx, wy)
    }
}

// ─── Helper: C string → &str ───

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() { return ""; }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

/// Pack (counter, client_id) into a single u64 for return over FFI.
fn pack_id(counter: u64, client_id: u32) -> u64 {
    (counter << 32) | (client_id as u64)
}

// ═══════════════════════════════════════════════════════════════
// Exported C-ABI functions
// ═══════════════════════════════════════════════════════════════

#[no_mangle]
pub extern "C" fn rendero_create(name: *const c_char, client_id: u32) -> *mut c_void {
    let name = unsafe { cstr_to_str(name) };
    let engine = Box::new(NativeEngine::new(name, client_id));
    Box::into_raw(engine) as *mut c_void
}

#[no_mangle]
pub extern "C" fn rendero_destroy(engine: *mut c_void) {
    if !engine.is_null() {
        unsafe { drop(Box::from_raw(engine as *mut NativeEngine)); }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_viewport(engine: *mut c_void, w: u32, h: u32) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    e.viewport_width = w;
    e.viewport_height = h;
}

#[no_mangle]
pub extern "C" fn rendero_set_camera(engine: *mut c_void, x: f32, y: f32, zoom: f32) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    e.cam_x = x;
    e.cam_y = y;
    e.cam_zoom = zoom;
}

#[no_mangle]
pub extern "C" fn rendero_get_camera(engine: *mut c_void, out: *mut f32) {
    let e = unsafe { &*(engine as *const NativeEngine) };
    unsafe {
        *out.offset(0) = e.cam_x;
        *out.offset(1) = e.cam_y;
        *out.offset(2) = e.cam_zoom;
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_insert_parent(engine: *mut c_void, counter: u32, client_id: u32) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    e.insert_parent = Some(NodeId::new(counter as u64, client_id));
}

#[no_mangle]
pub extern "C" fn rendero_clear_insert_parent(engine: *mut c_void) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    e.insert_parent = None;
}

#[no_mangle]
pub extern "C" fn rendero_add_frame(
    engine: *mut c_void, name: *const c_char,
    x: f32, y: f32, w: f32, h: f32,
    r: f32, g: f32, b: f32, a: f32,
) -> u64 {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let name = unsafe { cstr_to_str(name) };

    let id = e.document.next_id();
    let mut node = Node::frame(id, name, w, h);
    node.transform = Transform::translate(x, y);
    if a > 0.0 {
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));
    }

    let parent_id = e.effective_parent();
    e.document.add_node(e.current_page, node, parent_id, usize::MAX)
        .expect("add_frame insert failed");
    e.invalidate_cache();

    pack_id(id.0.counter, id.0.client_id)
}

#[no_mangle]
pub extern "C" fn rendero_add_text(
    engine: *mut c_void, name: *const c_char, text: *const c_char,
    x: f32, y: f32, font_size: f32,
    r: f32, g: f32, b: f32, a: f32,
) -> u64 {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let name = unsafe { cstr_to_str(name) };
    let text = unsafe { cstr_to_str(text) };

    let id = e.document.next_id();
    let color = Color::new(r, g, b, a);
    let mut node = Node::text(id, name, text, font_size, color);
    node.transform = Transform::translate(x, y);

    let parent_id = e.effective_parent();
    e.document.add_node(e.current_page, node, parent_id, usize::MAX)
        .expect("add_text insert failed");
    e.invalidate_cache();

    pack_id(id.0.counter, id.0.client_id)
}

#[no_mangle]
pub extern "C" fn rendero_set_node_position(
    engine: *mut c_void, counter: u32, client_id: u32, x: f32, y: f32,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            node.transform.tx = x;
            node.transform.ty = y;
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_node_size(
    engine: *mut c_void, counter: u32, client_id: u32, w: f32, h: f32,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            node.width = w;
            node.height = h;
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_node_fill(
    engine: *mut c_void, counter: u32, client_id: u32,
    r: f32, g: f32, b: f32, a: f32,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            let color = Color::new(r, g, b, a);
            if let NodeKind::Text { ref mut runs, .. } = node.kind {
                for run in runs.iter_mut() {
                    run.color = color;
                }
            }
            node.style.fills = vec![Paint::Solid(color)];
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_node_corner_radius(
    engine: *mut c_void, counter: u32, client_id: u32,
    tl: f32, tr: f32, br: f32, bl: f32,
) {
    use rendero_core::node::CornerRadii;
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            let radii = if tl == tr && tr == br && br == bl {
                CornerRadii::Uniform(tl)
            } else {
                CornerRadii::PerCorner { top_left: tl, top_right: tr, bottom_right: br, bottom_left: bl }
            };
            if let NodeKind::Frame { ref mut corner_radii, .. } = node.kind {
                *corner_radii = radii;
            }
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_node_opacity(
    engine: *mut c_void, counter: u32, client_id: u32, opacity: f32,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            node.style.opacity = opacity.clamp(0.0, 1.0);
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_node_text(
    engine: *mut c_void, counter: u32, client_id: u32, text: *const c_char,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let text = unsafe { cstr_to_str(text) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            if let NodeKind::Text { ref mut runs, .. } = node.kind {
                if let Some(run) = runs.first_mut() {
                    run.text = text.to_string();
                    // Recalculate text bounds
                    let (w, h) = // Same heuristic as WASM version
                    (text.len() as f32 * run.font_size * 0.65,
                     run.font_size * 1.5);
                    node.width = w;
                    node.height = h;
                }
            }
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_node_font_size(
    engine: *mut c_void, counter: u32, client_id: u32, size: f32,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            if let NodeKind::Text { ref mut runs, .. } = node.kind {
                for run in runs.iter_mut() {
                    run.font_size = size;
                }
                // Recalculate bounds
                if let Some(run) = runs.first() {
                    let (w, h) = (
                        run.text.len() as f32 * size * 0.65,
                        size * 1.5,
                    );
                    node.width = w;
                    node.height = h;
                }
            }
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_node_font_weight(
    engine: *mut c_void, counter: u32, client_id: u32, weight: u16,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            if let NodeKind::Text { ref mut runs, .. } = node.kind {
                for run in runs.iter_mut() {
                    run.font_weight = weight;
                }
            }
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_set_auto_layout(
    engine: *mut c_void, counter: u32, client_id: u32,
    direction: u32, spacing: f32,
    pad_top: f32, pad_right: f32, pad_bottom: f32, pad_left: f32,
) {
    use rendero_core::properties::{AutoLayout, LayoutDirection, SizingMode, LayoutAlign};
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    if let Some(page) = e.document.page_mut(e.current_page) {
        if let Some(node) = page.tree.get_mut(&node_id) {
            let dir = if direction == 0 { LayoutDirection::Horizontal } else { LayoutDirection::Vertical };
            if let NodeKind::Frame { ref mut auto_layout, .. } = node.kind {
                *auto_layout = Some(AutoLayout {
                    direction: dir,
                    spacing,
                    padding_top: pad_top,
                    padding_right: pad_right,
                    padding_bottom: pad_bottom,
                    padding_left: pad_left,
                    primary_sizing: SizingMode::Hug,
                    counter_sizing: SizingMode::Hug,
                    align: LayoutAlign::Start,
                });
            }
            e.invalidate_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn rendero_select_node(engine: *mut c_void, counter: u32, client_id: u32) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    e.selected = vec![node_id];
}

#[no_mangle]
pub extern "C" fn rendero_delete_selected(engine: *mut c_void) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };
    let ids: Vec<NodeId> = e.selected.drain(..).collect();
    for node_id in ids {
        if let Some(page) = e.document.page_mut(e.current_page) {
            page.tree.remove(&node_id);
        }
    }
    e.invalidate_cache();
}

#[no_mangle]
pub extern "C" fn rendero_get_node_bounds(
    engine: *mut c_void, counter: u32, client_id: u32,
    out_x: *mut f32, out_y: *mut f32, out_w: *mut f32, out_h: *mut f32,
) {
    let e = unsafe { &*(engine as *const NativeEngine) };
    let node_id = NodeId::new(counter as u64, client_id);
    let (wx, wy) = e.node_world_pos(&node_id);
    let (w, h) = e.document.page(e.current_page)
        .and_then(|p| p.tree.get(&node_id))
        .map(|n| (n.width, n.height))
        .unwrap_or((0.0, 0.0));
    unsafe {
        *out_x = wx;
        *out_y = wy;
        *out_w = w;
        *out_h = h;
    }
}

/// Render to raw RGBA pixels. Caller provides a buffer of size width*height*4.
#[no_mangle]
pub extern "C" fn rendero_render_pixels(
    engine: *mut c_void, buffer: *mut u8, width: u32, height: u32,
) {
    let e = unsafe { &mut *(engine as *mut NativeEngine) };

    // Run layout before rendering (Taffy flexbox)
    if let Some(page) = e.document.page_mut(e.current_page) {
        let root = page.tree.root_id();
        rendero_core::layout::compute_layout(&mut page.tree, &root);
    }

    let page = match e.document.page(e.current_page) {
        Some(p) => p,
        None => return,
    };
    let root_id = page.tree.root_id();

    // Apply camera transform to viewport
    let vp = AABB::new(
        Vec2::new(e.cam_x, e.cam_y),
        Vec2::new(
            e.cam_x + width as f32 / e.cam_zoom,
            e.cam_y + height as f32 / e.cam_zoom,
        ),
    );

    let items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &vp);
    let output = pipeline::render_items(&items, vp);
    let pixels = output.to_pixels(width, height);

    // Copy to caller's buffer
    let len = (width * height * 4) as usize;
    let buf = unsafe { std::slice::from_raw_parts_mut(buffer, len) };
    let copy_len = pixels.len().min(len);
    buf[..copy_len].copy_from_slice(&pixels[..copy_len]);
}
