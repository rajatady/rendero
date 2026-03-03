//! Render verification — deterministic pixel tests.
//!
//! PRINCIPLE: Never visually inspect renders. Instead:
//! 1. Render a known scene to pixels
//! 2. Hash the output
//! 3. Compare to expected hash
//! 4. Output: PASS or FAIL (one line, ~10 tokens)
//!
//! For debugging mismatches: sample specific pixels at known coordinates
//! and assert exact RGBA values.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::pipeline;
use crate::scene::AABB;
use rendero_core::id::ClockGen;
use rendero_core::node::Node;
use rendero_core::properties::*;
use rendero_core::tree::DocumentTree;
use glam::Vec2;

/// Hash a pixel buffer deterministically.
pub fn hash_pixels(pixels: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    pixels.hash(&mut hasher);
    hasher.finish()
}

/// A render test case.
pub struct RenderTest {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub setup: fn(&mut DocumentTree, &mut ClockGen),
    pub expected_hash: Option<u64>,
    /// Specific pixel assertions: (x, y, r, g, b, a)
    pub pixel_checks: Vec<(u32, u32, u8, u8, u8, u8)>,
    /// Pixels that must have alpha > 0 (something was rendered there).
    /// Useful for text/path tests where exact values are font-dependent.
    pub opaque_checks: Vec<(u32, u32)>,
}

/// Result of a render test.
pub enum TestResult {
    Pass,
    HashMismatch { expected: u64, got: u64 },
    PixelMismatch { x: u32, y: u32, expected: (u8, u8, u8, u8), got: (u8, u8, u8, u8) },
    /// First run — no expected hash yet. Returns hash for storage.
    NewHash(u64),
}

/// Run a render test. Returns structured result.
pub fn run_test(test: &RenderTest) -> TestResult {
    let mut tree = DocumentTree::new();
    let mut clock = ClockGen::new(0);
    (test.setup)(&mut tree, &mut clock);

    let root = tree.root_id();
    let viewport = AABB::new(Vec2::ZERO, Vec2::new(test.width as f32, test.height as f32));
    let output = pipeline::render(&tree, &root, viewport);
    let pixels = output.to_pixels(test.width, test.height);

    // Check specific pixels first (more informative on failure)
    for &(x, y, er, eg, eb, ea) in &test.pixel_checks {
        let idx = ((y * test.width + x) * 4) as usize;
        if idx + 3 < pixels.len() {
            let (gr, gg, gb, ga) = (pixels[idx], pixels[idx+1], pixels[idx+2], pixels[idx+3]);
            // Allow ±1 tolerance for floating point rounding
            if (gr as i16 - er as i16).unsigned_abs() > 1
                || (gg as i16 - eg as i16).unsigned_abs() > 1
                || (gb as i16 - eb as i16).unsigned_abs() > 1
                || (ga as i16 - ea as i16).unsigned_abs() > 1
            {
                return TestResult::PixelMismatch {
                    x, y,
                    expected: (er, eg, eb, ea),
                    got: (gr, gg, gb, ga),
                };
            }
        }
    }

    // Check opaque assertions (must have alpha > 0)
    for &(x, y) in &test.opaque_checks {
        let idx = ((y * test.width + x) * 4) as usize;
        if idx + 3 < pixels.len() {
            let ga = pixels[idx + 3];
            if ga == 0 {
                return TestResult::PixelMismatch {
                    x, y,
                    expected: (0, 0, 0, 1), // alpha > 0 expected
                    got: (pixels[idx], pixels[idx+1], pixels[idx+2], 0),
                };
            }
        }
    }

    // Hash comparison
    let hash = hash_pixels(&pixels);
    match test.expected_hash {
        Some(expected) if expected == hash => TestResult::Pass,
        Some(expected) => TestResult::HashMismatch { expected, got: hash },
        None => TestResult::NewHash(hash),
    }
}

