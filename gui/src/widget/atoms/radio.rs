use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Size};
use crate::tree::Desc;
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

        let mut children = vec![Desc::Container {
            id: Cow::Owned(format!("{id}::ring")),
            style: BoxStyle {
                width: Size::Fixed(tokens.ring_size),
                height: Size::Fixed(tokens.ring_size),
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Center,
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(visual.ring_background),
                border: visual.ring_border.map(|color| crate::renderer::Border {
                    width: tokens.border_width,
                    color,
                }),
                radius: [tokens.ring_size / 2.0; 4],
                shadow: None,
            }),
            children: vec![Desc::Container {
                id: Cow::Owned(format!("{id}::dot")),
                style: BoxStyle {
                    width: Size::Fixed(tokens.dot_size),
                    height: Size::Fixed(tokens.dot_size),
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(visual.dot),
                    border: None,
                    radius: [tokens.dot_size / 2.0; 4],
                    shadow: None,
                }),
                children: vec![],
            }],
        }];
        if let Some(label) = &self.label {
            children.push(Desc::Leaf {
                id: Cow::Owned(format!("{id}::label")),
                style: BoxStyle::default(),
                kind: crate::tree::layout::LeafKind::Text {
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
