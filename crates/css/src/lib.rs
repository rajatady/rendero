//! CSS parsing via Lightning CSS — full spec compliance.
//!
//! Implements `CssParser` trait from rendero-core.
//! Maps lightningcss typed AST → engine-consumable `CssValue`.

use lightningcss::declaration::DeclarationBlock;
use lightningcss::properties::Property;
use lightningcss::properties::PropertyId;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::traits::ToCss;
use lightningcss::values::color::CssColor;
use lightningcss::values::length::LengthValue;
use lightningcss::values::percentage::DimensionPercentage;

use rendero_core::properties::{LayoutAlign, LayoutDirection};
use rendero_core::providers::{
    CssDisplay, CssOverflow, CssParser, CssPosition, CssValue,
};

pub struct LightningCssParser;

impl CssParser for LightningCssParser {
    fn parse_property(&self, property: &str, value: &str, viewport: (f32, f32)) -> CssValue {
        let kebab = camel_to_kebab(property);
        parse_single(&kebab, value, viewport)
    }

    fn parse_declarations(&self, css: &str, viewport: (f32, f32)) -> Vec<(String, CssValue)> {
        let opts = ParserOptions::default();
        let block = match DeclarationBlock::parse_string(css, opts) {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };

        let mut result = Vec::new();
        for prop in block.declarations.iter() {
            let name = prop_name(prop);
            let value = map_property(prop, viewport);
            if !matches!(value, CssValue::None) {
                result.push((name, value));
            }
        }
        for prop in block.important_declarations.iter() {
            let name = prop_name(prop);
            let value = map_property(prop, viewport);
            if !matches!(value, CssValue::None) {
                result.push((name, value));
            }
        }
        result
    }
}

/// Parse a single CSS property (lifetime-safe wrapper).
fn parse_single(property: &str, value: &str, viewport: (f32, f32)) -> CssValue {
    let prop_id = PropertyId::from(property);
    let opts = ParserOptions::default();
    let result = Property::parse_string(prop_id, value, opts);
    match result {
        Ok(ref prop) => map_property(prop, viewport),
        Err(_) => CssValue::None,
    }
}

fn prop_name(prop: &Property) -> String {
    prop.property_id().name().to_string()
}

