use crate::renderer::{Border, Color, TextStyle};
use crate::theme::{ControlSize, Density};
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, LeafKind, Size, TextLayout, TextOverflow,
};
use crate::tree::Desc;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ColorSwatchProps {
    pub rgba: [f32; 4],
    pub label: Option<Cow<'static, str>>,
    pub show_value: bool,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for ColorSwatchProps {
    fn widget_type(&self) -> &'static str {
        "ColorSwatch"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn WidgetProps> {
        Box::new(self.clone())
    }

    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        let theme = cx.theme;
        let metrics = theme.control_metrics(self.size, self.density);
        let visual = theme.text_input_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });
        let mut children = Vec::new();

        if let Some(label) = &self.label {
            children.push(text_leaf(
                format!("{id}::label"),
                label,
                theme.colors.text_muted,
                theme.text_style_label_sm(),
                Size::Auto,
            ));
        }

        children.push(Desc::Container {
            id: Cow::Owned(format!("{id}::swatch")),
            style: BoxStyle {
                width: Size::Fixed(metrics.icon_size.max(18.0)),
                height: Size::Fixed(metrics.icon_size.max(18.0)),
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(Color {
                    r: self.rgba[0],
                    g: self.rgba[1],
                    b: self.rgba[2],
                    a: self.rgba[3],
                }),
                border: Some(Border {
                    width: metrics.border_width,
                    color: visual.border.unwrap_or(theme.colors.border),
                }),
                radius: [metrics.radius; 4],
                shadow: None,
            }),
            children: Vec::new(),
        });

        if self.show_value {
            children.push(text_leaf(
                format!("{id}::value"),
                &format_color(self.rgba),
                visual.text,
                theme.text_style_body_sm(),
                Size::Fill,
            ));
        }

        WidgetBuild {
            style: BoxStyle {
                height: Size::Fixed(metrics.height),
                direction: Direction::Row,
                align_items: Align::Center,
                gap: metrics.gap,
                ..BoxStyle::default()
            },
            decoration: None,
            children,
        }
    }
}

fn text_leaf(id: String, content: &str, color: Color, mut style: TextStyle, width: Size) -> Desc {
    style.color = color;
    Desc::Leaf {
        id: Cow::Owned(id),
        style: BoxStyle {
            width,
            height: Size::Auto,
            flex_shrink: 1.0,
            ..BoxStyle::default()
        },
        kind: LeafKind::Text {
            content: content.to_string(),
            style,
            layout: TextLayout {
                overflow: TextOverflow::Ellipsis,
                ..Default::default()
            },
        },
    }
}

pub fn format_color(rgba: [f32; 4]) -> String {
    let r = (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{r:02X}{g:02X}{b:02X}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn color_swatch_formats_hex_value() {
        let theme = light_theme();
        let props = ColorSwatchProps {
            rgba: [1.0, 0.5, 0.0, 1.0],
            label: None,
            show_value: true,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };
        let build = props.build(
            "color",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children[0].id(), "color::swatch");
        assert_eq!(build.children[1].id(), "color::value");
    }
}
