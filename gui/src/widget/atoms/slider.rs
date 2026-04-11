use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::widget::props::{WidgetBuild, WidgetProps};

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
    fn widget_type(&self) -> &'static str { "Slider" }
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn WidgetProps> { Box::new(self.clone()) }
    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |o| self == o)
    }
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
    fn build(&self, id: &str) -> WidgetBuild {
        use crate::tree::Desc;
        use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Align, LeafKind};
        use crate::renderer::Color;

        // shadcn zinc 色系
        let label_color = Color { r: 0.443, g: 0.443, b: 0.478, a: 1.0 };  // zinc-500
        let value_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };  // zinc-900
        let track_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };  // zinc-200
        let fill_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };   // zinc-900
        let font_size = 12.0;
        let track_height = 6.0;
        let track_radius = track_height / 2.0;

        // 填充比例
        let range = self.max - self.min;
        let ratio = if range > 0.0 { ((self.value - self.min) / range).clamp(0.0, 1.0) } else { 0.0 };

        // 值文本格式化
        let value_text = if self.step >= 1.0 {
            format!("{}", self.value as i32)
        } else {
            format!("{:.1}", self.value)
        };

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Row,
                gap: 8.0,
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
                        font_size,
                        color: label_color,
                    },
                },
                // 轨道
                Desc::Container {
                    id: Cow::Owned(format!("{id}::track")),
                    style: BoxStyle {
                        flex_grow: 1.0,
                        height: Size::Fixed(track_height),
                        direction: Direction::Row,
                        ..BoxStyle::default()
                    },
                    decoration: Some(Decoration {
                        background: Some(track_color),
                        border: None,
                        radius: [track_radius; 4],
                        shadow: None,
                    }),
                    children: vec![
                        // 填充条（按比例占空间）
                        Desc::Container {
                            id: Cow::Owned(format!("{id}::fill")),
                            style: BoxStyle {
                                flex_grow: ratio,
                                height: Size::Fill,
                                ..BoxStyle::default()
                            },
                            decoration: Some(Decoration {
                                background: Some(fill_color),
                                border: None,
                                radius: [track_radius; 4],
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
                        font_size,
                        color: value_color,
                    },
                },
            ],
        }
    }
}