fn map_property(prop: &Property, viewport: (f32, f32)) -> CssValue {
    match prop {
        // ─── Colors ───
        Property::Color(color) => css_color_to_value(color),
        Property::BackgroundColor(color) => css_color_to_value(color),

        // ─── Dimensions ───
        Property::Width(size) => resolve_size(size, viewport),
        Property::Height(size) => resolve_size(size, viewport),
        Property::MinWidth(size) => resolve_size(size, viewport),
        Property::MinHeight(size) => resolve_size(size, viewport),
        Property::MaxWidth(size) => resolve_max_size(size, viewport),
        Property::MaxHeight(size) => resolve_max_size(size, viewport),

        // ─── Padding (individual) ───
        Property::PaddingTop(lpa) => resolve_lpa(lpa, viewport),
        Property::PaddingRight(lpa) => resolve_lpa(lpa, viewport),
        Property::PaddingBottom(lpa) => resolve_lpa(lpa, viewport),
        Property::PaddingLeft(lpa) => resolve_lpa(lpa, viewport),
        Property::Padding(rect) => CssValue::Padding {
            top: resolve_lpa_px(&rect.top, viewport),
            right: resolve_lpa_px(&rect.right, viewport),
            bottom: resolve_lpa_px(&rect.bottom, viewport),
            left: resolve_lpa_px(&rect.left, viewport),
        },

        // ─── Margin (individual) ───
        Property::MarginTop(lpa) => resolve_lpa(lpa, viewport),
        Property::MarginRight(lpa) => resolve_lpa(lpa, viewport),
        Property::MarginBottom(lpa) => resolve_lpa(lpa, viewport),
        Property::MarginLeft(lpa) => resolve_lpa(lpa, viewport),
        Property::Margin(rect) => CssValue::Margin {
            top: resolve_lpa_px(&rect.top, viewport),
            right: resolve_lpa_px(&rect.right, viewport),
            bottom: resolve_lpa_px(&rect.bottom, viewport),
            left: resolve_lpa_px(&rect.left, viewport),
        },

        // ─── Display ───
        Property::Display(display) => {
            use lightningcss::properties::display::{Display, DisplayInside, DisplayKeyword};
            match display {
                Display::Keyword(kw) => match kw {
                    DisplayKeyword::None => CssValue::Display(CssDisplay::None),
                    _ => CssValue::Display(CssDisplay::Block),
                },
                Display::Pair(pair) => match &pair.inside {
                    DisplayInside::Flex(_) => CssValue::Display(CssDisplay::Flex),
                    _ => CssValue::Display(CssDisplay::Block),
                },
            }
        }

        // ─── Position ───
        Property::Position(pos) => {
            use lightningcss::properties::position::Position;
            match pos {
                Position::Static => CssValue::Position(CssPosition::Static),
                Position::Relative => CssValue::Position(CssPosition::Relative),
                Position::Absolute => CssValue::Position(CssPosition::Absolute),
                Position::Fixed => CssValue::Position(CssPosition::Fixed),
                Position::Sticky(_) => CssValue::Position(CssPosition::Sticky),
            }
        }

        // ─── Flexbox container ───
        Property::FlexDirection(dir, _vp) => {
            use lightningcss::properties::flex::FlexDirection;
            match dir {
                FlexDirection::Row | FlexDirection::RowReverse => {
                    CssValue::FlexDirection(LayoutDirection::Horizontal)
                }
                FlexDirection::Column | FlexDirection::ColumnReverse => {
                    CssValue::FlexDirection(LayoutDirection::Vertical)
                }
            }
        }

        Property::FlexWrap(wrap, _vp) => {
            use lightningcss::properties::flex::FlexWrap;
            match wrap {
                FlexWrap::Wrap | FlexWrap::WrapReverse => CssValue::FlexWrap(true),
                FlexWrap::NoWrap => CssValue::FlexWrap(false),
            }
        }

        Property::AlignItems(align, _vp) => {
            use lightningcss::properties::align::{AlignItems, SelfPosition};
            let la = match align {
                AlignItems::Normal => LayoutAlign::Start,
                AlignItems::Stretch => LayoutAlign::Stretch,
                AlignItems::SelfPosition { value, .. } => match value {
                    SelfPosition::Center => LayoutAlign::Center,
                    SelfPosition::FlexStart | SelfPosition::Start | SelfPosition::SelfStart => {
                        LayoutAlign::Start
                    }
                    SelfPosition::FlexEnd | SelfPosition::End | SelfPosition::SelfEnd => {
                        LayoutAlign::End
                    }
                },
                AlignItems::BaselinePosition(_) => LayoutAlign::Start,
            };
            CssValue::AlignItems(la)
        }

        Property::JustifyContent(justify, _vp) => {
            use lightningcss::properties::align::{
                ContentDistribution, ContentPosition, JustifyContent,
            };
            let la = match justify {
                JustifyContent::Normal => LayoutAlign::Start,
                JustifyContent::ContentDistribution(dist) => match dist {
                    ContentDistribution::SpaceBetween
                    | ContentDistribution::SpaceAround
                    | ContentDistribution::SpaceEvenly => LayoutAlign::Center, // approximate
                    ContentDistribution::Stretch => LayoutAlign::Stretch,
                },
                JustifyContent::ContentPosition { value, .. } => match value {
                    ContentPosition::Center => LayoutAlign::Center,
                    ContentPosition::Start | ContentPosition::FlexStart => LayoutAlign::Start,
                    ContentPosition::End | ContentPosition::FlexEnd => LayoutAlign::End,
                },
                JustifyContent::Left { .. } => LayoutAlign::Start,
                JustifyContent::Right { .. } => LayoutAlign::End,
            };
            CssValue::JustifyContent(la)
        }

        // ─── Flex item ───
        Property::FlexGrow(n, _vp) => CssValue::Number(*n),
        Property::FlexShrink(n, _vp) => CssValue::Number(*n),
        Property::FlexBasis(lpa, _vp) => resolve_lpa(lpa, viewport),

        // ─── Gap ───
        Property::Gap(gap) => {
            use lightningcss::properties::align::GapValue;
            let px = match &gap.row {
                GapValue::LengthPercentage(lp) => resolve_lp_px(lp, viewport),
                GapValue::Normal => 0.0,
            };
            CssValue::Length(px)
        }
        Property::RowGap(gap) => {
            use lightningcss::properties::align::GapValue;
            match gap {
                GapValue::LengthPercentage(lp) => CssValue::Length(resolve_lp_px(lp, viewport)),
                GapValue::Normal => CssValue::Length(0.0),
            }
        }
        Property::ColumnGap(gap) => {
            use lightningcss::properties::align::GapValue;
            match gap {
                GapValue::LengthPercentage(lp) => CssValue::Length(resolve_lp_px(lp, viewport)),
                GapValue::Normal => CssValue::Length(0.0),
            }
        }

        // ─── Border radius ───
        Property::BorderRadius(radii, _vp) => {
            let tl = resolve_lp_px(&radii.top_left.0, viewport);
            let tr = resolve_lp_px(&radii.top_right.0, viewport);
            let br = resolve_lp_px(&radii.bottom_right.0, viewport);
            let bl = resolve_lp_px(&radii.bottom_left.0, viewport);
            CssValue::BorderRadius { tl, tr, br, bl }
        }

        // ─── Opacity ───
        Property::Opacity(alpha) => CssValue::Number(alpha.0),

        // ─── Overflow ───
        Property::Overflow(overflow) => {
            use lightningcss::properties::overflow::OverflowKeyword;
            match overflow.x {
                OverflowKeyword::Hidden => CssValue::Overflow(CssOverflow::Hidden),
                OverflowKeyword::Scroll => CssValue::Overflow(CssOverflow::Scroll),
                OverflowKeyword::Auto => CssValue::Overflow(CssOverflow::Auto),
                _ => CssValue::Overflow(CssOverflow::Visible),
            }
        }

        // ─── Position offsets ───
        Property::Top(lpa) => resolve_lpa(lpa, viewport),
        Property::Right(lpa) => resolve_lpa(lpa, viewport),
        Property::Bottom(lpa) => resolve_lpa(lpa, viewport),
        Property::Left(lpa) => resolve_lpa(lpa, viewport),

        // ─── Font ───
        Property::FontSize(fs) => resolve_font_size(fs, viewport),
        Property::FontWeight(fw) => {
            use lightningcss::properties::font::{AbsoluteFontWeight, FontWeight};
            match fw {
                FontWeight::Absolute(abs) => match abs {
                    AbsoluteFontWeight::Weight(n) => CssValue::Number(*n),
                    AbsoluteFontWeight::Normal => CssValue::Number(400.0),
                    AbsoluteFontWeight::Bold => CssValue::Number(700.0),
                },
                FontWeight::Bolder => CssValue::Number(700.0),
                FontWeight::Lighter => CssValue::Number(300.0),
            }
        }
        Property::FontFamily(families) => {
            use lightningcss::properties::font::FontFamily;
            if let Some(first) = families.first() {
                let name = match first {
                    FontFamily::FamilyName(n) => {
                        n.to_css_string(lightningcss::printer::PrinterOptions::default())
                            .unwrap_or_default()
                            .trim_matches('"')
                            .to_string()
                    }
                    FontFamily::Generic(g) => format!("{:?}", g),
                };
                CssValue::Keyword(name)
            } else {
                CssValue::None
            }
        }

        // ─── Text ───
        Property::TextAlign(align) => {
            use lightningcss::properties::text::TextAlign;
            let kw = match align {
                TextAlign::Left => "left",
                TextAlign::Right => "right",
                TextAlign::Center => "center",
                TextAlign::Justify => "justify",
                _ => "left",
            };
            CssValue::Keyword(kw.to_string())
        }

        // ─── Z-Index ───
        Property::ZIndex(zi) => {
            use lightningcss::properties::position::ZIndex;
            match zi {
                ZIndex::Integer(n) => CssValue::Number(*n as f32),
                ZIndex::Auto => CssValue::None,
            }
        }

        // ─── Border width/color ───
        Property::BorderTopWidth(bw) => resolve_border_width(bw),
        Property::BorderRightWidth(bw) => resolve_border_width(bw),
        Property::BorderBottomWidth(bw) => resolve_border_width(bw),
        Property::BorderLeftWidth(bw) => resolve_border_width(bw),
        Property::BorderTopColor(color) => css_color_to_value(color),
        Property::BorderRightColor(color) => css_color_to_value(color),
        Property::BorderBottomColor(color) => css_color_to_value(color),
        Property::BorderLeftColor(color) => css_color_to_value(color),

        _ => CssValue::None,
    }
}

