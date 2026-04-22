use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::tree::layout::{Align, BoxStyle, Direction};
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckboxProps {
    pub label: Option<Cow<'static, str>>,
    pub checked: bool,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for CheckboxProps {
    fn widget_type(&self) -> &'static str {
        "Checkbox"
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
        let theme = cx.theme;
        let _metrics = theme.control_metrics(self.size, self.density);
        let tokens = theme.components.checkbox;
        let visual = theme.checkbox_visual(
            self.checked,
            if self.disabled {
                crate::interaction::WidgetVisualState::Disabled
            } else {
                crate::interaction::WidgetVisualState::Normal
            },
        );

        let mut box_builder = ui::container(format!("{id}::box"))
            .fixed_width(tokens.box_size)
            .fixed_height(tokens.box_size)
            .align_items(Align::Center)
            .justify_content(crate::tree::layout::Justify::Center)
            .background(visual.box_background)
            .radius_all(tokens.radius)
            .child(ui::text(
                format!("{id}::check"),
                if self.checked { "✓" } else { "" },
                TextStyle {
                    color: visual.check,
                    size: tokens.check_font_size,
                    ..theme.text_style_body_sm()
                },
            ));
        if let Some(color) = visual.box_border {
            box_builder = box_builder.border(crate::renderer::Border {
                width: tokens.border_width,
                color,
            });
        }

        let mut children = vec![box_builder.build()];
        if let Some(label) = &self.label {
            children.push(
                ui::text(
                    format!("{id}::label"),
                    label.to_string(),
                    TextStyle {
                        color: visual.text,
                        size: tokens.font_size,
                        ..theme.text_style_body_sm()
                    },
                )
                .build(),
            );
        }

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Row,
                gap: tokens.gap,
                align_items: Align::Center,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            decoration: None,
            children,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn checkbox_builds_box_and_label() {
        let theme = dark_theme();
        let props = CheckboxProps {
            label: Some(Cow::Borrowed("Snap to grid")),
            checked: true,
            disabled: false,
            size: ControlSize::default(),
            density: Density::default(),
        };

        let build = props.build(
            "checkbox",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 2);
    }

    #[test]
    fn checkbox_can_hide_label() {
        let theme = dark_theme();
        let props = CheckboxProps {
            label: None,
            checked: true,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "checkbox",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "checkbox::box");
    }
}
