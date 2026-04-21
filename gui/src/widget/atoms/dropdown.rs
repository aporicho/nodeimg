use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::widget::anatomy::Anatomy;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct DropdownProps {
    pub label: Cow<'static, str>,
    pub options: Vec<Cow<'static, str>>,
    pub selected: usize,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for DropdownProps {
    fn widget_type(&self) -> &'static str {
        "Dropdown"
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
        use crate::gesture::Gesture;
        use crate::renderer::Border;
        use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Size};
        use crate::tree::Desc;

        let theme = cx.theme;
        let metrics = theme.control_metrics(self.size, self.density);
        let visual = theme.dropdown_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });

        let selected_text = self
            .options
            .get(self.selected)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let anatomy = Anatomy::new(id);

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Column,
                gap: (metrics.gap / 2.0).max(2.0),
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![
                Desc::Leaf {
                    id: Cow::Owned(anatomy.label()),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: self.label.to_string(),
                        style: TextStyle {
                            color: theme.colors.text_muted,
                            size: metrics.label_font_size,
                            ..theme.text_style_label_sm()
                        },
                        layout: Default::default(),
                    },
                },
                Desc::Container {
                    id: Cow::Owned(anatomy.field()),
                    style: BoxStyle {
                        direction: Direction::Row,
                        gap: metrics.gap,
                        align_items: Align::Center,
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
                            id: Cow::Owned(anatomy.part("selected")),
                            style: BoxStyle {
                                width: Size::Auto,
                                height: Size::Auto,
                                ..BoxStyle::default()
                            },
                            kind: LeafKind::Text {
                                content: selected_text,
                                style: TextStyle {
                                    color: visual.text,
                                    size: metrics.font_size,
                                    ..theme.text_style_body_sm()
                                },
                                layout: Default::default(),
                            },
                        },
                        Desc::Container {
                            id: Cow::Owned(anatomy.part("spacer")),
                            style: BoxStyle {
                                flex_grow: 1.0,
                                ..BoxStyle::default()
                            },
                            decoration: None,
                            children: vec![],
                        },
                        Desc::Leaf {
                            id: Cow::Owned(anatomy.part("arrow")),
                            style: BoxStyle {
                                width: Size::Auto,
                                height: Size::Auto,
                                ..BoxStyle::default()
                            },
                            kind: LeafKind::Text {
                                content: "▾".to_string(),
                                style: TextStyle {
                                    color: theme.colors.text_muted,
                                    size: metrics.font_size,
                                    ..theme.text_style_body_sm()
                                },
                                layout: Default::default(),
                            },
                        },
                    ],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, Size};
    use crate::tree::Desc;

    #[test]
    fn dropdown_field_uses_control_metrics() {
        let theme = dark_theme();
        let props = DropdownProps {
            label: Cow::Borrowed("Mode"),
            options: vec![Cow::Borrowed("Normal")],
            selected: 0,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "dropdown",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        match &build.children[1] {
            Desc::Container { style, .. } => {
                assert_eq!(style.height, Size::Fixed(24.0));
                assert_eq!(style.padding, Edges::symmetric(4.0, 6.0));
            }
            _ => panic!("expected dropdown field container"),
        }
    }
}
