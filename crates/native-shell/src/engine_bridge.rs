//! Engine bridge — registers __rendero_* functions in the JS context.
//!
//! Strategy: We register ONE Rust dispatch function `__rendero_call(cmd, args_json)`
//! and define individual JS wrapper functions that call it. This avoids
//! rquickjs lifetime issues with multiple captured closures.

use std::cell::RefCell;
use std::rc::Rc;

use rquickjs::Ctx;
use serde_json::json;

use rendero_core::document::Document;
use rendero_core::id::NodeId;
use rendero_core::node::{Node, NodeKind};
use rendero_core::properties::*;
use rendero_renderer::pipeline;
use rendero_renderer::scene::AABB;
use rendero_text::ParleyTextMeasurer;
use glam::Vec2;

use crate::providers::NativeAPI;

/// The native engine.
pub struct Engine {
    pub document: Document,
    pub selected: Vec<NodeId>,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub cam_x: f32,
    pub cam_y: f32,
    pub cam_zoom: f32,
    pub insert_parent: Option<NodeId>,
    pub current_page: usize,
}

impl Engine {
    pub fn new(name: &str, client_id: u32) -> Self {
        Self {
            document: Document::new(name, client_id),
            selected: Vec::new(),
            viewport_width: 1280,
            viewport_height: 800,
            cam_x: 0.0,
            cam_y: 0.0,
            cam_zoom: 1.0,
            insert_parent: None,
            current_page: 0,
        }
    }