/// Run all built-in render tests. Returns (passed, failed, new) counts.
pub fn run_all_tests() -> (usize, usize, usize) {
    let tests = builtin_tests();
    let mut passed = 0;
    let mut failed = 0;
    let mut new = 0;

    for test in &tests {
        match run_test(test) {
            TestResult::Pass => {
                passed += 1;
            }
            TestResult::HashMismatch { expected, got } => {
                eprintln!("FAIL [{}]: hash expected={} got={}", test.name, expected, got);
                failed += 1;
            }
            TestResult::PixelMismatch { x, y, expected, got } => {
                eprintln!("FAIL [{}]: pixel({},{}) expected={:?} got={:?}",
                    test.name, x, y, expected, got);
                failed += 1;
            }
            TestResult::NewHash(hash) => {
                eprintln!("NEW [{}]: hash={}", test.name, hash);
                new += 1;
            }
        }
    }

    (passed, failed, new)
}

/// Get all built-in test definitions (for snapshot tooling).
pub fn get_all_tests() -> Vec<RenderTest> {
    builtin_tests()
}

/// Render a test and return raw pixels + hash (for snapshot tooling).
pub fn render_test_pixels(test: &RenderTest) -> (Vec<u8>, u64) {
    let mut tree = DocumentTree::new();
    let mut clock = ClockGen::new(0);
    (test.setup)(&mut tree, &mut clock);

    let root = tree.root_id();
    let viewport = AABB::new(Vec2::ZERO, Vec2::new(test.width as f32, test.height as f32));
    let output = pipeline::render(&tree, &root, viewport);
    let pixels = output.to_pixels(test.width, test.height);
    let hash = hash_pixels(&pixels);
    (pixels, hash)
}

