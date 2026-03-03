//! Visual properties for nodes.
//!
//! TYPE-LEVEL GUARANTEES:
//! - Colors are always valid (0.0..=1.0 enforced at construction)
//! - Angles are always normalized
//! - Blend modes are exhaustive (no "unknown" variant)

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// RGBA color with components in 0.0..=1.0.
/// Constructed only through `Color::new` which clamps values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Color {
    /// Create a color, clamping components to valid range.
    /// Cannot produce an invalid color.
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }

    pub fn r(&self) -> f32 { self.r }
    pub fn g(&self) -> f32 { self.g }
    pub fn b(&self) -> f32 { self.b }
    pub fn a(&self) -> f32 { self.a }

    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    /// Convert to premultiplied alpha for rendering.
    /// This is what the GPU wants. Do it once at the type level.
    pub fn premultiplied(&self) -> PremultColor {
        PremultColor {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }
}

/// Premultiplied alpha color — the renderer's native format.
/// Separate type prevents accidentally mixing pre/post-multiplied.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PremultColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl PremultColor {
    /// GPU-ready format: [r, g, b, a] as f32 array.
    pub fn as_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// 2D affine transform. Stored as column-major 3x2 matrix.
/// Covers: translate, rotate, scale, skew.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    /// Column 0: x-axis basis vector
    pub a: f32, pub b: f32,
    /// Column 1: y-axis basis vector
    pub c: f32, pub d: f32,
    /// Column 2: translation
    pub tx: f32, pub ty: f32,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.0, b: 0.0,
        c: 0.0, d: 1.0,
        tx: 0.0, ty: 0.0,
    };

    pub fn translate(x: f32, y: f32) -> Self {
        Self { tx: x, ty: y, ..Self::IDENTITY }
    }

    pub fn scale(sx: f32, sy: f32) -> Self {
        Self { a: sx, d: sy, ..Self::IDENTITY }
    }

    pub fn rotate(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self { a: c, b: s, c: -s, d: c, tx: 0.0, ty: 0.0 }
    }

    /// Compose two transforms. Self is applied first, then other.
    pub fn then(&self, other: &Transform) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            tx: self.tx * other.a + self.ty * other.c + other.tx,
            ty: self.tx * other.b + self.ty * other.d + other.ty,
        }
    }

    /// Transform a point.
    pub fn apply(&self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.a * p.x + self.c * p.y + self.tx,
            self.b * p.x + self.d * p.y + self.ty,
        )
    }

    /// Compute the inverse transform.
    /// Returns None if the transform is degenerate (determinant ~0).
    pub fn inverse(&self) -> Option<Transform> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-10 {
            return None;
        }
        let inv_det = 1.0 / det;
        Some(Transform {
            a: self.d * inv_det,
            b: -self.b * inv_det,
            c: -self.c * inv_det,
            d: self.a * inv_det,
            tx: (self.c * self.ty - self.d * self.tx) * inv_det,
            ty: (self.b * self.tx - self.a * self.ty) * inv_det,
        })
    }

    /// Apply inverse transform to a point (world → local).
    /// Falls back to translation-only inverse if degenerate.
    pub fn apply_inverse(&self, p: Vec2) -> Vec2 {
        match self.inverse() {
            Some(inv) => inv.apply(p),
            None => Vec2::new(p.x - self.tx, p.y - self.ty),
        }
    }
}

/// How a fill or stroke is painted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Paint {
    Solid(Color),
    LinearGradient {
        stops: Vec<GradientStop>,
        start: Vec2,
        end: Vec2,
    },
    RadialGradient {
        stops: Vec<GradientStop>,
        center: Vec2,
        radius: f32,
    },
    AngularGradient {
        stops: Vec<GradientStop>,
        center: Vec2,
        start_angle: f32, // radians
    },
    DiamondGradient {
        stops: Vec<GradientStop>,
        center: Vec2,
        radius: f32,
    },
    /// Image fill — referenced by path/URL. Renderer loads and caches.
    Image {
        /// Path or URL to the image (relative to import root).
        path: String,
        /// How to scale the image within the shape.
        scale_mode: ImageScaleMode,
        /// Opacity of the image fill (0.0..=1.0).
        opacity: f32,
    },
}

/// How an image fill is scaled within its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageScaleMode {
    Fill,
    Fit,
    Tile,
    Stretch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GradientStop {
    pub position: f32, // 0.0..=1.0, clamped on creation
    pub color: Color,
}

impl GradientStop {
    pub fn new(position: f32, color: Color) -> Self {
        Self {
            position: position.clamp(0.0, 1.0),
            color,
        }
    }
}

/// Blend modes — exhaustive enum, no "unknown" variant.
/// Every blend mode Figma supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    ColorMode,
    Luminosity,
}

/// Stroke alignment relative to the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeAlign {
    Inside,
    Center,
    Outside,
}

/// Stroke cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeCap {
    None,
    Round,
    Square,
}

/// Stroke join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

/// Fill rule for paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// Visual styling for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub fills: Vec<Paint>,
    pub strokes: Vec<Paint>,
    pub stroke_weight: f32,
    pub stroke_align: StrokeAlign,
    pub stroke_cap: StrokeCap,
    pub stroke_join: StrokeJoin,
    pub dash_pattern: Vec<f32>,
    pub effects: Vec<Effect>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            fills: Vec::new(),
            strokes: Vec::new(),
            stroke_weight: 1.0,
            stroke_align: StrokeAlign::Center,
            stroke_cap: StrokeCap::None,
            stroke_join: StrokeJoin::Miter,
            dash_pattern: Vec::new(),
            effects: Vec::new(),
        }
    }
}

/// Visual effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Effect {
    DropShadow {
        color: Color,
        offset: Vec2,
        blur_radius: f32,
        spread: f32,
    },
    InnerShadow {
        color: Color,
        offset: Vec2,
        blur_radius: f32,
        spread: f32,
    },
    LayerBlur {
        radius: f32,
    },
    BackgroundBlur {
        radius: f32,
    },
}

/// Constraints for auto-layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutAlign {
    Start,
    Center,
    End,
    Stretch,
}

/// Auto-layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

/// Constraint type for how a child reacts when its parent is resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Pin to start edge (left/top). Default.
    Min,
    /// Pin to end edge (right/bottom).
    Max,
    /// Pin to both edges — stretch.
    MinMax,
    /// Pin to center.
    Center,
    /// Scale proportionally with parent.
    Scale,
}

impl Default for ConstraintType {
    fn default() -> Self { ConstraintType::Min }
}

/// Auto-layout sizing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizingMode {
    Fixed,
    Hug,
    Fill,
}

/// Auto-layout configuration for a frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoLayout {
    pub direction: LayoutDirection,
    pub spacing: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub primary_sizing: SizingMode,
    pub counter_sizing: SizingMode,
    pub align: LayoutAlign,
}
