use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::widget::props::{WidgetBuild, WidgetProps};

#[derive(Clone, Debug, PartialEq)]
pub struct TextInputProps {
    pub label: Cow<'static, str>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
}

impl WidgetProps for TextInputProps {
    fn widget_type(&self) -> &'static str { "TextInput" }
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
        use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Edges, LeafKind};
        use crate::renderer::{Border, Color};

        // shadcn zinc 色系
        let bg = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let border_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };   // zinc-200
        let label_color = Color { r: 0.443, g: 0.443, b: 0.478, a: 1.0 };    // zinc-500
        let value_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };    // zinc-900

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Column,
                gap: 4.0,
                padding: Edges::symmetric(8.0, 12.0),
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(bg),
                border: Some(Border { width: 1.0, color: border_color }),
                radius: [4.0; 4],
                shadow: None,
            }),
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
                        font_size: 11.0,
                        color: label_color,
                    },
                },
                // value
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::value")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: self.value.to_string(),
                        font_size: 12.0,
                        color: value_color,
                    },
                },
            ],
        }
    }
}
