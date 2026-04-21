use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::widget::anatomy::Anatomy;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    pub label: Cow<'static, str>,
    pub icon: Option<Cow<'static, str>>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for ButtonProps {
    fn widget_type(&self) -> &'static str {
        "Button"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn WidgetProps> {
        Box::new(self.clone())
    }
    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map_or(false, |o| self == o)
    }
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        use crate::renderer::Border;
        use crate::tree::layout::{
            Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Size,
        };
        use crate::tree::Desc;

        let theme = cx.theme;
        let metrics = theme.control_metrics(self.size, self.density);
        let visual = theme.button_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });
        let anatomy = Anatomy::new(id);

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Row,
                align_items: Align::Center,
                justify_content: Justify::Center,
                gap: metrics.gap,
                height: Size::Fixed(metrics.height),
                padding: Edges::symmetric(metrics.padding_y, metrics.padding_x),
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(visual.background),
                border: Some(Border {
                    width: metrics.border_width,
                    color: visual.border.unwrap_or(theme.colors.border),
                }),
                radius: [metrics.radius; 4],
                shadow: None,
            }),
            children: vec![Desc::Leaf {
                id: Cow::Owned(anatomy.label()),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: self.label.to_string(),
                    style: TextStyle {
                        color: visual.text,
                        size: metrics.font_size,
                        ..theme.text_style_body_sm()
                    },
                    layout: Default::default(),
                },
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, Size};

    #[test]
    fn button_uses_control_metrics_for_default_size() {
        let theme = dark_theme();
        let props = ButtonProps {
            label: Cow::Borrowed("Run"),
            icon: None,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        };

        let build = props.build(
            "button",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.height, Size::Fixed(36.0));
        assert_eq!(build.style.padding, Edges::symmetric(8.0, 12.0));
    }

    #[test]
    fn button_can_use_compact_small_metrics() {
        let theme = dark_theme();
        let props = ButtonProps {
            label: Cow::Borrowed("Run"),
            icon: None,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "button",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.height, Size::Fixed(24.0));
        assert_eq!(build.style.padding, Edges::symmetric(4.0, 6.0));
    }
}