/// Built-in test scenes.
fn builtin_tests() -> Vec<RenderTest> {
    vec![
        RenderTest {
            name: "empty_canvas",
            width: 64,
            height: 64,
            setup: |_tree, _clock| {
                // Empty — just root node
            },
            expected_hash: None, // Will be set after first run
            pixel_checks: vec![
                (0, 0, 0, 0, 0, 0),   // Top-left: transparent
                (32, 32, 0, 0, 0, 0),  // Center: transparent
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "single_red_rect",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                let id = clock.next_node_id();
                let mut node = Node::rectangle(id, "red", 50.0, 50.0);
                node.transform = Transform::translate(10.0, 10.0);
                node.style.fills.push(Paint::Solid(Color::new(1.0, 0.0, 0.0, 1.0)));
                let root = tree.root_id();
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                (0, 0, 0, 0, 0, 0),       // Outside rect: transparent
                (25, 25, 255, 0, 0, 255),  // Inside rect: red
                (10, 10, 255, 0, 0, 255),  // Top-left corner of rect: red
                (60, 60, 0, 0, 0, 0),      // Just outside rect (10+50=60): transparent
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "overlapping_rects_alpha",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                let root = tree.root_id();

                // Red background rect
                let id1 = clock.next_node_id();
                let mut r1 = Node::rectangle(id1, "bg", 80.0, 80.0);
                r1.transform = Transform::translate(0.0, 0.0);
                r1.style.fills.push(Paint::Solid(Color::new(1.0, 0.0, 0.0, 1.0)));
                tree.insert(r1, root, 0).unwrap();

                // Semi-transparent blue on top
                let id2 = clock.next_node_id();
                let mut r2 = Node::rectangle(id2, "overlay", 60.0, 60.0);
                r2.transform = Transform::translate(20.0, 20.0);
                r2.style.fills.push(Paint::Solid(Color::new(0.0, 0.0, 1.0, 0.5)));
                tree.insert(r2, root, 1).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                (10, 10, 255, 0, 0, 255),  // Red only area
                // In overlap: blue(0,0,127,127) over red(255,0,0,255)
                // source-over: out = src + dst*(1-src_a) = (0,0,127) + (255,0,0)*(0.5) = (127,0,127)
                // alpha: 127 + 255*0.5 = 254
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "ellipse_center",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                let root = tree.root_id();
                let id = clock.next_node_id();
                let mut node = Node::rectangle(id, "ellipse", 80.0, 60.0);
                node.transform = Transform::translate(10.0, 20.0);
                node.kind = rendero_core::node::NodeKind::Ellipse {
                    arc_start: 0.0,
                    arc_end: std::f32::consts::TAU,
                    inner_radius_ratio: 0.0,
                };
                node.style.fills.push(Paint::Solid(Color::new(0.0, 1.0, 0.0, 1.0)));
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                (50, 50, 0, 255, 0, 255),  // Center of ellipse: green
                (0, 0, 0, 0, 0, 0),        // Corner: outside ellipse
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "triangle_path",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                use rendero_core::node::{NodeKind, PathCommand, VectorPath};
                use glam::Vec2;

                let root = tree.root_id();
                let id = clock.next_node_id();
                let mut node = Node::rectangle(id, "triangle", 100.0, 100.0);
                node.transform = Transform::translate(0.0, 0.0);
                node.kind = NodeKind::Vector {
                    paths: vec![VectorPath {
                        commands: vec![
                            PathCommand::MoveTo(Vec2::new(50.0, 10.0)),
                            PathCommand::LineTo(Vec2::new(90.0, 90.0)),
                            PathCommand::LineTo(Vec2::new(10.0, 90.0)),
                            PathCommand::Close,
                        ],
                        fill_rule: FillRule::NonZero,
                    }],
                };
                node.style.fills.push(Paint::Solid(Color::new(1.0, 0.0, 1.0, 1.0)));
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                (50, 50, 255, 0, 255, 255),  // Center of triangle: magenta
                (5, 5, 0, 0, 0, 0),          // Top-left corner: outside
                (50, 95, 0, 0, 0, 0),        // Below triangle: outside
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "rotated_rect",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                let root = tree.root_id();
                let id = clock.next_node_id();
                let mut node = Node::rectangle(id, "rotated", 40.0, 40.0);
                // Translate to center, then rotate 45 degrees
                let rot = Transform::rotate(std::f32::consts::FRAC_PI_4);
                let trans = Transform::translate(50.0, 30.0);
                node.transform = rot.then(&trans);
                node.style.fills.push(Paint::Solid(Color::new(0.0, 1.0, 1.0, 1.0)));
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // The rotated rect should have a pixel near the transform origin
                // Hard to assert exact pixels for rotation, so just check outside
                (0, 0, 0, 0, 0, 0), // Far corner: should be empty
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "linear_gradient_rect",
            width: 100,
            height: 50,
            setup: |tree, clock| {
                let root = tree.root_id();
                let id = clock.next_node_id();
                let mut node = Node::rectangle(id, "gradient", 100.0, 50.0);
                node.transform = Transform::translate(0.0, 0.0);
                node.style.fills.push(Paint::LinearGradient {
                    stops: vec![
                        GradientStop::new(0.0, Color::new(1.0, 0.0, 0.0, 1.0)), // Red at left
                        GradientStop::new(1.0, Color::new(0.0, 0.0, 1.0, 1.0)), // Blue at right
                    ],
                    start: Vec2::new(0.0, 0.0),
                    end: Vec2::new(1.0, 0.0),
                });
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // x=0, pixel center at 0.5: t=0.005, nearly red
                // x=99, pixel center at 99.5: t=0.995, nearly blue
                // Use approximate: just ensure left side is more red than blue,
                // and right side is more blue than red.
                // Exact at center: x=50, center at 50.5: t=0.505
                // r = 255*(1-0.505) = 126, b = 255*0.505 = 128
                (50, 25, 126, 0, 128, 255),
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "radial_gradient_rect",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                let root = tree.root_id();
                let id = clock.next_node_id();
                let mut node = Node::rectangle(id, "radial", 100.0, 100.0);
                node.transform = Transform::translate(0.0, 0.0);
                node.style.fills.push(Paint::RadialGradient {
                    stops: vec![
                        GradientStop::new(0.0, Color::new(1.0, 1.0, 1.0, 1.0)), // White at center
                        GradientStop::new(1.0, Color::new(0.0, 0.0, 0.0, 1.0)), // Black at edge
                    ],
                    center: Vec2::new(0.5, 0.5),
                    radius: 0.5,
                });
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // Center pixel at (50.5, 50.5): dist from (50,50) = 0.707, t = 0.707/50 ≈ 0.014
                // Color: lerp(white, black, 0.014) ≈ (251, 251, 251)
                (50, 50, 251, 251, 251, 255),
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "boolean_subtract",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                use rendero_core::node::{BooleanOperation, NodeKind};

                let root = tree.root_id();

                // Create a BooleanOp(Subtract) node
                let bool_id = clock.next_node_id();
                let mut bool_node = Node::rectangle(bool_id, "bool_sub", 100.0, 100.0);
                bool_node.kind = NodeKind::BooleanOp {
                    operation: BooleanOperation::Subtract,
                };
                bool_node.style.fills.push(Paint::Solid(Color::new(1.0, 0.0, 0.0, 1.0)));
                tree.insert(bool_node, root, 0).unwrap();

                // First child: large rect (base shape)
                let c1 = clock.next_node_id();
                let child1 = Node::rectangle(c1, "base", 80.0, 80.0);
                tree.insert(child1, bool_id, 0).unwrap();

                // Second child: smaller rect to subtract (overlapping)
                let c2 = clock.next_node_id();
                let mut child2 = Node::rectangle(c2, "cut", 40.0, 40.0);
                child2.transform = Transform::translate(20.0, 20.0);
                tree.insert(child2, bool_id, 1).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // (10, 10) is inside base but outside cut → should be filled (red)
                (10, 10, 255, 0, 0, 255),
                // (40, 40) is inside both → subtracted, should be empty
                (40, 40, 0, 0, 0, 0),
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "text_renders_pixels",
            width: 200,
            height: 50,
            setup: |tree, clock| {
                let root = tree.root_id();
                let id = clock.next_node_id();
                let node = Node::text(
                    id, "test-text", "AB",
                    24.0,
                    Color::new(1.0, 1.0, 1.0, 1.0),
                );
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // Text "AB" at 24px font renders within the bounding box.
                // We can't assert exact glyph pixels (font-dependent), but we CAN
                // assert that SOME pixels are non-transparent (i.e., text actually rendered).
                // The bounding box is at (0,0), ~31x36 pixels.
                // If this pixel check fails with (0,0,0,0), text rendering is broken.
                // We check a pixel roughly in the middle of where "A" should be (~8, 18)
                // Just verify it's not fully transparent — any alpha > 0 means text rendered.
            ],
            // Text "AB" at 24px renders in bounding box x=[0,27] y=[7,24].
            // Coordinates verified by diagnostic — these are ON glyph strokes.
            opaque_checks: vec![
                (17, 15),  // Glyph stroke (alpha=255)
                (16, 11),  // Upper glyph stroke (alpha=251)
                (25, 21),  // Lower-right "B" stroke (alpha=255)
            ],
        },
        RenderTest {
            name: "boolean_union",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                use rendero_core::node::{BooleanOperation, NodeKind};

                let root = tree.root_id();

                let bool_id = clock.next_node_id();
                let mut bool_node = Node::rectangle(bool_id, "bool_union", 100.0, 100.0);
                bool_node.kind = NodeKind::BooleanOp {
                    operation: BooleanOperation::Union,
                };
                bool_node.style.fills.push(Paint::Solid(Color::new(0.0, 1.0, 0.0, 1.0)));
                tree.insert(bool_node, root, 0).unwrap();

                // Two overlapping rects
                let c1 = clock.next_node_id();
                let child1 = Node::rectangle(c1, "a", 60.0, 40.0);
                tree.insert(child1, bool_id, 0).unwrap();

                let c2 = clock.next_node_id();
                let mut child2 = Node::rectangle(c2, "b", 40.0, 60.0);
                child2.transform = Transform::translate(20.0, 20.0);
                tree.insert(child2, bool_id, 1).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // (10, 10) inside first rect only → filled (green)
                (10, 10, 0, 255, 0, 255),
                // (30, 30) inside both → filled (green, union)
                (30, 30, 0, 255, 0, 255),
                // (50, 70) inside second rect only → filled (green)
                (50, 70, 0, 255, 0, 255),
                // (70, 70) outside both → empty
                (70, 70, 0, 0, 0, 0),
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "image_node_pixel_sampling",
            width: 64,
            height: 64,
            setup: |tree, clock| {
                // 2x2 image: [red, green, blue, yellow] rendered at 40x30
                let id = clock.next_node_id();
                let mut img = Node::image(id, "test_img", 40.0, 30.0, 2, 2,
                    vec![255,0,0,255, 0,255,0,255, 0,0,255,255, 255,255,0,255]);
                img.transform = Transform::translate(10.0, 10.0);
                let root = tree.root_id();
                tree.insert(img, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // (15, 15) → local (5,5) → top-left quadrant → red
                (15, 15, 255, 0, 0, 255),
                // (40, 15) → local (30,5) → top-right quadrant → green
                (40, 15, 0, 255, 0, 255),
                // (15, 30) → local (5,20) → bottom-left quadrant → blue
                (15, 30, 0, 0, 255, 255),
                // (40, 30) → local (30,20) → bottom-right quadrant → yellow
                (40, 30, 255, 255, 0, 255),
                // (5, 5) outside → transparent
                (5, 5, 0, 0, 0, 0),
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "rounded_rect_corners",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                use rendero_core::node::CornerRadii;

                let root = tree.root_id();
                let id = clock.next_node_id();
                let mut node = Node::rectangle(id, "rounded", 80.0, 60.0);
                node.transform = Transform::translate(10.0, 20.0);
                node.kind = rendero_core::node::NodeKind::Rectangle {
                    corner_radii: CornerRadii::Uniform(15.0),
                };
                node.style.fills.push(Paint::Solid(Color::new(0.0, 0.5, 1.0, 1.0)));
                tree.insert(node, root, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // Center: should be filled (blue)
                (50, 50, 0, 127, 255, 255),
                // Top-left corner (10, 20): within the 15px radius curve, should be transparent
                (10, 20, 0, 0, 0, 0),
                // Well inside top-left: (30, 35) should be filled
                (30, 35, 0, 127, 255, 255),
            ],
            opaque_checks: vec![],
        },
        RenderTest {
            name: "frame_clips_children",
            width: 100,
            height: 100,
            setup: |tree, clock| {
                let root = tree.root_id();
                // Frame at (10,10), 40x40, clip_content=true
                let frame_id = clock.next_node_id();
                let mut frame = Node::frame(frame_id, "clip_frame", 40.0, 40.0);
                frame.transform = Transform::translate(10.0, 10.0);
                frame.style.fills.push(Paint::Solid(Color::new(0.5, 0.5, 0.5, 1.0)));
                tree.insert(frame, root, 0).unwrap();

                // Child rect at (20,20) relative to frame, 40x40 — extends beyond frame
                let child_id = clock.next_node_id();
                let mut child = Node::rectangle(child_id, "overflow", 40.0, 40.0);
                child.transform = Transform::translate(20.0, 20.0);
                child.style.fills.push(Paint::Solid(Color::new(1.0, 0.0, 0.0, 1.0)));
                tree.insert(child, frame_id, 0).unwrap();
            },
            expected_hash: None,
            pixel_checks: vec![
                // Inside frame, inside child (world 35, 35): red
                (35, 35, 255, 0, 0, 255),
                // Inside frame, outside child (world 15, 15): gray frame
                (15, 15, 127, 127, 127, 255),
                // Outside frame entirely (world 55, 55): should be transparent (clipped)
                (55, 55, 0, 0, 0, 0),
            ],
            opaque_checks: vec![],
        },
    ]
}

/// Performance benchmark: render N objects and return time in microseconds.
/// This answers: "can we handle 1000 artboards?"
pub fn bench_render(object_count: u32, viewport_size: u32) -> (u64, usize) {
    use std::time::Instant;

    let mut tree = DocumentTree::new();
    let mut clock = ClockGen::new(0);
    let root = tree.root_id();

    // Create N rectangles in a grid pattern
    let cols = (object_count as f32).sqrt().ceil() as u32;
    let cell_size = viewport_size as f32 / cols.max(1) as f32;

    for i in 0..object_count {
        let col = i % cols;
        let row = i / cols;
        let id = clock.next_node_id();
        let mut node = Node::rectangle(id, "r", cell_size * 0.8, cell_size * 0.8);
        node.transform = Transform::translate(
            col as f32 * cell_size + cell_size * 0.1,
            row as f32 * cell_size + cell_size * 0.1,
        );
        // Alternate colors
        let r = (i % 3 == 0) as u32 as f32;
        let g = (i % 3 == 1) as u32 as f32;
        let b = (i % 3 == 2) as u32 as f32;
        node.style.fills.push(Paint::Solid(Color::new(r, g, b, 1.0)));
        tree.insert(node, root, i as usize).unwrap();
    }

    let viewport = AABB::new(
        Vec2::ZERO,
        Vec2::new(viewport_size as f32, viewport_size as f32),
    );

    let start = Instant::now();
    let output = pipeline::render(&tree, &root, viewport);
    let _pixels = output.to_pixels(viewport_size, viewport_size);
    let elapsed_us = start.elapsed().as_micros() as u64;

    (elapsed_us, output.item_count)
}
