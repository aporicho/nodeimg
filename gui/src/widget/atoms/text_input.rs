use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::anatomy::Anatomy;
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct TextInputProps {
    pub label: Option<Cow<'static, str>>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
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
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        use crate::renderer::Border;
        use crate::tree::layout::Align;

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
                .auto_width()
                .auto_height()
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
                .child(
                    ui::text(
                        anatomy.part("value"),
                        self.value.to_string(),
                        TextStyle {
                            color: visual.text,
                            size: tokens.value_size,
                            ..theme.text_style_body_sm()
                        },
                    )
                    .auto_width()
                    .auto_height(),
                )
                .build(),
        );

        build::column()
            .gap(tokens.gap)
            .auto_height()
            .children(children)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, Size};
    use crate::tree::Desc;

    #[test]
    fn text_input_field_uses_control_metrics() {
        let theme = dark_theme();
        let props = TextInputProps {
            label: Some(Cow::Borrowed("Prompt")),
            value: Cow::Borrowed("hello"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "input",
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
            _ => panic!("expected field container"),
        }
    }

    #[test]
    fn text_input_can_hide_label() {
        let theme = dark_theme();
        let props = TextInputProps {
            label: None,
            value: Cow::Borrowed("hello"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "input",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "input::field");
    }
}
