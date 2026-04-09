use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::widget::props::{WidgetBuild, WidgetProps};

#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    pub label: Cow<'static, str>,
    pub icon: Option<Cow<'static, str>>,
    pub disabled: bool,
}

impl WidgetProps for ButtonProps {
    fn widget_type(&self) -> &'static str { "Button" }
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
        use crate::renderer::{Border, Color};

        // shadcn zinc 色系
        let bg_color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };           // white
        let border_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };  // zinc-200
        let text_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };    // zinc-900
        let font_size = 12.0;

        WidgetBuild {
            style: BoxStyle {
                height: Size::Auto,
                direction: Direction::Row,
                align_items: Align::Center,
                justify_content: Justify::Center,
                padding: Edges::symmetric(8.0, 16.0),
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(bg_color),
                border: Some(Border { width: 1.0, color: border_color }),
                radius: [4.0; 4],
            }),
            children: vec![
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
                        color: text_color,
                    },
                },
            ],
        }
    }
}
