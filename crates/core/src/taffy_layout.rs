//! Taffy-powered layout engine — full CSS flexbox.
//!
//! Implements `LayoutEngine` trait. Swappable with `LegacyLayout`.
//!
//! Flow:
//!   DocumentTree → build parallel Taffy tree → compute → apply results back
//!
//! The Taffy tree is rebuilt each time. For incremental layout, cache the
//! Taffy tree and only update changed nodes (future optimization).

use std::collections::HashMap;

use taffy::prelude::*;
use taffy::TaffyTree;

use crate::id::NodeId;
use crate::node::{NodeKind, TextRun};
use crate::properties::*;
use crate::providers::{LayoutEngine, TextMeasurer};
use crate::tree::DocumentTree;

/// Layout engine powered by Taffy (CSS flexbox + grid).
pub struct TaffyLayout<M: TextMeasurer> {
    measurer: M,
}

#[derive(Clone)]
struct TextMeasureContext {
    runs: Vec<TextRun>,
}

impl<M: TextMeasurer> TaffyLayout<M> {
    pub fn new(measurer: M) -> Self {
        Self { measurer }
    }
}

impl<M: TextMeasurer> LayoutEngine for TaffyLayout<M> {
    fn compute(&mut self, tree: &mut DocumentTree, root: &NodeId, viewport: (f32, f32)) {
        let mut taffy: TaffyTree<Option<TextMeasureContext>> = TaffyTree::new();
        let mut id_map: HashMap<NodeId, taffy::NodeId> = HashMap::new();

        // Build Taffy tree bottom-up (children before parents)
        let traversal = tree.traverse_depth_first(root);

        // Iterate in reverse so children are created before parents
        for node_id in traversal.iter().rev() {
            let rendero_node = match tree.get(node_id) {
                Some(n) => n,
                None => continue,
            };

            let parent_node = tree.parent_of(node_id).and_then(|pid| tree.get(&pid));
            let mut taffy_style = node_to_taffy_style(rendero_node, parent_node);

            let text_context = match &rendero_node.kind {
                NodeKind::Text { runs, .. } if rendero_node.width > 0.0 && rendero_node.height > 0.0 => {
                    taffy_style.size.width = Dimension::length(rendero_node.width);
                    taffy_style.size.height = Dimension::length(rendero_node.height);
                    None
                }
                NodeKind::Text { runs, .. } => Some(TextMeasureContext { runs: runs.clone() }),
                _ => None,
            };

            // Collect child Taffy IDs
            let child_ids: Vec<taffy::NodeId> = tree.children_of(node_id)
                .map(|children| {
                    children.iter()
                        .filter_map(|cid| id_map.get(cid).copied())
                        .collect()
                })
                .unwrap_or_default();

            let taffy_node = if child_ids.is_empty() {
                match text_context {
                    Some(context) => taffy.new_leaf_with_context(taffy_style, Some(context)).unwrap(),
                    None => taffy.new_leaf(taffy_style).unwrap(),
                }
            } else {
                taffy.new_with_children(taffy_style, &child_ids).unwrap()
            };

            id_map.insert(*node_id, taffy_node);
        }

        // Compute layout
        let Some(&taffy_root) = id_map.get(root) else { return };

        let root_node = tree.get(root);
        let vp_w = if viewport.0 > 0.0 { viewport.0 }
            else { root_node.map(|n| n.width).filter(|w| *w > 0.0).unwrap_or(1280.0) };
        let _vp_h = if viewport.1 > 0.0 { viewport.1 }
            else { root_node.map(|n| n.height).filter(|h| *h > 0.0).unwrap_or(800.0) };

        if taffy.compute_layout_with_measure(
            taffy_root,
            Size { width: AvailableSpace::Definite(vp_w), height: AvailableSpace::MaxContent },
            |known_dimensions, available_space, _node_id, node_context, _style| {
                let Some(context) = node_context.and_then(|ctx| ctx.as_ref()) else {
                    return Size::ZERO;
                };

                let max_width = known_dimensions.width
                    .or(match available_space.width {
                        AvailableSpace::Definite(width) => Some(width),
                        AvailableSpace::MinContent => Some(0.0),
                        AvailableSpace::MaxContent => None,
                    })
                    .unwrap_or(f32::INFINITY);

                let (mut width, mut height) = self.measurer.measure(&context.runs, max_width);
                if let Some(known_width) = known_dimensions.width {
                    width = known_width;
                }
                if let Some(known_height) = known_dimensions.height {
                    height = known_height;
                }
                Size { width, height }
            }
        ).is_err() {
            return;
        }

        // Apply results back to DocumentTree.
        for node_id in &traversal {
            let Some(&taffy_node) = id_map.get(node_id) else { continue };
            let Ok(layout) = taffy.layout(taffy_node) else { continue };

            if let Some(node) = tree.get_mut(node_id) {
                if node_id != root {
                    node.transform.tx = layout.location.x;
                    node.transform.ty = layout.location.y;
                }
                let lw = layout.size.width;
                let lh = layout.size.height;
                // Apply finite sizes. If Taffy returns inf, the node has no
                // constraint — leave its size as-is (0 or whatever was set).
                // The renderer handles 0-height containers by not clipping children.
                if lw > 0.0 && lw.is_finite() { node.width = lw; }
                if lh > 0.0 && lh.is_finite() { node.height = lh; }
            }
        }

        // Taffy can leave some auto-sized wrapper frames shorter than the
        // positioned children they contain in this DOM-shim pipeline. Ensure a
        // container is never shorter than the deepest child it actually lays out.
        for node_id in traversal.iter().rev() {
            let child_ids = match tree.children_of(node_id) {
                Some(ids) if !ids.is_empty() => ids,
                _ => continue,
            };

            let mut max_bottom = 0.0f32;
            for child_id in child_ids.iter() {
                let Some(child) = tree.get(child_id) else { continue };
                max_bottom = max_bottom.max(child.transform.ty + child.height);
            }

            if max_bottom <= 0.0 {
                continue;
            }

            if let Some(node) = tree.get_mut(node_id) {
                if let NodeKind::Frame { .. } = node.kind {
                    if node.height < max_bottom {
                        node.height = max_bottom;
                    }
                }
            }
        }
    }
}

