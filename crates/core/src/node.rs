//! Node types in the document tree.
//!
//! TYPE-LEVEL GUARANTEE: Each node kind carries ONLY the data relevant to it.
//! A Rectangle cannot have text properties. An Ellipse cannot have auto-layout.
//! This is enforced by the enum — not by runtime checks.
//!
//! The node enum is exhaustive. Every renderer, every CRDT operation,
//! every export must handle all variants. The compiler enforces this via
//! match exhaustiveness.

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::id::NodeId;
use crate::properties::*;

/// A path command for vector shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathCommand {
    MoveTo(Vec2),
    LineTo(Vec2),
    CubicTo {
        control1: Vec2,
        control2: Vec2,
        to: Vec2,
    },
    QuadTo {
        control: Vec2,
        to: Vec2,
    },
    Close,
}

/// A vector path — sequence of commands with a fill rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPath {
    pub commands: Vec<PathCommand>,
    pub fill_rule: FillRule,
}

/// Text horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justified,
}

/// Text vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextVerticalAlign {
    Top,
    Center,
    Bottom,
}

/// Text decoration style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextDecoration {
    None,
    Underline,
    Strikethrough,
}

/// A run of text with uniform styling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub italic: bool,
    pub color: Color,
    pub letter_spacing: f32,
    pub line_height: Option<f32>, // None = auto
    pub decoration: TextDecoration,
    /// Override fill for gradient text. When Some, renderer uses this instead of `color`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_override: Option<Paint>,
}

/// Text auto-resize behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextResize {
    None,
    Height,
    WidthAndHeight,
    Truncate,
}

/// Corner radii — can be uniform or per-corner.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CornerRadii {
    Uniform(f32),
    PerCorner {
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    },
}

impl Default for CornerRadii {
    fn default() -> Self {
        Self::Uniform(0.0)
    }
}

/// The kind of node — determines what data it carries.
/// Exhaustive: renderer MUST handle every variant (compiler enforces).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    /// A frame/group — can contain children, optionally has auto-layout.
    Frame {
        clip_content: bool,
        auto_layout: Option<AutoLayout>,
        corner_radii: CornerRadii,
    },

    /// A rectangle primitive.
    Rectangle {
        corner_radii: CornerRadii,
    },

    /// An ellipse primitive.
    Ellipse {
        /// Arc start/end in radians. Full ellipse = (0, 2π).
        arc_start: f32,
        arc_end: f32,
        /// Inner radius ratio for donuts. 0.0 = solid, 0.5 = half-hollow.
        inner_radius_ratio: f32,
    },

    /// A line primitive.
    Line,

    /// A polygon/star.
    Polygon {
        point_count: u32,
        /// 0.0 = regular polygon, 0.5 = star with half-indented points.
        inner_radius_ratio: f32,
    },

    /// A vector shape defined by paths.
    Vector {
        paths: Vec<VectorPath>,
    },

    /// A text node with styled runs.
    Text {
        runs: Vec<TextRun>,
        align: TextAlign,
        vertical_align: TextVerticalAlign,
        resize: TextResize,
    },

    /// A boolean operation combining child vector shapes.
    BooleanOp {
        operation: BooleanOperation,
    },

    /// A component definition (reusable).
    Component,

    /// An instance of a component.
    Instance {
        component_id: NodeId,
        /// Overridden properties. Only changed properties are stored.
        overrides: Vec<Override>,
    },

    /// A raster image.
    Image {
        /// Raw RGBA pixel data.
        data: Vec<u8>,
        /// Source image width in pixels.
        image_width: u32,
        /// Source image height in pixels.
        image_height: u32,
    },
}

/// Boolean operations on paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanOperation {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

/// A property override on a component instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Override {
    /// Path to the overridden node within the component.
    pub target_path: Vec<NodeId>,
    /// What is overridden.
    pub value: OverrideValue,
}

