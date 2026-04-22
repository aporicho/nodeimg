use crate::gesture::Gesture;
use crate::renderer::{Border, TextStyle};
use crate::theme::{ControlSize, Density};
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Size, TextLayout, TextOverflow,
};
use crate::tree::Desc;
use crate::widget::anatomy::Anatomy;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct PathInputProps {
    pub label: Option<Cow<'static, str>>,
    pub value: Cow<'static, str>,
    pub extensions: Vec<Cow<'static, str>>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for PathInputProps {
    fn widget_type(&self) -> &'static str {
        "PathInput"
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
        let anatomy = Anatomy::new(id);
        let mut children = Vec::new();

        if let Some(label) = &self.label {
            children.push(Desc::Leaf {
                id: Cow::Owned(anatomy.label()),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: label.to_string(),
                    style: TextStyle {
                        color: theme.colors.text_muted,
                        size: metrics.label_font_size,
                        ..theme.text_style_label_sm()
                    },
                    layout: Default::default(),
                },
            });
        }

        let value = if self.value.is_empty() {
            extension_hint(&self.extensions)
        } else {
            self.value.to_string()
        };

        children.push(Desc::Container {
            id: Cow::Owned(anatomy.field()),
            style: BoxStyle {
                direction: Direction::Row,
                align_items: Align::Center,
                gap: metrics.gap,
                height: Size::Fixed(metrics.height),
                padding: Edges::symmetric(metrics.padding_y, metrics.padding_x),
                gestures: vec![Gesture::Tap],
                hittable: Some(true),
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
            children: vec![
                Desc::Leaf {
                    id: Cow::Owned(anatomy.part("value")),
                    style: BoxStyle {
                        width: Size::Fill,
                        height: Size::Auto,
                        flex_shrink: 1.0,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: value,
                        style: TextStyle {
                            color: visual.text,
                            size: metrics.font_size,
                            ..theme.text_style_body_sm()
                        },
                        layout: TextLayout {
                            overflow: TextOverflow::Ellipsis,
                            ..Default::default()
                        },
                    },
                },
                Desc::Leaf {
                    id: Cow::Owned(anatomy.part("button")),
                    style: BoxStyle::default(),
                    kind: LeafKind::Text {
                        content: "...".to_string(),
                        style: TextStyle {
                            color: theme.colors.text_muted,
                            size: metrics.font_size,
                            ..theme.text_style_body_sm()
                        },
                        layout: Default::default(),
                    },
                },
            ],
        });

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Column,
                gap: (metrics.gap / 2.0).max(2.0),
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children,
        }
    }
}

fn extension_hint(extensions: &[Cow<'static, str>]) -> String {
    if extensions.is_empty() {
        "select file".to_string()
    } else {
        extensions
            .iter()
            .map(|extension| format!("*.{extension}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn path_input_can_render_without_label() {
        let theme = light_theme();
        let props = PathInputProps {
            label: None,
            value: Cow::Borrowed(""),
            extensions: vec![Cow::Borrowed("png")],
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };
        let build = props.build(
            "path",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "path::field");
    }
}