/// Convert a Rendero Node to a Taffy Style.
fn node_to_taffy_style(node: &crate::node::Node, parent: Option<&crate::node::Node>) -> taffy::Style {
    let mut style = taffy::Style::default();

    if !node.visible {
        style.display = Display::None;
        return style;
    }

    // Check for auto-layout (flex container)
    let auto_layout = match &node.kind {
        NodeKind::Frame { auto_layout: Some(al), .. } => Some(al.clone()),
        _ => None,
    };

    if let Some(al) = auto_layout {
        style.display = Display::Flex;

        style.flex_direction = match al.direction {
            LayoutDirection::Horizontal => FlexDirection::Row,
            LayoutDirection::Vertical => FlexDirection::Column,
        };

        style.gap = Size {
            width: LengthPercentage::length(al.spacing),
            height: LengthPercentage::length(al.spacing),
        };

        style.padding = taffy::Rect {
            top: LengthPercentage::length(al.padding_top),
            right: LengthPercentage::length(al.padding_right),
            bottom: LengthPercentage::length(al.padding_bottom),
            left: LengthPercentage::length(al.padding_left),
        };

        style.align_items = Some(match al.align {
            LayoutAlign::Start => AlignItems::FlexStart,
            LayoutAlign::Center => AlignItems::Center,
            LayoutAlign::End => AlignItems::FlexEnd,
            LayoutAlign::Stretch => AlignItems::Stretch,
        });

        style.justify_content = Some(match al.justify {
            LayoutJustify::Start => JustifyContent::FlexStart,
            LayoutJustify::Center => JustifyContent::Center,
            LayoutJustify::End => JustifyContent::FlexEnd,
            LayoutJustify::SpaceBetween => JustifyContent::SpaceBetween,
            LayoutJustify::SpaceAround => JustifyContent::SpaceAround,
            LayoutJustify::SpaceEvenly => JustifyContent::SpaceEvenly,
        });

        style.flex_wrap = match al.wrap {
            LayoutWrap::Wrap => FlexWrap::Wrap,
            LayoutWrap::NoWrap => FlexWrap::NoWrap,
        };

        // Container size comes from node.width/height directly — same as any node.
        //
        // CHANGE (rev 63de7cf): Removed primary_sizing/counter_sizing from container
        // sizing path. These Figma-style fields conflated container sizing with item
        // sizing. In CSS flexbox, a node's size as a container is independent of how
        // it sizes as a flex item in its parent. Item sizing (flex-grow, flex-basis)
        // is handled below via node.horizontal_sizing/vertical_sizing.
        //
        // AutoLayout.primary_sizing/counter_sizing still exist on the struct for
        // backwards compat with legacy layout and CRDT ops, but the Taffy path
        // no longer reads them. Container size is node.width/height, item size is
        // node.horizontal_sizing/vertical_sizing. No conflation.
        // Do not copy node.width/node.height into the authored container size here.
        // Those fields are also overwritten with computed layout results after each
        // pass, and treating them as authored inputs makes hug/auto containers stick
        // to their previous computed size on the next layout.
    } else {
        // No explicit auto-layout. Frames default to vertical column (CSS block flow).
        // Leaf nodes (rect, text, ellipse) just get explicit dimensions.
        style.display = Display::Flex;
        match &node.kind {
            NodeKind::Frame { .. } => {
                // Block-level container: stack children vertically (CSS default)
                style.flex_direction = FlexDirection::Column;
            }
            _ => {}
        }
        // Fixed-size behavior is handled below from sizing mode / percentages.
    }

    let parent_direction = match parent.and_then(|p| match &p.kind {
        NodeKind::Frame { auto_layout: Some(al), .. } => Some(al.direction),
        _ => None,
    }) {
        Some(dir) => dir,
        None => LayoutDirection::Vertical,
    };

    match (parent_direction, node.horizontal_sizing) {
        (LayoutDirection::Horizontal, SizingMode::Fill) => {
            style.flex_grow = 1.0;
            style.flex_shrink = 1.0;
            style.flex_basis = Dimension::length(0.0);
            style.size.width = Dimension::auto();
        }
        (LayoutDirection::Vertical, SizingMode::Fill) => {
            style.align_self = Some(AlignSelf::Stretch);
            style.size.width = Dimension::auto();
        }
        (_, SizingMode::Fixed) if node.width > 0.0 => {
            style.size.width = Dimension::length(node.width);
        }
        _ => {}
    }

    match (parent_direction, node.vertical_sizing) {
        (LayoutDirection::Vertical, SizingMode::Fill) => {
            style.flex_grow = 1.0;
            style.flex_shrink = 1.0;
            style.flex_basis = Dimension::length(0.0);
            style.size.height = Dimension::auto();
        }
        (LayoutDirection::Horizontal, SizingMode::Fill) => {
            style.align_self = Some(AlignSelf::Stretch);
            style.size.height = Dimension::auto();
        }
        (_, SizingMode::Fixed) if node.height > 0.0 => {
            style.size.height = Dimension::length(node.height);
        }
        _ => {}
    }

    if let Some(width_percent) = node.width_percent {
        style.size.width = Dimension::percent(width_percent);
        style.flex_grow = 0.0;
        style.flex_basis = Dimension::auto();
        if matches!(parent_direction, LayoutDirection::Vertical) {
            style.align_self = None;
        }
    }
    if let Some(height_percent) = node.height_percent {
        style.size.height = Dimension::percent(height_percent);
    }

    // Margin
    let margin_edge = |value: f32, is_auto: bool| {
        if is_auto {
            LengthPercentageAuto::auto()
        } else {
            LengthPercentageAuto::length(value)
        }
    };
    style.margin = taffy::Rect {
        top: margin_edge(node.margin.top, node.margin.auto_top),
        right: margin_edge(node.margin.right, node.margin.auto_right),
        bottom: margin_edge(node.margin.bottom, node.margin.auto_bottom),
        left: margin_edge(node.margin.left, node.margin.auto_left),
    };

    // Absolute positioning
    if let Some(lp) = node.layout_position {
        style.position = Position::Absolute;
        style.inset = taffy::Rect {
            top: LengthPercentageAuto::length(lp.y),
            right: LengthPercentageAuto::auto(),
            bottom: LengthPercentageAuto::auto(),
            left: LengthPercentageAuto::length(lp.x),
        };
    }

    // Min/max size constraints
    if node.size_constraints.min_width > 0.0 {
        style.min_size.width = Dimension::length(node.size_constraints.min_width);
    }
    if node.size_constraints.min_height > 0.0 {
        style.min_size.height = Dimension::length(node.size_constraints.min_height);
    }
    if node.size_constraints.max_width > 0.0 {
        style.max_size.width = Dimension::length(node.size_constraints.max_width);
    }
    if node.size_constraints.max_height > 0.0 {
        style.max_size.height = Dimension::length(node.size_constraints.max_height);
    }

    style
}


