use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleProps {
    pub label: Option<Cow<'static, str>>,
    pub value: bool,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for ToggleProps {
    fn widget_type(&self) -> &'static str {
        "Toggle"
    }
    fn role(&self) -> crate::widget::WidgetRole {
        crate::widget::WidgetRole::Toggle
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
        use crate::tree::layout::{Align, Justify};

        let theme = cx.theme;
        let _metrics = theme.control_metrics(self.size, self.density);
        let tokens = theme.components.toggle;
        let visual = theme.toggle_visual(
            self.value,
            if self.disabled {
                crate::interaction::WidgetVisualState::Disabled
            } else {
                crate::interaction::WidgetVisualState::Normal
            },
        );
        let thumb_justify = if self.value {
            Justify::End
        } else {
            Justify::Start
        };

        let mut children = vec![ui::row(format!("{id}::track"))
            .fixed_width(tokens.track_width)
            .fixed_height(tokens.track_height)
            .padding_all(tokens.track_padding)
            .justify_content(thumb_justify)
            .align_items(Align::Center)
            .gesture(Gesture::Tap)
            .background(visual.track_background)
            .radius_all(tokens.track_radius)
            .child(
                ui::container(format!("{id}::thumb"))
                    .fixed_width(tokens.thumb_size)
                    .fixed_height(tokens.thumb_size)
                    .background(visual.thumb)
                    .radius_all(tokens.thumb_size / 2.0),
            )
            .build()];
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
                .auto_width()
                .auto_height()
                .build(),
            );
        }

        build::row()
            .gap(tokens.gap)
            .align_items(Align::Center)
            .auto_height()
            .children(children)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn toggle_can_hide_label() {
        let theme = dark_theme();
        let props = ToggleProps {
            label: None,
            value: true,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "toggle",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "toggle::track");
    }
}
