use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::widget::props::{WidgetBuild, WidgetProps};

#[derive(Clone, Debug, PartialEq)]
pub struct DropdownProps {
    pub label: Cow<'static, str>,
    pub options: Vec<Cow<'static, str>>,
    pub selected: usize,
    pub disabled: bool,
}

impl WidgetProps for DropdownProps {
    fn widget_type(&self) -> &'static str { "Dropdown" }
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
        use crate::tree::layout::{BoxStyle, Decoration, Size, Direction, Align, Edges, LeafKind};
        use crate::renderer::{Border, Color};

        // shadcn zinc 色系
        let bg = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let border_color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };  // zinc-200
        let text_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };    // zinc-900
        let arrow_color = Color { r: 0.443, g: 0.443, b: 0.478, a: 1.0 };   // zinc-500

        let selected_text = self.options.get(self.selected)
            .map(|s| s.to_string())
            .unwrap_or_default();

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Row,
                gap: 8.0,
                align_items: Align::Center,
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
                // 显示选中项
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::selected")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: selected_text,
                        font_size: 12.0,
                        color: text_color,
                    },
                },
                // 弹性占位，把箭头推到右边
                Desc::Container {
                    id: Cow::Owned(format!("{id}::spacer")),
                    style: BoxStyle {
                        flex_grow: 1.0,
                        ..BoxStyle::default()
                    },
                    decoration: None,
                    children: vec![],
                },
                // 箭头
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::arrow")),
                    style: BoxStyle {
                        width: Size::Auto,
                        height: Size::Auto,
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Text {
                        content: "▾".to_string(),
                        font_size: 12.0,
                        color: arrow_color,
                    },
                },
            ],
        }
    }
}