// ═══════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::node::Node;
    use crate::providers::HeuristicTextMeasurer;

    fn make_doc() -> Document {
        Document::new("test", 1)
    }

    fn root_id(doc: &Document) -> NodeId {
        doc.page(0).unwrap().tree.root_id()
    }

    fn add_frame(doc: &mut Document, parent: NodeId, name: &str, w: f32, h: f32) -> NodeId {
        let id = doc.next_id();
        let mut node = Node::frame(id, name, w, h);
        doc.add_node(0, node, parent, usize::MAX).unwrap();
        id
    }

    fn add_text(doc: &mut Document, parent: NodeId, name: &str, text: &str, font_size: f32) -> NodeId {
        let id = doc.next_id();
        let color = crate::properties::Color::new(0.0, 0.0, 0.0, 1.0);
        let node = Node::text(id, name, text, font_size, color);
        doc.add_node(0, node, parent, usize::MAX).unwrap();
        id
    }

    fn set_auto_layout(doc: &mut Document, node_id: NodeId, dir: LayoutDirection, spacing: f32) {
        let page = doc.page_mut(0).unwrap();
        let node = page.tree.get_mut(&node_id).unwrap();
        if let NodeKind::Frame { ref mut auto_layout, .. } = node.kind {
            *auto_layout = Some(AutoLayout {
                direction: dir,
                spacing,
                padding_top: 0.0,
                padding_right: 0.0,
                padding_bottom: 0.0,
                padding_left: 0.0,
                primary_sizing: SizingMode::Fixed,
                counter_sizing: SizingMode::Fixed,
                align: LayoutAlign::Start,
                justify: LayoutJustify::Start,
                wrap: LayoutWrap::NoWrap,
            });
        }
    }

    #[test]
    fn test_vertical_stack_positions() {
        let mut doc = make_doc();
        let root = root_id(&doc);

        // Create container 400x600 with vertical auto-layout, gap=10
        let container = add_frame(&mut doc, root, "container", 400.0, 600.0);
        set_auto_layout(&mut doc, container, LayoutDirection::Vertical, 10.0);

        // Add 3 children: 400x100 each
        let c1 = add_frame(&mut doc, container, "child1", 400.0, 100.0);
        let c2 = add_frame(&mut doc, container, "child2", 400.0, 100.0);
        let c3 = add_frame(&mut doc, container, "child3", 400.0, 100.0);

        // Run Taffy layout
        let mut engine = TaffyLayout::new(HeuristicTextMeasurer);
        let page = doc.page_mut(0).unwrap();
        engine.compute(&mut page.tree, &root, (800.0, 600.0));

        // Verify positions
        let n1 = page.tree.get(&c1).unwrap();
        let n2 = page.tree.get(&c2).unwrap();
        let n3 = page.tree.get(&c3).unwrap();

        assert_eq!(n1.transform.ty, 0.0, "child1 y should be 0");
        assert_eq!(n2.transform.ty, 110.0, "child2 y should be 100 + 10 gap = 110");
        assert_eq!(n3.transform.ty, 220.0, "child3 y should be 200 + 20 gap = 220");
    }

    #[test]
    fn test_horizontal_row_positions() {
        let mut doc = make_doc();
        let root = root_id(&doc);

        let container = add_frame(&mut doc, root, "row", 600.0, 100.0);
        set_auto_layout(&mut doc, container, LayoutDirection::Horizontal, 20.0);

        let c1 = add_frame(&mut doc, container, "c1", 100.0, 50.0);
        let c2 = add_frame(&mut doc, container, "c2", 150.0, 50.0);
        let c3 = add_frame(&mut doc, container, "c3", 100.0, 50.0);

        let mut engine = TaffyLayout::new(HeuristicTextMeasurer);
        let page = doc.page_mut(0).unwrap();
        engine.compute(&mut page.tree, &root, (800.0, 600.0));

        let n1 = page.tree.get(&c1).unwrap();
        let n2 = page.tree.get(&c2).unwrap();
        let n3 = page.tree.get(&c3).unwrap();

        assert_eq!(n1.transform.tx, 0.0);
        assert_eq!(n2.transform.tx, 120.0); // 100 + 20 gap
        assert_eq!(n3.transform.tx, 290.0); // 100 + 20 + 150 + 20
    }

    #[test]
    fn test_padding() {
        let mut doc = make_doc();
        let root = root_id(&doc);

        let container = add_frame(&mut doc, root, "padded", 400.0, 400.0);
        {
            let page = doc.page_mut(0).unwrap();
            let node = page.tree.get_mut(&container).unwrap();
            if let NodeKind::Frame { ref mut auto_layout, .. } = node.kind {
                *auto_layout = Some(AutoLayout {
                    direction: LayoutDirection::Vertical,
                    spacing: 0.0,
                    padding_top: 20.0,
                    padding_right: 30.0,
                    padding_bottom: 20.0,
                    padding_left: 30.0,
                    primary_sizing: SizingMode::Fixed,
                    counter_sizing: SizingMode::Fixed,
                    align: LayoutAlign::Start,
                    justify: LayoutJustify::Start,
                    wrap: LayoutWrap::NoWrap,
                });
            }
        }

        let child = add_frame(&mut doc, container, "child", 100.0, 50.0);

        let mut engine = TaffyLayout::new(HeuristicTextMeasurer);
        let page = doc.page_mut(0).unwrap();
        engine.compute(&mut page.tree, &root, (800.0, 600.0));

        let n = page.tree.get(&child).unwrap();
        assert_eq!(n.transform.tx, 30.0, "child should be offset by left padding");
        assert_eq!(n.transform.ty, 20.0, "child should be offset by top padding");
    }

    #[test]
    fn test_text_measurement_used() {
        let mut doc = make_doc();
        let root = root_id(&doc);

        let container = add_frame(&mut doc, root, "container", 400.0, 200.0);
        set_auto_layout(&mut doc, container, LayoutDirection::Vertical, 0.0);

        let text = add_text(&mut doc, container, "label", "Hello World", 16.0);

        let mut engine = TaffyLayout::new(HeuristicTextMeasurer);
        let page = doc.page_mut(0).unwrap();
        engine.compute(&mut page.tree, &root, (800.0, 600.0));

        let tn = page.tree.get(&text).unwrap();
        // HeuristicTextMeasurer: width = 11 * 16 * 0.65 = 114.4, height = 16 * 1.5 = 24
        assert!(tn.width > 100.0, "text width should be measured: got {}", tn.width);
        assert!(tn.height > 20.0, "text height should be measured: got {}", tn.height);
    }

    #[test]
    fn test_nested_layout() {
        let mut doc = make_doc();
        let root = root_id(&doc);

        // Outer: vertical, 400x400
        let outer = add_frame(&mut doc, root, "outer", 400.0, 400.0);
        set_auto_layout(&mut doc, outer, LayoutDirection::Vertical, 10.0);

        // Inner: horizontal, 400x100
        let inner = add_frame(&mut doc, outer, "inner", 400.0, 100.0);
        set_auto_layout(&mut doc, inner, LayoutDirection::Horizontal, 5.0);

        // Two children in inner row
        let a = add_frame(&mut doc, inner, "a", 100.0, 80.0);
        let b = add_frame(&mut doc, inner, "b", 100.0, 80.0);

        // Another child in outer column
        let c = add_frame(&mut doc, outer, "c", 400.0, 50.0);

        let mut engine = TaffyLayout::new(HeuristicTextMeasurer);
        let page = doc.page_mut(0).unwrap();
        engine.compute(&mut page.tree, &root, (800.0, 600.0));

        // Inner row should be at y=0 in outer
        let inner_n = page.tree.get(&inner).unwrap();
        assert_eq!(inner_n.transform.ty, 0.0);

        // "c" should be below inner (100 + 10 gap = 110)
        let c_n = page.tree.get(&c).unwrap();
        assert_eq!(c_n.transform.ty, 110.0);

        // "a" should be at x=0 within inner
        let a_n = page.tree.get(&a).unwrap();
        assert_eq!(a_n.transform.tx, 0.0);

        // "b" should be at x=105 (100 + 5 gap)
        let b_n = page.tree.get(&b).unwrap();
        assert_eq!(b_n.transform.tx, 105.0);
    }

    #[test]
    fn test_taffy_matches_expected_vertical() {
        // Verify Taffy produces expected positions for a simple vertical layout
        let mut doc = make_doc();
        let root = root_id(&doc);

        let container = add_frame(&mut doc, root, "container", 300.0, 500.0);
        set_auto_layout(&mut doc, container, LayoutDirection::Vertical, 10.0);
        let c1 = add_frame(&mut doc, container, "c1", 300.0, 80.0);
        let c2 = add_frame(&mut doc, container, "c2", 300.0, 60.0);

        let mut engine = TaffyLayout::new(HeuristicTextMeasurer);
        let page = doc.page_mut(0).unwrap();
        engine.compute(&mut page.tree, &root, (800.0, 600.0));

        let n1 = page.tree.get(&c1).unwrap();
        let n2 = page.tree.get(&c2).unwrap();

        // Expected: c1 at y=0, c2 at y=80+10=90
        assert_eq!(n1.transform.ty, 0.0, "c1 y");
        assert_eq!(n2.transform.ty, 90.0, "c2 y = 80 + 10 gap");
    }

    /// Mock measurer for testing — returns fixed dimensions
    struct MockTextMeasurer { width: f32, height: f32 }
    impl TextMeasurer for MockTextMeasurer {
        fn measure(&self, _runs: &[crate::node::TextRun], _max_width: f32) -> (f32, f32) {
            (self.width, self.height)
        }
    }

    #[test]
    fn test_custom_text_measurer() {
        let mut doc = make_doc();
        let root = root_id(&doc);

        let container = add_frame(&mut doc, root, "c", 400.0, 200.0);
        set_auto_layout(&mut doc, container, LayoutDirection::Vertical, 0.0);
        let text = add_text(&mut doc, container, "t", "X", 16.0);

        // Use mock that returns 200x40 regardless of content
        let mut engine = TaffyLayout::new(MockTextMeasurer { width: 200.0, height: 40.0 });
        let page = doc.page_mut(0).unwrap();
        engine.compute(&mut page.tree, &root, (800.0, 600.0));

        let tn = page.tree.get(&text).unwrap();
        assert_eq!(tn.width, 200.0, "should use mock measurer width");
        assert_eq!(tn.height, 40.0, "should use mock measurer height");
    }
}
