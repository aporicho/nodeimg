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
pub struct DropdownProps {
    pub label: Option<Cow<'static, str>>,
    pub options: Vec<Cow<'static, str>>,
    pub selected: usize,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for DropdownProps {
    fn widget_type(&self) -> &'static str {
        "Dropdown"
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
        use crate::gesture::Gesture;
        use crate::renderer::Border;
        use crate::tree::layout::Align;

        let theme = cx.theme;
        let metrics = theme.control_metrics(self.size, self.density);
        let visual = theme.dropdown_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });

        let selected_text = self
            .options
            .get(self.selected)
            .map(|s| s.to_string())
            .unwrap_or_default();
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
                .auto_width()
                .auto_height()
                .build(),
            );
        }
        children.push(
            ui::row(anatomy.field())
                .gap(metrics.gap)
                .align_items(Align::Center)
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
                    ui::text(
                        anatomy.part("selected"),
                        selected_text,
                        TextStyle {
                            color: visual.text,
                            size: metrics.font_size,
                            ..theme.text_style_body_sm()
                        },
                    )
                    .auto_width()
                    .auto_height()
                    .build(),
                    ui::container(anatomy.part("spacer")).flex_grow(1.0).build(),
                    ui::text(
                        anatomy.part("arrow"),
                        "▾",
                        TextStyle {
                            color: theme.colors.text_muted,
                            size: metrics.font_size,
                            ..theme.text_style_body_sm()
                        },
                    )
                    .auto_width()
                    .auto_height()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, Size};
    use crate::tree::Desc;

    #[test]
    fn dropdown_field_uses_control_metrics() {
        let theme = dark_theme();
        let props = DropdownProps {
            label: Some(Cow::Borrowed("Mode")),
            options: vec![Cow::Borrowed("Normal")],
            selected: 0,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "dropdown",
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
            _ => panic!("expected dropdown field container"),
        }
    }

    #[test]
    fn dropdown_can_hide_label() {
        let theme = dark_theme();
        let props = DropdownProps {
            label: None,
            options: vec![Cow::Borrowed("Normal")],
            selected: 0,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "dropdown",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "dropdown::field");
    }
}