// ─── Helpers ───

fn css_color_to_value(color: &CssColor) -> CssValue {
    match color.to_rgb() {
        Ok(CssColor::RGBA(rgba)) => CssValue::Color {
            r: rgba.red_f32(),
            g: rgba.green_f32(),
            b: rgba.blue_f32(),
            a: rgba.alpha_f32(),
        },
        _ => CssValue::None,
    }
}

fn resolve_length_value(lv: &LengthValue, viewport: (f32, f32)) -> f32 {
    match lv {
        LengthValue::Px(v) => *v,
        LengthValue::Rem(v) => v * 16.0,
        LengthValue::Em(v) => v * 16.0,
        LengthValue::Vh(v) => v * viewport.1 / 100.0,
        LengthValue::Vw(v) => v * viewport.0 / 100.0,
        LengthValue::Pt(v) => v * 4.0 / 3.0,
        LengthValue::In(v) => v * 96.0,
        LengthValue::Cm(v) => v * 96.0 / 2.54,
        LengthValue::Mm(v) => v * 96.0 / 25.4,
        _ => lv.to_px().unwrap_or(0.0),
    }
}

fn resolve_lp_px(
    lp: &DimensionPercentage<LengthValue>,
    viewport: (f32, f32),
) -> f32 {
    match lp {
        DimensionPercentage::Dimension(lv) => resolve_length_value(lv, viewport),
        DimensionPercentage::Percentage(p) => p.0 * viewport.0,
        DimensionPercentage::Calc(_) => 0.0,
    }
}

