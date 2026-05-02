use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::tree::layout::Align;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct RadioProps {
    pub label: Option<Cow<'static, str>>,
    pub selected: bool,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for RadioProps {
    fn widget_type(&self) -> &'static str {
        "Radio"
    }

    fn role(&self) -> crate::widget::WidgetRole {
        crate::widget::WidgetRole::Radio
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
        let tokens = theme.components.radio;
        let visual = theme.radio_visual(
            self.selected,
            if self.disabled {
                crate::interaction::WidgetVisualState::Disabled
            } else {
                crate::interaction::WidgetVisualState::Normal
            },
        );

        let mut ring = ui::container(format!("{id}::ring"))
            .fixed_width(tokens.ring_size)
            .fixed_height(tokens.ring_size)
            .align_items(Align::Center)
            .justify_content(crate::tree::layout::Justify::Center)
            .background(visual.ring_background)
            .radius_all(tokens.ring_size / 2.0)
            .child(
                ui::container(format!("{id}::dot"))
                    .fixed_width(tokens.dot_size)
                    .fixed_height(tokens.dot_size)
                    .background(visual.dot)
                    .radius_all(tokens.dot_size / 2.0),
            );
        if let Some(color) = visual.ring_border {
            ring = ring.border(crate::renderer::Border {
                width: tokens.border_width,
                color,
            });
        }

        let mut children = vec![ring.build()];
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

        build::row()
            .gap(tokens.gap)
            .align_items(Align::Center)
            .gesture(Gesture::Tap)
            .children(children)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn radio_builds_ring_and_label() {
        let theme = dark_theme();
        let props = RadioProps {
            label: Some(Cow::Borrowed("High quality")),
            selected: true,
            disabled: false,
            size: ControlSize::default(),
            density: Density::default(),
        };

        let build = props.build(
            "radio",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 2);
    }

    #[test]
    fn radio_can_hide_label() {
        let theme = dark_theme();
        let props = RadioProps {
            label: None,
            selected: true,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "radio",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "radio::ring");
    }
}