/// What can be overridden on a component instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverrideValue {
    Style(Style),
    Text(Vec<TextRun>),
    Visible(bool),
    // More as needed
}

/// A complete node with its identity, geometry, visual style, and kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,

    // Geometry
    pub transform: Transform,
    pub width: f32,
    pub height: f32,

    // Visual
    pub style: Style,

    // What this node IS
    pub kind: NodeKind,

    // Constraints for resizing within parent
    pub horizontal_sizing: SizingMode,
    pub vertical_sizing: SizingMode,

    // Figma-style constraints: how child responds to parent resize
    pub constraint_horizontal: ConstraintType,
    pub constraint_vertical: ConstraintType,

    /// If true, this node acts as a mask for its subsequent siblings.
    pub is_mask: bool,
}

impl Node {
    /// Create a new frame node.
    pub fn frame(id: NodeId, name: impl Into<String>, width: f32, height: f32) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            transform: Transform::IDENTITY,
            width,
            height,
            style: Style::default(),
            kind: NodeKind::Frame {
                clip_content: true,
                auto_layout: None,
                corner_radii: CornerRadii::default(),
            },
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        }
    }

    /// Create a new rectangle node.
    pub fn rectangle(id: NodeId, name: impl Into<String>, width: f32, height: f32) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            transform: Transform::IDENTITY,
            width,
            height,
            style: Style::default(),
            kind: NodeKind::Rectangle {
                corner_radii: CornerRadii::default(),
            },
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        }
    }

    pub fn ellipse(id: NodeId, name: impl Into<String>, width: f32, height: f32) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            transform: Transform::IDENTITY,
            width,
            height,
            style: Style::default(),
            kind: NodeKind::Ellipse {
                arc_start: 0.0,
                arc_end: std::f32::consts::TAU,
                inner_radius_ratio: 0.0,
            },
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        }
    }

    pub fn text(id: NodeId, name: impl Into<String>, content: &str, font_size: f32, color: Color) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            transform: Transform::IDENTITY,
            width: content.len() as f32 * font_size * 0.65,
            height: font_size * 1.5,
            style: Style::default(),
            kind: NodeKind::Text {
                runs: vec![TextRun {
                    text: content.to_string(),
                    font_family: "Inter".to_string(),
                    font_size,
                    font_weight: 400,
                    italic: false,
                    color,
                    letter_spacing: 0.0,
                    line_height: None,
                    decoration: TextDecoration::None,
                    fill_override: None,
                }],
                align: TextAlign::Left,
                vertical_align: TextVerticalAlign::Top,
                resize: TextResize::WidthAndHeight,
            },
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        }
    }

    pub fn component(id: NodeId, name: impl Into<String>, width: f32, height: f32) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            transform: Transform::IDENTITY,
            width,
            height,
            style: Style::default(),
            kind: NodeKind::Component,
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        }
    }

    pub fn instance(id: NodeId, name: impl Into<String>, component_id: NodeId, width: f32, height: f32) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            transform: Transform::IDENTITY,
            width,
            height,
            style: Style::default(),
            kind: NodeKind::Instance {
                component_id,
                overrides: Vec::new(),
            },
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        }
    }

    pub fn image(id: NodeId, name: impl Into<String>, width: f32, height: f32, image_width: u32, image_height: u32, data: Vec<u8>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            transform: Transform::IDENTITY,
            width,
            height,
            style: Style::default(),
            kind: NodeKind::Image {
                data,
                image_width,
                image_height,
            },
            horizontal_sizing: SizingMode::Fixed,
            vertical_sizing: SizingMode::Fixed,
            constraint_horizontal: ConstraintType::Min,
            constraint_vertical: ConstraintType::Min,
            is_mask: false,
        }
    }

    /// Is this node a container that can have children?
    pub fn is_container(&self) -> bool {
        matches!(
            self.kind,
            NodeKind::Frame { .. }
                | NodeKind::Component
                | NodeKind::Instance { .. }
                | NodeKind::BooleanOp { .. }
        )
    }
}