fn resolve_lp(
    lp: &DimensionPercentage<LengthValue>,
    viewport: (f32, f32),
) -> CssValue {
    match lp {
        DimensionPercentage::Dimension(lv) => CssValue::Length(resolve_length_value(lv, viewport)),
        DimensionPercentage::Percentage(p) => CssValue::Percentage(p.0),
        DimensionPercentage::Calc(_) => CssValue::Length(0.0),
    }
}

fn resolve_lpa(
    lpa: &lightningcss::values::length::LengthPercentageOrAuto,
    viewport: (f32, f32),
) -> CssValue {
    use lightningcss::values::length::LengthPercentageOrAuto;
    match lpa {
        LengthPercentageOrAuto::LengthPercentage(lp) => resolve_lp(lp, viewport),
        LengthPercentageOrAuto::Auto => CssValue::None,
    }
}

fn resolve_lpa_px(
    lpa: &lightningcss::values::length::LengthPercentageOrAuto,
    viewport: (f32, f32),
) -> f32 {
    use lightningcss::values::length::LengthPercentageOrAuto;
    match lpa {
        LengthPercentageOrAuto::LengthPercentage(lp) => resolve_lp_px(lp, viewport),
        LengthPercentageOrAuto::Auto => 0.0,
    }
}

fn resolve_size(
    size: &lightningcss::properties::size::Size,
    viewport: (f32, f32),
) -> CssValue {
    use lightningcss::properties::size::Size;
    match size {
        Size::LengthPercentage(lp) => resolve_lp(lp, viewport),
        Size::Auto => CssValue::None,
        _ => CssValue::None,
    }
}

fn resolve_max_size(
    size: &lightningcss::properties::size::MaxSize,
    viewport: (f32, f32),
) -> CssValue {
    use lightningcss::properties::size::MaxSize;
    match size {
        MaxSize::LengthPercentage(lp) => resolve_lp(lp, viewport),
        MaxSize::None => CssValue::None,
        _ => CssValue::None,
    }
}

fn resolve_font_size(
    fs: &lightningcss::properties::font::FontSize,
    viewport: (f32, f32),
) -> CssValue {
    use lightningcss::properties::font::{AbsoluteFontSize, FontSize, RelativeFontSize};
    match fs {
        FontSize::Length(lp) => resolve_lp(lp, viewport),
        FontSize::Absolute(abs) => {
            let px = match abs {
                AbsoluteFontSize::XXSmall => 10.0,
                AbsoluteFontSize::XSmall => 12.0,
                AbsoluteFontSize::Small => 13.0,
                AbsoluteFontSize::Medium => 16.0,
                AbsoluteFontSize::Large => 18.0,
                AbsoluteFontSize::XLarge => 24.0,
                AbsoluteFontSize::XXLarge => 32.0,
                _ => 16.0,
            };
            CssValue::Length(px)
        }
        FontSize::Relative(rel) => match rel {
            RelativeFontSize::Smaller => CssValue::Length(13.0),
            RelativeFontSize::Larger => CssValue::Length(19.0),
        },
    }
}