    fn effective_parent(&self) -> NodeId {
        self.insert_parent.unwrap_or_else(|| {
            self.document.page(self.current_page).unwrap().tree.root_id()
        })
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
                Some(parent) => cur = parent,
                None => break,
            }
        }
        (wx, wy)
    }

    pub fn compute_layout(&mut self) {
        if let Some(page) = self.document.page_mut(self.current_page) {
            let root = page.tree.root_id();
            rendero_core::layout::compute_layout_with_text_measurer(
                &mut page.tree,
                &root,
                ParleyTextMeasurer::new(),
            );
        }
    }

    pub fn export_document_json(&self) -> String {
        serde_json::to_string(&self.document.to_snapshot())
            .unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    pub fn export_engine_model_json(&self) -> String {
        let Some(page) = self.document.page(self.current_page) else {
            return "[]".to_string();
        };
        let root_id = page.tree.root_id();
        let traversal = page.tree.traverse_depth_first(&root_id);
        let entries: Vec<_> = traversal.iter().filter_map(|node_id| {
            let node = page.tree.get(node_id)?;
            let parent = page.tree.parent_of(node_id);
            let kind = match &node.kind {
                NodeKind::Frame { .. } => "Frame",
                NodeKind::Rectangle { .. } => "Rectangle",
                NodeKind::Ellipse { .. } => "Ellipse",
                NodeKind::Line => "Line",
                NodeKind::Polygon { .. } => "Polygon",
                NodeKind::Vector { .. } => "Vector",
                NodeKind::Text { .. } => "Text",
                NodeKind::BooleanOp { .. } => "BooleanOp",
                NodeKind::Component => "Component",
                NodeKind::Instance { .. } => "Instance",
                NodeKind::Image { .. } => "Image",
            };
            let text = match &node.kind {
                NodeKind::Text { runs, align, .. } => Some(json!({
                    "text": runs.iter().map(|run| run.text.as_str()).collect::<String>(),
                    "align": align,
                    "runs": runs.iter().map(|run| json!({
                        "text": run.text,
                        "fontFamily": run.font_family,
                        "fontSize": run.font_size,
                        "fontWeight": run.font_weight,
                        "lineHeight": run.line_height,
                        "letterSpacing": run.letter_spacing,
                    })).collect::<Vec<_>>(),
                })),
                _ => None,
            };
            Some(json!({
                "key": format!("{}:{}", node.id.0.counter, node.id.0.client_id),
                "parentKey": parent.map(|p| format!("{}:{}", p.0.counter, p.0.client_id)),
                "name": node.name,
                "kind": kind,
                "width": node.width,
                "height": node.height,
                "transform": {
                    "tx": node.transform.tx,
                    "ty": node.transform.ty,
                },
                "sizing": {
                    "horizontal": node.horizontal_sizing,
                    "vertical": node.vertical_sizing,
                },
                "margin": node.margin,
                "layoutPosition": node.layout_position,
                "sizeConstraints": node.size_constraints,
                "autoLayout": match &node.kind {
                    NodeKind::Frame { auto_layout, .. } => auto_layout,
                    _ => &None,
                },
                "fills": node.style.fills,
                "opacity": node.style.opacity,
                "text": text,
            }))
        }).collect();
        serde_json::to_string(&entries).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    pub fn export_layout_json(&self) -> String {
        let Some(page) = self.document.page(self.current_page) else {
            return "[]".to_string();
        };
        let root_id = page.tree.root_id();
        let traversal = page.tree.traverse_depth_first(&root_id);
        let entries: Vec<_> = traversal.iter().filter_map(|node_id| {
            let node = page.tree.get(node_id)?;
            let parent = page.tree.parent_of(node_id);
            let (wx, wy) = self.node_world_pos(node_id);
            let kind = match &node.kind {
                NodeKind::Frame { .. } => "Frame",
                NodeKind::Rectangle { .. } => "Rectangle",
                NodeKind::Ellipse { .. } => "Ellipse",
                NodeKind::Line => "Line",
                NodeKind::Polygon { .. } => "Polygon",
                NodeKind::Vector { .. } => "Vector",
                NodeKind::Text { .. } => "Text",
                NodeKind::BooleanOp { .. } => "BooleanOp",
                NodeKind::Component => "Component",
                NodeKind::Instance { .. } => "Instance",
                NodeKind::Image { .. } => "Image",
            };
            Some(json!({
                "key": format!("{}:{}", node.id.0.counter, node.id.0.client_id),
                "parentKey": parent.map(|p| format!("{}:{}", p.0.counter, p.0.client_id)),
                "name": node.name,
                "kind": kind,
                "local": {
                    "x": node.transform.tx,
                    "y": node.transform.ty,
                },
                "world": {
                    "x": wx,
                    "y": wy,
                },
                "size": {
                    "width": node.width,
                    "height": node.height,
                },
                "bounds": {
                    "x": wx,
                    "y": wy,
                    "width": node.width,
                    "height": node.height,
                },
            }))
        }).collect();
        serde_json::to_string(&entries).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    pub fn render_pixels(&mut self, width: u32, height: u32) -> Vec<u8> {
        self.compute_layout();

        let page = match self.document.page(self.current_page) {
            Some(p) => p,
            None => return vec![0u8; (width * height * 4) as usize],
        };
        let root_id = page.tree.root_id();
        let vp = AABB::new(
            Vec2::new(self.cam_x, self.cam_y),
            Vec2::new(
                self.cam_x + width as f32 / self.cam_zoom,
                self.cam_y + height as f32 / self.cam_zoom,
            ),
        );
        let items = rendero_renderer::scene::build_scene(&page.tree, &root_id, &vp);
        if !items.is_empty() {
            let traversal = page.tree.traverse_depth_first(&root_id);
            println!(
                "[Engine] tree nodes={} scene items={} viewport=({:.0},{:.0})→({:.0},{:.0})",
                traversal.len(),
                items.len(),
                vp.min.x,
                vp.min.y,
                vp.max.x,
                vp.max.y
            );
            for (i, node_id) in traversal.iter().take(16).enumerate() {
                if let Some(node) = page.tree.get(node_id) {
                    let parent = page.tree.parent_of(node_id);
                    println!(
                        "[Engine] tree[{i}] node={:?} parent={:?} name={} pos=({:.0},{:.0}) size=({:.0},{:.0})",
                        node_id,
                        parent,
                        node.name,
                        node.transform.tx,
                        node.transform.ty,
                        node.width,
                        node.height
                    );
                }
            }
            for (i, item) in items.iter().take(12).enumerate() {
                println!(
                    "[Engine] item[{i}] node={:?} bounds=({:.0},{:.0})→({:.0},{:.0}) fills={} opacity={:.2} shape={}",
                    item.node_id,
                    item.world_bounds.min.x,
                    item.world_bounds.min.y,
                    item.world_bounds.max.x,
                    item.world_bounds.max.y,
                    item.style.fills.len(),
                    item.style.opacity,
                    match &item.shape {
                        rendero_renderer::scene::RenderShape::Rect { .. } => "Rect",
                        rendero_renderer::scene::RenderShape::Text { .. } => "Text",
                        rendero_renderer::scene::RenderShape::Image { .. } => "Image",
                        rendero_renderer::scene::RenderShape::Ellipse { .. } => "Ellipse",
                        rendero_renderer::scene::RenderShape::Line { .. } => "Line",
                        rendero_renderer::scene::RenderShape::Path { .. } => "Path",
                    }
                );
            }
        } else {
            println!(
                "[Engine] scene items=0 viewport=({:.0},{:.0})→({:.0},{:.0})",
                vp.min.x, vp.min.y, vp.max.x, vp.max.y
            );
        }
        let output = pipeline::render_items(&items, vp);
        output.to_pixels(width, height)
    }

    /// Dispatch a command from JS. Returns a f64 (packed ID for create ops, 0 for others).
    pub fn dispatch(&mut self, cmd: &str, args: &[f64]) -> f64 {
        match cmd {
            "set_viewport" => {
                self.viewport_width = args[0] as u32;
                self.viewport_height = args[1] as u32;
                0.0
            }
            "set_camera" => {
                self.cam_x = args[0] as f32;
                self.cam_y = args[1] as f32;
                self.cam_zoom = args[2] as f32;
                0.0
            }
            "get_camera_x" => self.cam_x as f64,
            "get_camera_y" => self.cam_y as f64,
            "get_camera_zoom" => self.cam_zoom as f64,
            "set_insert_parent" => {
                self.insert_parent = Some(NodeId::new(args[0] as u64, args[1] as u32));
                0.0
            }
            "clear_insert_parent" => {
                self.insert_parent = None;
                0.0
            }
            "add_frame" => {
                // args: name_len, ...name_chars, x, y, w, h, r, g, b, a
                // Simplified: just use numeric args, name generated
                let (x, y, w, h) = (args[0] as f32, args[1] as f32, args[2] as f32, args[3] as f32);
                let (r, g, b, a) = (args[4] as f32, args[5] as f32, args[6] as f32, args[7] as f32);
                let id = self.document.next_id();
                let mut node = Node::frame(id, &format!("f_{}", id.0.counter), w, h);
                node.transform = Transform::translate(x, y);
                if a > 0.0 {
                    node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));
                }
                let parent_id = self.effective_parent();
                self.document.add_node(self.current_page, node, parent_id, usize::MAX).ok();
                ((id.0.counter << 32) | (id.0.client_id as u64)) as f64
            }
            "add_text" => {
                // args: x, y, fontSize, r, g, b, a (text passed separately)
                let (x, y) = (args[0] as f32, args[1] as f32);
                let font_size = args[2] as f32;
                let (r, g, b, a) = (args[3] as f32, args[4] as f32, args[5] as f32, args[6] as f32);
                let id = self.document.next_id();
                let color = Color::new(r, g, b, a);
                // Text content is passed as arg[7..] encoded, but we'll use a simpler approach
                let mut node = Node::text(id, &format!("t_{}", id.0.counter), " ", font_size, color);
                node.transform = Transform::translate(x, y);
                let parent_id = self.effective_parent();
                self.document.add_node(self.current_page, node, parent_id, usize::MAX).ok();
                ((id.0.counter << 32) | (id.0.client_id as u64)) as f64
            }
            "set_node_position" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.transform.tx = args[2] as f32;
                        node.transform.ty = args[3] as f32;
                    }
                }
                0.0
            }
            "set_node_layout_position" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.layout_position = Some(LayoutPosition {
                            x: args[2] as f32,
                            y: args[3] as f32,
                        });
                    }
                }
                0.0
            }
            "set_node_clip_content" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Frame { clip_content, .. } = &mut node.kind {
                            *clip_content = args.get(2).copied().unwrap_or(0.0) != 0.0;
                        }
                    }
                }
                0.0
            }
            "set_node_size" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.width = args[2] as f32;
                        node.height = args[3] as f32;
                        if matches!(node.kind, NodeKind::Text { .. }) {
                            node.text_size_locked = true;
                        }
                        if node.width > 0.0 {
                            node.width_percent = None;
                        }
                        if node.height > 0.0 {
                            node.height_percent = None;
                        }
                    }
                }
                0.0
            }
            "set_node_size_constraints" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.size_constraints.min_width = (args[2] as f32).max(0.0);
                        node.size_constraints.min_height = (args[3] as f32).max(0.0);
                        node.size_constraints.max_width = (args[4] as f32).max(0.0);
                        node.size_constraints.max_height = (args[5] as f32).max(0.0);
                    }
                }
                0.0
            }
            "set_node_sizing" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        let decode = |value: u32| match value {
                            1 => SizingMode::Hug,
                            2 => SizingMode::Fill,
                            _ => SizingMode::Fixed,
                        };
                        node.horizontal_sizing = decode(args[2] as u32);
                        node.vertical_sizing = decode(args[3] as u32);
                    }
                }
                0.0
            }
            "set_node_margin" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.margin = LayoutMargin {
                            top: args[2] as f32,
                            right: args[3] as f32,
                            bottom: args[4] as f32,
                            left: args[5] as f32,
                            auto_top: args.get(6).copied().unwrap_or_default() != 0.0,
                            auto_right: args.get(7).copied().unwrap_or_default() != 0.0,
                            auto_bottom: args.get(8).copied().unwrap_or_default() != 0.0,
                            auto_left: args.get(9).copied().unwrap_or_default() != 0.0,
                        };
                    }
                }
                0.0
            }
            "set_node_size_percent" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.width_percent = if args[2] > 0.0 { Some(args[2] as f32) } else { None };
                        node.height_percent = if args[3] > 0.0 { Some(args[3] as f32) } else { None };
                    }
                }
                0.0
            }
            "set_node_fill" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        let color = Color::new(args[2] as f32, args[3] as f32, args[4] as f32, args[5] as f32);
                        if let NodeKind::Text { ref mut runs, .. } = node.kind {
                            for run in runs.iter_mut() { run.color = color; }
                        }
                        node.style.fills = vec![Paint::Solid(color)];
                    }
                }
                0.0
            }
            "set_node_corner_radius" => {
                use rendero_core::node::CornerRadii;
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        let (tl, tr, br, bl) = (args[2] as f32, args[3] as f32, args[4] as f32, args[5] as f32);
                        let radii = if tl == tr && tr == br && br == bl {
                            CornerRadii::Uniform(tl)
                        } else {
                            CornerRadii::PerCorner { top_left: tl, top_right: tr, bottom_right: br, bottom_left: bl }
                        };
                        if let NodeKind::Frame { ref mut corner_radii, .. } = node.kind {
                            *corner_radii = radii;
                        }
                    }
                }
                0.0
            }
            "set_node_opacity" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.style.opacity = (args[2] as f32).clamp(0.0, 1.0);
                    }
                }
                0.0
            }
            "set_node_font_size" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Text { ref mut runs, .. } = node.kind {
                            let sz = args[2] as f32;
                            node.text_size_locked = false;
                            for run in runs.iter_mut() { run.font_size = sz; }
                            // Don't set width/height here — let Taffy's text measurer
                            // or JS browser measurement handle sizing.
                        }
                    }
                }
                0.0
            }
            "set_node_font_weight" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Text { ref mut runs, .. } = node.kind {
                            node.text_size_locked = false;
                            for run in runs.iter_mut() { run.font_weight = args[2] as u16; }
                        }
                    }
                }
                0.0
            }
            "set_node_stroke" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.style.strokes = vec![Paint::Solid(Color::new(
                            args[2] as f32,
                            args[3] as f32,
                            args[4] as f32,
                            args[5] as f32,
                        ))];
                        node.style.stroke_weight = args[6] as f32;
                    }
                }
                0.0
            }
            "set_node_rotation" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        let radians = (args[2] as f32).to_radians();
                        let (s, c) = radians.sin_cos();
                        let sx = (node.transform.a.powi(2) + node.transform.b.powi(2)).sqrt().max(1.0);
                        let sy = (node.transform.c.powi(2) + node.transform.d.powi(2)).sqrt().max(1.0);
                        node.transform.a = c * sx;
                        node.transform.b = s * sx;
                        node.transform.c = -s * sy;
                        node.transform.d = c * sy;
                    }
                }
                0.0
            }
            "add_drop_shadow" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.style.effects.push(Effect::DropShadow {
                            color: Color::new(args[2] as f32, args[3] as f32, args[4] as f32, args[5] as f32),
                            offset: Vec2::new(args[6] as f32, args[7] as f32),
                            blur_radius: args[8] as f32,
                            spread: args[9] as f32,
                        });
                    }
                }
                0.0
            }
            "set_node_linear_gradient" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                let stop_count = args.get(6).copied().unwrap_or(0.0).max(0.0) as usize;
                let positions_start = 7;
                let colors_start = positions_start + stop_count;
                if args.len() < colors_start + stop_count * 4 {
                    return 0.0;
                }
                let mut stops = Vec::with_capacity(stop_count);
                for idx in 0..stop_count {
                    let pos = args[positions_start + idx] as f32;
                    let color_base = colors_start + idx * 4;
                    let color = Color::new(
                        args[color_base] as f32,
                        args[color_base + 1] as f32,
                        args[color_base + 2] as f32,
                        args[color_base + 3] as f32,
                    );
                    stops.push(GradientStop::new(pos, color));
                }
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        node.style.fills = vec![Paint::LinearGradient {
                            stops,
                            start: Vec2::new(args[2] as f32, args[3] as f32),
                            end: Vec2::new(args[4] as f32, args[5] as f32),
                        }];
                    }
                }
                0.0
            }
            "set_auto_layout" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        let dir = if args[2] as u32 == 0 { LayoutDirection::Horizontal } else { LayoutDirection::Vertical };
                        let align = match args.get(8).copied().unwrap_or(0.0) as u32 {
                            1 => LayoutAlign::Center,
                            2 => LayoutAlign::End,
                            3 => LayoutAlign::Stretch,
                            _ => LayoutAlign::Start,
                        };
                        let justify = match args.get(9).copied().unwrap_or(0.0) as u32 {
                            1 => LayoutJustify::Center,
                            2 => LayoutJustify::End,
                            3 => LayoutJustify::SpaceBetween,
                            4 => LayoutJustify::SpaceAround,
                            5 => LayoutJustify::SpaceEvenly,
                            _ => LayoutJustify::Start,
                        };
                        let wrap = match args.get(10).copied().unwrap_or(0.0) as u32 {
                            1 => LayoutWrap::Wrap,
                            _ => LayoutWrap::NoWrap,
                        };
                        let resolve_sizing = |mode: SizingMode, extent: f32| match mode {
                            SizingMode::Fixed if extent <= 0.0 => SizingMode::Hug,
                            _ => mode,
                        };
                        let (primary_sizing, counter_sizing) = match dir {
                            LayoutDirection::Horizontal => (
                                resolve_sizing(node.horizontal_sizing, node.width),
                                resolve_sizing(node.vertical_sizing, node.height),
                            ),
                            LayoutDirection::Vertical => (
                                resolve_sizing(node.vertical_sizing, node.height),
                                resolve_sizing(node.horizontal_sizing, node.width),
                            ),
                        };
                        if let NodeKind::Frame { ref mut auto_layout, .. } = node.kind {
                            *auto_layout = Some(AutoLayout {
                                direction: dir,
                                spacing: args[3] as f32,
                                padding_top: args[4] as f32,
                                padding_right: args[5] as f32,
                                padding_bottom: args[6] as f32,
                                padding_left: args[7] as f32,
                                primary_sizing,
                                counter_sizing,
                                align,
                                justify,
                                wrap,
                            });
                        }
                    }
                }
                0.0
            }
            "select_node" => {
                self.selected.clear();
                self.selected.push(NodeId::new(args[0] as u64, args[1] as u32));
                0.0
            }
            "delete_selected" => {
                let selected: Vec<_> = self.selected.drain(..).collect();
                if let Some(page) = self.document.page_mut(self.current_page) {
                    for node_id in selected {
                        let _ = page.tree.remove(&node_id);
                    }
                }
                0.0
            }
            "get_node_bounds_x" => self.node_bounds_component(args, 0) as f64,
            "get_node_bounds_y" => self.node_bounds_component(args, 1) as f64,
            "get_node_bounds_w" => self.node_bounds_component(args, 2) as f64,
            "get_node_bounds_h" => self.node_bounds_component(args, 3) as f64,
            "request_render" => {
                0.0
            }
            _ => 0.0,
        }
    }

    /// Dispatch a command that includes a string arg (for text/name operations).
    pub fn dispatch_str(&mut self, cmd: &str, args: &[f64], text: &str) -> f64 {
        match cmd {
            "add_frame_named" => {
                let (x, y, w, h) = (args[0] as f32, args[1] as f32, args[2] as f32, args[3] as f32);
                let (r, g, b, a) = (args[4] as f32, args[5] as f32, args[6] as f32, args[7] as f32);
                let id = self.document.next_id();
                let mut node = Node::frame(id, text, w, h);
                node.transform = Transform::translate(x, y);
                if a > 0.0 {
                    node.style.fills.push(Paint::Solid(Color::new(r, g, b, a)));
                }
                let parent_id = self.effective_parent();
                self.document.add_node(self.current_page, node, parent_id, usize::MAX).ok();
                ((id.0.counter << 32) | (id.0.client_id as u64)) as f64
            }
            "add_text_named" => {
                let (x, y) = (args[0] as f32, args[1] as f32);
                let font_size = args[2] as f32;
                let (r, g, b, a) = (args[3] as f32, args[4] as f32, args[5] as f32, args[6] as f32);
                let name = &text[..text.find('\0').unwrap_or(text.len())];
                let content = &text[text.find('\0').map(|i| i + 1).unwrap_or(text.len())..];
                let id = self.document.next_id();
                let color = Color::new(r, g, b, a);
                let content = if content.is_empty() { " " } else { content };
                let mut node = Node::text(id, name, content, font_size, color);
                node.transform = Transform::translate(x, y);
                let parent_id = self.effective_parent();
                self.document.add_node(self.current_page, node, parent_id, usize::MAX).ok();
                ((id.0.counter << 32) | (id.0.client_id as u64)) as f64
            }
            "set_node_text" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Text { ref mut runs, .. } = node.kind {
                            if let Some(run) = runs.first_mut() {
                                node.text_size_locked = false;
                                run.text = text.to_string();
                                // Don't set width/height — let layout engine measure.
                            }
                        }
                    }
                }
                0.0
            }
            "set_node_font_family" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Text { ref mut runs, .. } = node.kind {
                            node.text_size_locked = false;
                            for run in runs.iter_mut() {
                                run.font_family = text.to_string();
                            }
                        }
                    }
                }
                0.0
            }
            "set_node_letter_spacing" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Text { ref mut runs, .. } = node.kind {
                            node.text_size_locked = false;
                            for run in runs.iter_mut() {
                                run.letter_spacing = args[2] as f32;
                            }
                        }
                    }
                }
                0.0
            }
            "set_node_line_height" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Text { ref mut runs, .. } = node.kind {
                            let height = args[2] as f32;
                            node.text_size_locked = false;
                            for run in runs.iter_mut() {
                                run.line_height = if height > 0.0 { Some(height) } else { None };
                            }
                        }
                    }
                }
                0.0
            }
            "set_text_align" => {
                let node_id = NodeId::new(args[0] as u64, args[1] as u32);
                if let Some(page) = self.document.page_mut(self.current_page) {
                    if let Some(node) = page.tree.get_mut(&node_id) {
                        if let NodeKind::Text { ref mut align, .. } = node.kind {
                            *align = match text {
                                "center" => rendero_core::node::TextAlign::Center,
                                "right" => rendero_core::node::TextAlign::Right,
                                "justify" | "justified" => rendero_core::node::TextAlign::Justified,
                                _ => rendero_core::node::TextAlign::Left,
                            };
                        }
                    }
                }
                0.0
            }
            _ => 0.0,
        }
    }
}

