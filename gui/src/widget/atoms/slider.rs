use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    pub label: Cow<'static, str>,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: f32,
    pub disabled: bool,
}

impl WidgetProps for SliderProps {
    fn widget_type(&self) -> &'static str {
        "Slider"
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
        use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, LeafKind, Size};
        use crate::tree::Desc;

        let theme = cx.theme;
        let tokens = theme.components.slider;
        let visual = theme.slider_visual(if self.disabled {
            crate::widget::state::WidgetVisualState::Disabled
        } else {
            crate::widget::state::WidgetVisualState::Normal
        });

        // 填充比例
        let range = self.max - self.min;
        let ratio = if range > 0.0 {
            ((self.value - self.min) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // 值文本格式化
        let value_text = if self.step >= 1.0 {
            format!("{}", self.value as i32)
        } else {
            format!("{:.1}", self.value)
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
                            color: theme.colors.text_muted,
                            size: tokens.font_size,
                            ..theme.text_style_body_sm()
                        },
                    },
                },
                // 轨道
                Desc::Container {
                    id: Cow::Owned(format!("{id}::track")),
                    style: BoxStyle {
                        flex_grow: 1.0,
                        height: Size::Fixed(tokens.track_height),
                        padding: crate::tree::layout::Edges::all(tokens.track_padding),
                        direction: Direction::Row,
                        gestures: vec![Gesture::Tap, Gesture::Drag],
                        align_items: Align::Center,
                        ..BoxStyle::default()
                    },
                    decoration: Some(Decoration {
                        background: Some(visual.track_background),
                        border: None,
                        radius: [tokens.track_radius; 4],
                        shadow: None,
                    }),
                    children: vec![
                        // 填充条（按比例占空间）
                        Desc::Container {
                            id: Cow::Owned(format!("{id}::fill")),
                            style: BoxStyle {
                                flex_grow: ratio,
                                height: Size::Fill,
                                gestures: vec![Gesture::Tap, Gesture::Drag],
                                ..BoxStyle::default()
                            },
                            decoration: Some(Decoration {
                                background: Some(visual.fill),
                                border: None,
                                radius: [tokens.track_radius; 4],
                                shadow: None,
                            }),
                            children: vec![],
                        },
                        // Thumb（明确可拖拽的圆点）
                        Desc::Container {
                            id: Cow::Owned(format!("{id}::thumb")),
                            style: BoxStyle {
                                width: Size::Fixed(tokens.thumb_size),
                                height: Size::Fixed(tokens.thumb_size),
                                gestures: vec![Gesture::Tap, Gesture::Drag],
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
                        // 空白（剩余空间）
                        Desc::Container {
                            id: Cow::Owned(format!("{id}::spacer")),
                            style: BoxStyle {
                                flex_grow: 1.0 - ratio,
                                height: Size::Fill,
                                gestures: vec![Gesture::Tap, Gesture::Drag],
                                ..BoxStyle::default()
                            },
                            decoration: None,
                            children: vec![],
                        },
                    ],
                },
                // 值显示
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::value")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: value_text,
                        style: TextStyle {
                            color: visual.text,
                            size: tokens.font_size,
                            ..theme.text_style_mono_md()
                        },
                    },
                },
            ],
        }
    }
}
