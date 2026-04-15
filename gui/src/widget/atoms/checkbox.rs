use crate::gesture::Gesture;
use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, LeafKind, Size};
use crate::tree::Desc;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckboxProps {
    pub label: Cow<'static, str>,
    pub checked: bool,
    pub disabled: bool,
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
        let tokens = theme.components.checkbox;
        let visual = theme.checkbox_visual(
            self.checked,
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
                    id: Cow::Owned(format!("{id}::box")),
                    style: BoxStyle {
                        width: Size::Fixed(tokens.box_size),
                        height: Size::Fixed(tokens.box_size),
                        align_items: Align::Center,
                        justify_content: crate::tree::layout::Justify::Center,
                        ..BoxStyle::default()
                    },
                    decoration: Some(Decoration {
                        background: Some(visual.box_background),
                        border: visual.box_border.map(|color| crate::renderer::Border {
                            width: tokens.border_width,
                            color,
                        }),
                        radius: [tokens.radius; 4],
                        shadow: None,
                    }),
                    children: vec![Desc::Leaf {
                        id: Cow::Owned(format!("{id}::check")),
                        style: BoxStyle::default(),
                        kind: LeafKind::Text {
                            content: if self.checked { "✓" } else { "" }.to_string(),
                            font_size: tokens.check_font_size,
                            color: visual.check,
                        },
                    }],
                },
                Desc::Leaf {
                    id: Cow::Owned(format!("{id}::label")),
                    style: BoxStyle::default(),
                    kind: LeafKind::Text {
                        content: self.label.to_string(),
                        font_size: tokens.font_size,
                        color: visual.text,
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
    fn checkbox_builds_box_and_label() {
        let theme = dark_theme();
        let props = CheckboxProps {
            label: Cow::Borrowed("Snap to grid"),
            checked: true,
            disabled: false,
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
}
