use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::widget::anatomy::Anatomy;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct NumberInputProps {
    pub label: Option<Cow<'static, str>>,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub precision: usize,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
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
        use crate::tree::layout::{Align, BoxStyle, Direction, Size};
        use crate::ui::{self, DecorationBuilder, StyleBuilder};

        let theme = cx.theme;
        let tokens = theme.text_field_metrics(self.size, self.density);
        let visual = theme.text_input_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });
        let anatomy = Anatomy::new(id);

        let mut children = Vec::new();
        if let Some(label) = &self.label {
            children.push(
                ui::text(
                    anatomy.label(),
                    label.to_string(),
                    TextStyle {
                        color: theme.colors.text_muted,
                        size: tokens.label_size,
                        ..theme.text_style_label_sm()
                    },
                )
                .build(),
            );
        }
        children.push(
            ui::row(anatomy.field())
                .fixed_height(tokens.field_height)
                .padding_symmetric(tokens.padding_y, tokens.padding_x)
                .align_items(Align::Center)
                .hittable(true)
                .background(visual.background)
                .border(Border {
                    width: tokens.border_width,
                    color: visual.border.unwrap_or(theme.colors.border),
                })
                .radius_all(tokens.radius)
                .child(ui::text(
                    anatomy.part("value"),
                    format_number(self.value, self.precision),
                    TextStyle {
                        color: visual.text,
                        size: tokens.value_size,
                        ..theme.text_style_mono_md()
                    },
                ))
                .build(),
        );

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Column,
                gap: tokens.gap,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children,
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
    use crate::tree::layout::{Edges, Size};
    use crate::tree::Desc;

    #[test]
    fn number_input_formats_value_with_precision() {
        let theme = dark_theme();
        let props = NumberInputProps {
            label: Some(Cow::Borrowed("Radius")),
            value: 1.25,
            min: 0.0,
            max: 10.0,
            step: 0.25,
            precision: 2,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        };
        let build = props.build(
            "number",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        match &build.children[1] {
            Desc::Container { children, .. } => match &children[0] {
                Desc::Leaf {
                    kind: crate::tree::layout::LeafKind::Text { content, .. },
                    ..
                } => assert_eq!(content, "1.25"),
                _ => panic!("expected value leaf"),
            },
            _ => panic!("expected field container"),
        }
    }

    #[test]
    fn number_input_field_uses_control_metrics() {
        let theme = dark_theme();
        let props = NumberInputProps {
            label: Some(Cow::Borrowed("Radius")),
            value: 1.25,
            min: 0.0,
            max: 10.0,
            step: 0.25,
            precision: 2,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };
        let build = props.build(
            "number",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        match &build.children[1] {
            Desc::Container { style, .. } => {
                assert_eq!(style.height, Size::Fixed(24.0));
                assert_eq!(style.padding, Edges::symmetric(4.0, 6.0));
            }
            _ => panic!("expected number field container"),
        }
    }

    #[test]
    fn number_input_can_hide_label() {
        let theme = dark_theme();
        let props = NumberInputProps {
            label: None,
            value: 1.25,
            min: 0.0,
            max: 10.0,
            step: 0.25,
            precision: 2,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };
        let build = props.build(
            "number",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "number::field");
    }
}
