use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::widget::props::{WidgetBuild, WidgetProps};

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleProps {
    pub label: Cow<'static, str>,
    pub value: bool,
    pub disabled: bool,
}

impl WidgetProps for ToggleProps {
    fn widget_type(&self) -> &'static str { "Toggle" }
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn WidgetProps> { Box::new(self.clone()) }
    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |o| self == o)
    }
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
    fn build(&self, id: &str) -> WidgetBuild {
        use crate::widget::desc::Desc;
        use crate::widget::layout::{BoxStyle, Decoration, Size, Direction, Align, Justify, Edges, LeafKind};
        use crate::renderer::Color;

        // shadcn 色系
        let white = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let off_bg = Color { r: 0.831, g: 0.831, b: 0.847, a: 1.0 };  // zinc-300
        let on_bg = Color { r: 0.231, g: 0.510, b: 0.965, a: 1.0 };   // blue-500
        let label_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };  // zinc-900

        let track_bg = if self.value { on_bg } else { off_bg };
        let thumb_justify = if self.value { Justify::End } else { Justify::Start };

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
                // 开关轨道 (32 x 18)
                Desc::Container {
                    id: Cow::Owned(format!("{id}::track")),
                    style: BoxStyle {
                        width: Size::Fixed(32.0),
                        height: Size::Fixed(18.0),
                        padding: Edges::all(2.0),
                        direction: Direction::Row,
                        justify_content: thumb_justify,
                        align_items: Align::Center,
                        ..BoxStyle::default()
                    },
                    decoration: Some(Decoration {
                        background: Some(track_bg),
                        border: None,
                        radius: [9.0; 4],
                        shadow: None,
                    }),
                    children: vec![
                        // 滑块圆点 (14 x 14)
                        Desc::Container {
                            id: Cow::Owned(format!("{id}::thumb")),
                            style: BoxStyle {
                                width: Size::Fixed(14.0),
                                height: Size::Fixed(14.0),
                                ..BoxStyle::default()
                            },
                            decoration: Some(Decoration {
                                background: Some(white),
                                border: None,
                                radius: [7.0; 4],
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
                        font_size: 12.0,
                        color: label_color,
                    },
                },
            ],
        }
    }
}
