use crate::gesture::Gesture;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    pub label: Cow<'static, str>,
    pub icon: Option<Cow<'static, str>>,
    pub disabled: bool,
}

impl WidgetProps for ButtonProps {
    fn widget_type(&self) -> &'static str {
        "Button"
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
        use crate::renderer::Border;
        use crate::tree::layout::{
            Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Size,
        };
        use crate::tree::Desc;

        let theme = cx.theme;
        let tokens = theme.components.button;
        let visual = theme.button_visual(if self.disabled {
            crate::widget::state::WidgetVisualState::Disabled
        } else {
            crate::widget::state::WidgetVisualState::Normal
        });

        WidgetBuild {
            style: BoxStyle {
                height: Size::Auto,
                direction: Direction::Row,
                align_items: Align::Center,
                justify_content: Justify::Center,
                padding: Edges::symmetric(tokens.padding_y, tokens.padding_x),
                gestures: vec![Gesture::Tap],
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
            children: vec![Desc::Leaf {
                id: Cow::Owned(format!("{id}::label")),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: self.label.to_string(),
                    font_size: tokens.font_size,
                    color: visual.text,
                },
            }],
        }
    }
}