fn resolve_border_width(bw: &lightningcss::properties::border::BorderSideWidth) -> CssValue {
    use lightningcss::properties::border::BorderSideWidth;
    match bw {
        BorderSideWidth::Length(len) => {
            use lightningcss::values::length::Length;
            match len {
                Length::Value(lv) => CssValue::Length(resolve_length_value(lv, (0.0, 0.0))),
                Length::Calc(_) => CssValue::Length(0.0),
            }
        }
        BorderSideWidth::Thin => CssValue::Length(1.0),
        BorderSideWidth::Medium => CssValue::Length(3.0),
        BorderSideWidth::Thick => CssValue::Length(5.0),
    }
}

fn camel_to_kebab(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> LightningCssParser {
        LightningCssParser
    }

    fn vp() -> (f32, f32) {
        (1280.0, 800.0)
    }

    #[test]
    fn test_parse_color_hex() {
        let v = parser().parse_property("color", "#ff0000", vp());
        match v {
            CssValue::Color { r, g, b, a } => {
                assert!((r - 1.0).abs() < 0.01);
                assert!(g.abs() < 0.01);
                assert!(b.abs() < 0.01);
                assert!((a - 1.0).abs() < 0.01);
            }
            _ => panic!("expected Color, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_color_rgba() {
        let v = parser().parse_property("color", "rgba(0, 128, 255, 0.5)", vp());
        match v {
            CssValue::Color { r, g, b, a } => {
                assert!(r.abs() < 0.01);
                assert!((g - 0.502).abs() < 0.01);
                assert!((b - 1.0).abs() < 0.01);
                assert!((a - 0.5).abs() < 0.01);
            }
            _ => panic!("expected Color, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_color_named() {
        let v = parser().parse_property("background-color", "red", vp());
        match v {
            CssValue::Color { r, g, b, .. } => {
                assert!((r - 1.0).abs() < 0.01);
                assert!(g.abs() < 0.01);
                assert!(b.abs() < 0.01);
            }
            _ => panic!("expected Color, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_length_px() {
        let v = parser().parse_property("width", "100px", vp());
        match v {
            CssValue::Length(px) => assert!((px - 100.0).abs() < 0.01),
            _ => panic!("expected Length, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_length_rem() {
        let v = parser().parse_property("width", "2rem", vp());
        match v {
            CssValue::Length(px) => assert!((px - 32.0).abs() < 0.01),
            _ => panic!("expected Length, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_length_vh() {
        let v = parser().parse_property("height", "100vh", vp());
        match v {
            CssValue::Length(px) => assert!((px - 800.0).abs() < 0.01),
            _ => panic!("expected Length, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_percentage() {
        let v = parser().parse_property("width", "50%", vp());
        match v {
            CssValue::Percentage(p) => assert!((p - 0.5).abs() < 0.01),
            _ => panic!("expected Percentage, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_display_flex() {
        let v = parser().parse_property("display", "flex", vp());
        match v {
            CssValue::Display(CssDisplay::Flex) => {}
            _ => panic!("expected Display(Flex), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_display_none() {
        let v = parser().parse_property("display", "none", vp());
        match v {
            CssValue::Display(CssDisplay::None) => {}
            _ => panic!("expected Display(None), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_flex_direction() {
        let v = parser().parse_property("flex-direction", "row", vp());
        match v {
            CssValue::FlexDirection(LayoutDirection::Horizontal) => {}
            _ => panic!("expected FlexDirection(Horizontal), got {:?}", v),
        }

        let v = parser().parse_property("flex-direction", "column", vp());
        match v {
            CssValue::FlexDirection(LayoutDirection::Vertical) => {}
            _ => panic!("expected FlexDirection(Vertical), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_flex_wrap() {
        let v = parser().parse_property("flex-wrap", "wrap", vp());
        match v {
            CssValue::FlexWrap(true) => {}
            _ => panic!("expected FlexWrap(true), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_flex_grow() {
        let v = parser().parse_property("flex-grow", "1", vp());
        match v {
            CssValue::Number(n) => assert!((n - 1.0).abs() < 0.01),
            _ => panic!("expected Number(1.0), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_opacity() {
        let v = parser().parse_property("opacity", "0.5", vp());
        match v {
            CssValue::Number(n) => assert!((n - 0.5).abs() < 0.01),
            _ => panic!("expected Number(0.5), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_font_size() {
        let v = parser().parse_property("font-size", "24px", vp());
        match v {
            CssValue::Length(px) => assert!((px - 24.0).abs() < 0.01),
            _ => panic!("expected Length(24), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_font_weight_bold() {
        let v = parser().parse_property("font-weight", "bold", vp());
        match v {
            CssValue::Number(n) => assert!((n - 700.0).abs() < 0.01),
            _ => panic!("expected Number(700), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_font_weight_numeric() {
        let v = parser().parse_property("font-weight", "600", vp());
        match v {
            CssValue::Number(n) => assert!((n - 600.0).abs() < 0.01),
            _ => panic!("expected Number(600), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_padding_shorthand() {
        let v = parser().parse_property("padding", "10px 20px 30px 40px", vp());
        match v {
            CssValue::Padding {
                top,
                right,
                bottom,
                left,
            } => {
                assert!((top - 10.0).abs() < 0.01);
                assert!((right - 20.0).abs() < 0.01);
                assert!((bottom - 30.0).abs() < 0.01);
                assert!((left - 40.0).abs() < 0.01);
            }
            _ => panic!("expected Padding, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_gap() {
        let v = parser().parse_property("gap", "10px", vp());
        match v {
            CssValue::Length(px) => assert!((px - 10.0).abs() < 0.01),
            _ => panic!("expected Length(10), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_border_radius() {
        let v = parser().parse_property("border-radius", "8px", vp());
        match v {
            CssValue::BorderRadius { tl, tr, br, bl } => {
                assert!((tl - 8.0).abs() < 0.01);
                assert!((tr - 8.0).abs() < 0.01);
                assert!((br - 8.0).abs() < 0.01);
                assert!((bl - 8.0).abs() < 0.01);
            }
            _ => panic!("expected BorderRadius, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_declarations_block() {
        let css = "display: flex; flex-direction: column; gap: 10px; background-color: #ff0000;";
        let results = parser().parse_declarations(css, vp());
        assert!(
            results.len() >= 4,
            "expected at least 4 properties, got {}",
            results.len()
        );

        let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"display"));
        assert!(names.contains(&"flex-direction"));
        assert!(names.contains(&"gap"));
        assert!(names.contains(&"background-color"));
    }

    #[test]
    fn test_camel_to_kebab() {
        assert_eq!(camel_to_kebab("backgroundColor"), "background-color");
        assert_eq!(camel_to_kebab("flexDirection"), "flex-direction");
        assert_eq!(
            camel_to_kebab("borderTopLeftRadius"),
            "border-top-left-radius"
        );
        assert_eq!(camel_to_kebab("width"), "width");
    }

    #[test]
    fn test_parse_camelcase_property() {
        let v = parser().parse_property("backgroundColor", "#00ff00", vp());
        match v {
            CssValue::Color { r, g, b, .. } => {
                assert!(r.abs() < 0.01);
                assert!((g - 1.0).abs() < 0.02);
                assert!(b.abs() < 0.01);
            }
            _ => panic!("expected Color, got {:?}", v),
        }
    }

    #[test]
    fn test_parse_overflow_hidden() {
        let v = parser().parse_property("overflow", "hidden", vp());
        match v {
            CssValue::Overflow(CssOverflow::Hidden) => {}
            _ => panic!("expected Overflow(Hidden), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_min_height() {
        let v = parser().parse_property("min-height", "100vh", vp());
        match v {
            CssValue::Length(px) => assert!((px - 800.0).abs() < 0.01),
            _ => panic!("expected Length(800), got {:?}", v),
        }
    }

    #[test]
    fn test_parse_position_absolute() {
        let v = parser().parse_property("position", "absolute", vp());
        match v {
            CssValue::Position(CssPosition::Absolute) => {}
            _ => panic!("expected Position(Absolute), got {:?}", v),
        }
    }
}
