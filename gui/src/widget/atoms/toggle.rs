use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
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
        use crate::tree::layout::{
            Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Size,
        };
        use crate::tree::Desc;

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

        let mut children = vec![Desc::Container {
            id: Cow::Owned(format!("{id}::track")),
            style: BoxStyle {
                width: Size::Fixed(tokens.track_width),
                height: Size::Fixed(tokens.track_height),
                padding: Edges::all(tokens.track_padding),
                direction: Direction::Row,
                justify_content: thumb_justify,
                align_items: Align::Center,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(visual.track_background),
                border: None,
                radius: [tokens.track_radius; 4],
                shadow: None,
            }),
            children: vec![Desc::Container {
                id: Cow::Owned(format!("{id}::thumb")),
                style: BoxStyle {
                    width: Size::Fixed(tokens.thumb_size),
                    height: Size::Fixed(tokens.thumb_size),
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(visual.thumb),
                    border: None,
                    radius: [tokens.thumb_size / 2.0; 4],
                    shadow: None,
                }),
                children: vec![],
            }],
        }];
        if let Some(label) = &self.label {
            children.push(Desc::Leaf {
                id: Cow::Owned(format!("{id}::label")),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: label.to_string(),
                    style: TextStyle {
                        color: visual.text,
                        size: tokens.font_size,
                        ..theme.text_style_body_sm()
                    },
                    layout: Default::default(),
                },
            });
        }

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Row,
                gap: tokens.gap,
                align_items: Align::Center,
                height: Size::Auto,
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