impl Engine {
    fn node_bounds_component(&self, args: &[f64], idx: usize) -> f32 {
        let node_id = NodeId::new(args[0] as u64, args[1] as u32);
        let Some(page) = self.document.page(self.current_page) else {
            return 0.0;
        };
        let Some(node) = page.tree.get(&node_id) else {
            return 0.0;
        };

        let (x, y) = self.node_world_pos(&node_id);
        match idx {
            0 => x,
            1 => y,
            2 => node.width,
            3 => node.height,
            _ => 0.0,
        }
    }
}

/// Register the dispatch function + JS wrappers in the QuickJS context.
pub fn register_engine_functions(ctx: &Ctx<'_>, engine: &Rc<RefCell<Engine>>, native_api: &dyn NativeAPI) {
    let globals = ctx.globals();

    // console.log/warn/error
    ctx.eval::<(), _>(r#"
        var console = {
            log: function() { __rendero_log(Array.prototype.join.call(arguments, ' ')); },
            warn: function() { __rendero_log('WARN: ' + Array.prototype.join.call(arguments, ' ')); },
            error: function() { __rendero_log('ERROR: ' + Array.prototype.join.call(arguments, ' ')); }
        };
    "#).ok();

    // Register the two dispatch functions
    let eng = engine.clone();
    globals.set("__rendero_dispatch", rquickjs::Function::new(ctx.clone(), rquickjs::function::MutFn::from(move |cmd: String, args: Vec<f64>| -> f64 {
        eng.borrow_mut().dispatch(&cmd, &args)
    })).unwrap()).unwrap();

    let eng = engine.clone();
    globals.set("__rendero_dispatch_str", rquickjs::Function::new(ctx.clone(), rquickjs::function::MutFn::from(move |cmd: String, args: Vec<f64>, text: String| -> f64 {
        eng.borrow_mut().dispatch_str(&cmd, &args, &text)
    })).unwrap()).unwrap();

    // __rendero_log
    globals.set("__rendero_log", rquickjs::Function::new(ctx.clone(), |msg: String| {
        println!("[JS] {msg}");
    }).unwrap()).unwrap();

    let storage_get = native_api.storage_get("__bootstrap").unwrap_or_default();
    let clipboard_text = native_api.clipboard_read_text().unwrap_or_default();
    let notifications_enabled = native_api.notifications_request_permission();

    // performance.now
    ctx.eval::<(), _>(r#"
        var performance = { now: function() { return Date.now(); } };
    "#).ok();

    // Internal bridge object. Legacy __rendero_* globals are thin wrappers only.
    ctx.eval::<(), _>(r#"
        globalThis.__RenderoHostBridge = {
            dispatch: function(cmd, args) { return __rendero_dispatch(cmd, args || []); },
            dispatchStr: function(cmd, args, text) { return __rendero_dispatch_str(cmd, args || [], text || ''); },
            log: function(msg) { __rendero_log(String(msg)); },
            screen: {
                width: typeof __screenWidth !== 'undefined' ? __screenWidth : 1280,
                height: typeof __screenHeight !== 'undefined' ? __screenHeight : 800,
                scale: typeof __screenScale !== 'undefined' ? __screenScale : 2
            }
        };

        function __rendero_set_viewport(w, h) { return __RenderoHostBridge.dispatch('set_viewport', [w, h]); }
        function __rendero_set_camera(x, y, z) { return __RenderoHostBridge.dispatch('set_camera', [x, y, z]); }
        function __rendero_get_camera() {
            return {
                x: __RenderoHostBridge.dispatch('get_camera_x', []),
                y: __RenderoHostBridge.dispatch('get_camera_y', []),
                zoom: __RenderoHostBridge.dispatch('get_camera_zoom', [])
            };
        }
        function __rendero_set_insert_parent(c, ci) { return __RenderoHostBridge.dispatch('set_insert_parent', [c, ci]); }
        function __rendero_clear_insert_parent() { return __RenderoHostBridge.dispatch('clear_insert_parent', []); }
        function __rendero_add_frame(name, x, y, w, h, r, g, b, a) {
            return __RenderoHostBridge.dispatchStr('add_frame_named', [x, y, w, h, r, g, b, a], name);
        }
        function __rendero_add_text(name, text, x, y, fs, r, g, b, a) {
            return __RenderoHostBridge.dispatchStr('add_text_named', [x, y, fs, r, g, b, a], name + '\0' + text);
        }
        function __rendero_set_node_position(c, ci, x, y) { return __RenderoHostBridge.dispatch('set_node_position', [c, ci, x, y]); }
        function __rendero_set_node_layout_position(c, ci, x, y) { return __RenderoHostBridge.dispatch('set_node_layout_position', [c, ci, x, y]); }
        function __rendero_set_node_clip_content(c, ci, clip) { return __RenderoHostBridge.dispatch('set_node_clip_content', [c, ci, clip ? 1 : 0]); }
        function __rendero_set_node_size(c, ci, w, h) { return __RenderoHostBridge.dispatch('set_node_size', [c, ci, w, h]); }
        function __rendero_set_node_size_percent(c, ci, w, h) { return __RenderoHostBridge.dispatch('set_node_size_percent', [c, ci, w, h]); }
        function __rendero_set_node_size_constraints(c, ci, minW, minH, maxW, maxH) { return __RenderoHostBridge.dispatch('set_node_size_constraints', [c, ci, minW, minH, maxW, maxH]); }
        function __rendero_set_node_sizing(c, ci, h, v) { return __RenderoHostBridge.dispatch('set_node_sizing', [c, ci, h, v]); }
        function __rendero_set_node_margin(c, ci, t, r, b, l, at, ar, ab, al) { return __RenderoHostBridge.dispatch('set_node_margin', [c, ci, t, r, b, l, at ? 1 : 0, ar ? 1 : 0, ab ? 1 : 0, al ? 1 : 0]); }
        function __rendero_set_node_fill(c, ci, r, g, b, a) { return __RenderoHostBridge.dispatch('set_node_fill', [c, ci, r, g, b, a]); }
        function __rendero_set_node_corner_radius(c, ci, tl, tr, br, bl) { return __RenderoHostBridge.dispatch('set_node_corner_radius', [c, ci, tl, tr, br, bl]); }
        function __rendero_set_node_opacity(c, ci, o) { return __RenderoHostBridge.dispatch('set_node_opacity', [c, ci, o]); }
        function __rendero_set_node_text(c, ci, text) { return __RenderoHostBridge.dispatchStr('set_node_text', [c, ci], text); }
        function __rendero_set_node_font_size(c, ci, s) { return __RenderoHostBridge.dispatch('set_node_font_size', [c, ci, s]); }
        function __rendero_set_node_font_weight(c, ci, w) { return __RenderoHostBridge.dispatch('set_node_font_weight', [c, ci, w]); }
        function __rendero_set_node_font_family(c, ci, family) { return __RenderoHostBridge.dispatchStr('set_node_font_family', [c, ci], family); }
        function __rendero_set_node_letter_spacing(c, ci, spacing) { return __RenderoHostBridge.dispatch('set_node_letter_spacing', [c, ci, spacing]); }
        function __rendero_set_node_line_height(c, ci, height) { return __RenderoHostBridge.dispatch('set_node_line_height', [c, ci, height]); }
        function __rendero_set_text_align(c, ci, align) { return __RenderoHostBridge.dispatchStr('set_text_align', [c, ci], align); }
        function __rendero_set_node_stroke(c, ci, r, g, b, a, w) { return __RenderoHostBridge.dispatch('set_node_stroke', [c, ci, r, g, b, a, w]); }
        function __rendero_set_node_rotation(c, ci, degrees) { return __RenderoHostBridge.dispatch('set_node_rotation', [c, ci, degrees]); }
        function __rendero_add_drop_shadow(c, ci, r, g, b, a, ox, oy, blur, spread) { return __RenderoHostBridge.dispatch('add_drop_shadow', [c, ci, r, g, b, a, ox, oy, blur, spread]); }
        function __rendero_set_node_linear_gradient(c, ci, startX, startY, endX, endY, stopCount) {
            const rest = Array.prototype.slice.call(arguments, 7);
            return __RenderoHostBridge.dispatch('set_node_linear_gradient', [c, ci, startX, startY, endX, endY, stopCount].concat(rest));
        }
        function __rendero_set_auto_layout(c, ci, dir, sp, pt, pr, pb, pl, align, justify, wrap) { return __RenderoHostBridge.dispatch('set_auto_layout', [c, ci, dir, sp, pt, pr, pb, pl, align, justify, wrap]); }
        function __rendero_select_node(c, ci) { return __RenderoHostBridge.dispatch('select_node', [c, ci]); }
        function __rendero_delete_selected() { return __RenderoHostBridge.dispatch('delete_selected', []); }
        function __rendero_get_node_bounds(c, ci) {
            return {
                x: __RenderoHostBridge.dispatch('get_node_bounds_x', [c, ci]),
                y: __RenderoHostBridge.dispatch('get_node_bounds_y', [c, ci]),
                w: __RenderoHostBridge.dispatch('get_node_bounds_w', [c, ci]),
                h: __RenderoHostBridge.dispatch('get_node_bounds_h', [c, ci])
            };
        }
        function __rendero_request_render() { return __RenderoHostBridge.dispatch('request_render', []); }

        globalThis.Rendero = globalThis.Rendero || {};
        globalThis.Rendero.__bridge = __RenderoHostBridge;
    "#).ok();

    ctx.eval::<(), _>(format!(r#"
        globalThis.Rendero = Object.assign(globalThis.Rendero || {{}}, {{
            native: {{
                storage: {{
                    get: function(key) {{
                        if (key === '__bootstrap') return {storage_get:?};
                        return null;
                    }},
                    set: function(_key, _value) {{ return true; }}
                }},
                clipboard: {{
                    readText: function() {{ return {clipboard_text:?}; }},
                    writeText: function(_text) {{ return true; }}
                }},
                dialogs: {{
                    alert: function(message) {{ __RenderoHostBridge.log('ALERT: ' + message); return true; }}
                }},
                notifications: {{
                    requestPermission: function() {{ return {notifications_enabled}; }}
                }},
                haptics: {{
                    impact: function(_style) {{ return true; }}
                }},
                media: {{
                    pickImage: function() {{ return null; }}
                }}
            }}
        }});
    "#)).ok();

    // Screen info
    {
        let e = engine.borrow();
        ctx.eval::<(), _>(format!(
            "var __screenWidth = {}; var __screenHeight = {}; var __screenScale = 2;",
            e.viewport_width, e.viewport_height
        )).ok();
    }
}

/// Browser polyfills — minimal globals that React expects.
pub const BROWSER_POLYFILLS: &str = r#"
var window = globalThis;
var self = globalThis;
var global = globalThis;

var navigator = { userAgent: 'RenderoNative/1.0', platform: 'macOS' };

var Element = function Element() {};
Element.prototype = {};
var HTMLElement = function HTMLElement() {};
HTMLElement.prototype = Object.create(Element.prototype);
var SVGElement = function SVGElement() {};
SVGElement.prototype = Object.create(HTMLElement.prototype);
var MathMLElement = function MathMLElement() {};
MathMLElement.prototype = Object.create(HTMLElement.prototype);
var HTMLIFrameElement = function HTMLIFrameElement() {};
HTMLIFrameElement.prototype = Object.create(HTMLElement.prototype);
var HTMLInputElement = function HTMLInputElement() {};
HTMLInputElement.prototype = Object.create(HTMLElement.prototype);
var HTMLTextAreaElement = function HTMLTextAreaElement() {};
var HTMLSelectElement = function HTMLSelectElement() {};
var Node = function Node() {};
var Event = function Event(type) { this.type = type; this.preventDefault = function(){}; this.stopPropagation = function(){}; };
var CustomEvent = Event;
var MutationObserver = function(cb) { this.observe = function() {}; this.disconnect = function() {}; };

var requestAnimationFrame = function(cb) { setTimeout(cb, 16); return 0; };
var cancelAnimationFrame = function() {};

var document = {
    createElement: function(tag) {
        return {
            tagName: tag.toUpperCase(), style: {}, childNodes: [],
            setAttribute: function() {}, getAttribute: function() { return null; },
            removeAttribute: function() {}, addEventListener: function() {},
            removeEventListener: function() {},
            appendChild: function(c) { this.childNodes.push(c); return c; },
            removeChild: function() {}, insertBefore: function() {},
            contains: function() { return false; },
            cloneNode: function() { return document.createElement(tag); },
            nodeType: 1, parentNode: null, firstChild: null, lastChild: null,
            nextSibling: null, ownerDocument: null
        };
    },
    createTextNode: function(t) { return { nodeType: 3, textContent: t, parentNode: null }; },
    createComment: function() { return { nodeType: 8 }; },
    createDocumentFragment: function() { return { nodeType: 11, childNodes: [], appendChild: function(c) { this.childNodes.push(c); return c; } }; },
    createEvent: function() { return { initEvent: function() {} }; },
    body: null, documentElement: null, activeElement: null, defaultView: null,
    addEventListener: function() {}, removeEventListener: function() {},
    getElementById: function() { return null; }, querySelector: function() { return null; },
    querySelectorAll: function() { return []; },
    implementation: { hasFeature: function() { return true; } }
};
document.body = document.createElement('body');
document.documentElement = document.createElement('html');
document.documentElement.appendChild(document.body);
document.defaultView = window;
document.activeElement = document.body;

function MessageChannel() {
    this.port1 = { onmessage: null };
    var self = this;
    this.port2 = { postMessage: function() {
        if (self.port1 && self.port1.onmessage) {
            var cb = self.port1.onmessage;
            setTimeout(function() { cb({ data: undefined }); }, 0);
        }
    }};
}
"#;
