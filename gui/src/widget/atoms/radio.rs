use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Size};
use crate::tree::Desc;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct RadioProps {
    pub label: Cow<'static, str>,
    pub selected: bool,
    pub disabled: bool,
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
        let tokens = theme.components.radio;
        let visual = theme.radio_visual(
            self.selected,
            if self.disabled {
                crate::widget::state::WidgetVisualState::Disabled
            } else {
                crate::widget::state::WidgetVisualState::Normal
            },
        );

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Row,
                gap: tokens.gap,
                align_items: Align::Center,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![
                Desc::Container {
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
                },
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::label")),
                    style: BoxStyle::default(),
                    kind: crate::tree::layout::LeafKind::Text {
                        content: self.label.to_string(),
                        style: TextStyle {
                            color: visual.text,
                            size: tokens.font_size,
                            ..theme.text_style_body_sm()
                        },
                    },
                },
            ],
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
            label: Cow::Borrowed("High quality"),
            selected: true,
            disabled: false,
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
}
