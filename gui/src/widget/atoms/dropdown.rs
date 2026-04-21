use crate::renderer::TextStyle;
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
        let tokens = theme.components.dropdown;
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
                gap: tokens.gap,
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
                            size: tokens.font_size - 1.0,
                            ..theme.text_style_label_sm()
                        },
                        layout: Default::default(),
                    },
                },
                Desc::Container {
                    id: Cow::Owned(anatomy.field()),
                    style: BoxStyle {
                        direction: Direction::Row,
                        gap: tokens.gap,
                        align_items: Align::Center,
                        padding: Edges::symmetric(tokens.padding_y, tokens.padding_x),
                        gestures: vec![Gesture::Tap],
                        hittable: Some(true),
                        ..BoxStyle::default()
                    },
                    decoration: Some(Decoration {
                        background: Some(visual.background),
                        border: Some(Border {
                            width: tokens.border_width,
                            color: visual.border.unwrap_or(theme.colors.border),
                        }),
                        radius: [tokens.radius; 4],
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
                                    size: tokens.font_size,
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
                                    size: tokens.font_size,
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
