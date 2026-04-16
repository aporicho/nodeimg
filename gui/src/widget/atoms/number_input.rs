use crate::renderer::TextStyle;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct NumberInputProps {
    pub label: Cow<'static, str>,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub precision: usize,
    pub disabled: bool,
}

impl WidgetProps for NumberInputProps {
    fn widget_type(&self) -> &'static str {
        "NumberInput"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn WidgetProps> {
        Box::new(self.clone())
    }

    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        use crate::renderer::Border;
        use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Size};
        use crate::tree::Desc;

        let theme = cx.theme;
        let tokens = theme.components.number_input;
        let visual = theme.text_input_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });

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
                    id: Cow::Owned(format!("{id}::label")),
                    style: BoxStyle::default(),
                    kind: LeafKind::Text {
                        content: self.label.to_string(),
                        style: TextStyle {
                            color: theme.colors.text_muted,
                            size: tokens.label_size,
                            ..theme.text_style_label_sm()
                        },
                    },
                },
                Desc::Container {
                    id: Cow::Owned(format!("{id}::field")),
                    style: BoxStyle {
                        height: Size::Fixed(tokens.field_height),
                        padding: Edges::symmetric(tokens.padding_y, tokens.padding_x),
                        direction: Direction::Row,
                        align_items: Align::Center,
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
                    children: vec![Desc::Leaf {
                        id: Cow::Owned(format!("{id}::value")),
                        style: BoxStyle::default(),
                        kind: LeafKind::Text {
                            content: format_number(self.value, self.precision),
                            style: TextStyle {
                                color: visual.text,
                                size: tokens.value_size,
                                ..theme.text_style_mono_md()
                            },
                        },
                    }],
                },
            ],
        }
    }
}

pub fn format_number(value: f32, precision: usize) -> String {
    format!("{value:.precision$}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn number_input_formats_value_with_precision() {
        let theme = dark_theme();
        let props = NumberInputProps {
            label: Cow::Borrowed("Radius"),
            value: 1.25,
            min: 0.0,
            max: 10.0,
            step: 0.25,
            precision: 2,
            disabled: false,
        };
        let build = props.build(
            "number",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        match &build.children[1] {
            crate::tree::Desc::Container { children, .. } => match &children[0] {
                crate::tree::Desc::Leaf {
                    kind: crate::tree::layout::LeafKind::Text { content, .. },
                    ..
                } => assert_eq!(content, "1.25"),
                _ => panic!("expected value leaf"),
            },
            _ => panic!("expected field container"),
        }
    }
}
