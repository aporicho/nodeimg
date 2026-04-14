use crate::widget::props::{WidgetBuild, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

pub const TEXT_INPUT_LABEL_FONT_SIZE: f32 = 11.0;
pub const TEXT_INPUT_VALUE_FONT_SIZE: f32 = 12.0;
pub const TEXT_INPUT_FIELD_HEIGHT: f32 = 36.0;
pub const TEXT_INPUT_FIELD_PADDING_X: f32 = 12.0;
pub const TEXT_INPUT_FIELD_PADDING_Y: f32 = 8.0;
pub const TEXT_INPUT_FIELD_RADIUS: f32 = 4.0;

#[derive(Clone, Debug, PartialEq)]
pub struct TextInputProps {
    pub label: Cow<'static, str>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
}

impl WidgetProps for TextInputProps {
    fn widget_type(&self) -> &'static str {
        "TextInput"
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
    fn build(&self, id: &str) -> WidgetBuild {
        use crate::renderer::{Border, Color};
        use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Size};
        use crate::tree::Desc;

        let border_color = Color {
            r: 0.894,
            g: 0.894,
            b: 0.906,
            a: 1.0,
        }; // zinc-200
        let label_color = Color {
            r: 0.443,
            g: 0.443,
            b: 0.478,
            a: 1.0,
        }; // zinc-500
        let value_color = Color {
            r: 0.094,
            g: 0.094,
            b: 0.106,
            a: 1.0,
        }; // zinc-900

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Column,
                gap: 4.0,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![
                // label
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::label")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: self.label.to_string(),
                        font_size: TEXT_INPUT_LABEL_FONT_SIZE,
                        color: label_color,
                    },
                },
                // field
                Desc::Container {
                    id: Cow::Owned(format!("{id}::field")),
                    style: BoxStyle {
                        height: Size::Fixed(TEXT_INPUT_FIELD_HEIGHT),
                        padding: Edges::symmetric(
                            TEXT_INPUT_FIELD_PADDING_Y,
                            TEXT_INPUT_FIELD_PADDING_X,
                        ),
                        direction: Direction::Row,
                        align_items: Align::Center,
                        hittable: Some(true),
                        ..BoxStyle::default()
                    },
                    decoration: Some(Decoration {
                        background: Some(Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        border: Some(Border {
                            width: 1.0,
                            color: border_color,
                        }),
                        radius: [TEXT_INPUT_FIELD_RADIUS; 4],
                        shadow: None,
                    }),
                    children: vec![Desc::Leaf {
                        id: Cow::Owned(format!("{id}::value")),
                        style: BoxStyle {
                            width: Size::Auto,
                            height: Size::Auto,
                            ..BoxStyle::default()
                        },
                        kind: LeafKind::Text {
                            content: self.value.to_string(),
                            font_size: TEXT_INPUT_VALUE_FONT_SIZE,
                            color: value_color,
                        },
                    }],
                },
            ],
        }
    }
}
