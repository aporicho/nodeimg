use crate::gesture::Gesture;
use crate::renderer::{Border, TextStyle};
use crate::theme::{ControlSize, Density};
use crate::tree::layout::{Align, TextAlign, TextLayout, TextOverflow};
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::anatomy::Anatomy;
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct PathInputProps {
    pub label: Option<Cow<'static, str>>,
    pub value: Cow<'static, str>,
    pub extensions: Vec<Cow<'static, str>>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for PathInputProps {
    fn widget_type(&self) -> &'static str {
        "PathInput"
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
        let metrics = theme.control_metrics(self.size, self.density);
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
                        size: metrics.label_font_size,
                        ..theme.text_style_label_sm()
                    },
                )
                .build(),
            );
        }

        let value = if self.value.is_empty() {
            extension_hint(&self.extensions)
        } else {
            self.value.to_string()
        };

        children.push(
            ui::row(anatomy.field())
                .align_items(Align::Center)
                .gap(metrics.gap)
                .fixed_height(metrics.height)
                .padding_symmetric(metrics.padding_y, metrics.padding_x)
                .gesture(Gesture::Tap)
                .hittable(true)
                .background(visual.background)
                .border(Border {
                    width: metrics.border_width,
                    color: visual.border.unwrap_or(theme.colors.border),
                })
                .radius_all(metrics.radius)
                .children(vec![
                    ui::text_with_layout(
                        anatomy.part("value"),
                        value,
                        TextStyle {
                            color: visual.text,
                            size: metrics.font_size,
                            ..theme.text_style_body_sm()
                        },
                        TextLayout {
                            overflow: TextOverflow::Ellipsis,
                            align: TextAlign::Start,
                            ..Default::default()
                        },
                    )
                    .fill_width()
                    .auto_height()
                    .flex_shrink(1.0)
                    .build(),
                    ui::text(
                        anatomy.part("button"),
                        "...",
                        TextStyle {
                            color: theme.colors.text_muted,
                            size: metrics.font_size,
                            ..theme.text_style_body_sm()
                        },
                    )
                    .build(),
                ])
                .build(),
        );

        build::column()
            .gap((metrics.gap / 2.0).max(2.0))
            .auto_height()
            .children(children)
            .build()
    }
}

fn extension_hint(extensions: &[Cow<'static, str>]) -> String {
    if extensions.is_empty() {
        "select file".to_string()
    } else {
        extensions
            .iter()
            .map(|extension| format!("*.{extension}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn path_input_can_render_without_label() {
        let theme = light_theme();
        let props = PathInputProps {
            label: None,
            value: Cow::Borrowed(""),
            extensions: vec![Cow::Borrowed("png")],
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };
        let build = props.build(
            "path",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "path::field");
    }
}
