use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleProps {
    pub label: Cow<'static, str>,
    pub value: bool,
    pub disabled: bool,
}

impl WidgetProps for ToggleProps {
    fn widget_type(&self) -> &'static str {
        "Toggle"
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
        use crate::tree::layout::{
            Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Size,
        };
        use crate::tree::Desc;

        let theme = cx.theme;
        let tokens = theme.components.toggle;
        let visual = theme.toggle_visual(
            self.value,
            if self.disabled {
                crate::widget::state::WidgetVisualState::Disabled
            } else {
                crate::widget::state::WidgetVisualState::Normal
            },
        );
        let thumb_justify = if self.value {
            Justify::End
        } else {
            Justify::Start
        };

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Row,
                gap: tokens.gap,
                align_items: Align::Center,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![
                // 开关轨道 (32 x 18)
                Desc::Container {
                    id: Cow::Owned(format!("{id}::track")),
                    style: BoxStyle {
                        width: Size::Fixed(tokens.track_width),
                        height: Size::Fixed(tokens.track_height),
                        padding: Edges::all(tokens.track_padding),
                        direction: Direction::Row,
                        justify_content: thumb_justify,
                        align_items: Align::Center,
                        gestures: vec![Gesture::Tap],
                        ..BoxStyle::default()
                    },
                    decoration: Some(Decoration {
                        background: Some(visual.track_background),
                        border: None,
                        radius: [tokens.track_radius; 4],
                        shadow: None,
                    }),
                    children: vec![
                        // 滑块圆点 (14 x 14)
                        Desc::Container {
                            id: Cow::Owned(format!("{id}::thumb")),
                            style: BoxStyle {
                                width: Size::Fixed(tokens.thumb_size),
                                height: Size::Fixed(tokens.thumb_size),
                                ..BoxStyle::default()
                            },
                            decoration: Some(Decoration {
                                background: Some(visual.thumb),
                                border: None,
                                radius: [tokens.thumb_size / 2.0; 4],
                                shadow: None,
                            }),
                            children: vec![],
                        },
                    ],
                },
                // 标签
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::label")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: self.label.to_string(),
                        style: TextStyle {
                            color: visual.text,
                            size: tokens.font_size,
                            ..theme.text_style_body_sm()
                        },
                    },
                },
            ],
        }
    }
}
