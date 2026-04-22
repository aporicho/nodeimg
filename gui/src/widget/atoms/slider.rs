use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::widget::anatomy::Anatomy;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    pub label: Option<Cow<'static, str>>,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: f32,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for SliderProps {
    fn widget_type(&self) -> &'static str {
        "Slider"
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
        use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, LeafKind, Size};
        use crate::tree::Desc;

        let theme = cx.theme;
        let _metrics = theme.control_metrics(self.size, self.density);
        let tokens = theme.components.slider;
        let visual = theme.slider_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });

        // 填充比例
        let range = self.max - self.min;
        let ratio = if range > 0.0 {
            ((self.value - self.min) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // 值文本格式化
        let value_text = if self.step >= 1.0 {
            format!("{}", self.value as i32)
        } else {
            format!("{:.1}", self.value)
        };
        let anatomy = Anatomy::new(id);

        let mut children = Vec::new();
        if let Some(label) = &self.label {
            children.push(Desc::Leaf {
                id: Cow::Owned(anatomy.label()),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: label.to_string(),
                    style: TextStyle {
                        color: theme.colors.text_muted,
                        size: tokens.font_size,
                        ..theme.text_style_body_sm()
                    },
                    layout: Default::default(),
                },
            });
        }
        children.extend([
            Desc::Container {
                id: Cow::Owned(anatomy.track()),
                style: BoxStyle {
                    flex_grow: 1.0,
                    height: Size::Fixed(tokens.track_height),
                    padding: crate::tree::layout::Edges::all(tokens.track_padding),
                    direction: Direction::Row,
                    gestures: vec![Gesture::Tap, Gesture::Drag],
                    align_items: Align::Center,
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(visual.track_background),
                    border: None,
                    radius: [tokens.track_radius; 4],
                    shadow: None,
                }),
                children: vec![
                    Desc::Container {
                        id: Cow::Owned(anatomy.part("fill")),
                        style: BoxStyle {
                            flex_grow: ratio,
                            height: Size::Fill,
                            gestures: vec![Gesture::Tap, Gesture::Drag],
                            ..BoxStyle::default()
                        },
                        decoration: Some(Decoration {
                            background: Some(visual.fill),
                            border: None,
                            radius: [tokens.track_radius; 4],
                            shadow: None,
                        }),
                        children: vec![],
                    },
                    Desc::Container {
                        id: Cow::Owned(anatomy.thumb()),
                        style: BoxStyle {
                            width: Size::Fixed(tokens.thumb_size),
                            height: Size::Fixed(tokens.thumb_size),
                            gestures: vec![Gesture::Tap, Gesture::Drag],
                            ..BoxStyle::default()
                        },
                        decoration: Some(Decoration {
                            background: Some(visual.thumb),
                            border: None,
                            radius: [tokens.thumb_size / 2.0; 4],
                            shadow: None,
                        }),
                        children: vec![],
                    },
                    Desc::Container {
                        id: Cow::Owned(anatomy.part("spacer")),
                        style: BoxStyle {
                            flex_grow: 1.0 - ratio,
                            height: Size::Fill,
                            gestures: vec![Gesture::Tap, Gesture::Drag],
                            ..BoxStyle::default()
                        },
                        decoration: None,
                        children: vec![],
                    },
                ],
            },
            Desc::Leaf {
                id: Cow::Owned(anatomy.part("value")),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: value_text,
                    style: TextStyle {
                        color: visual.text,
                        size: tokens.font_size,
                        ..theme.text_style_mono_md()
                    },
                    layout: Default::default(),
                },
            },
        ]);

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
    fn slider_can_hide_label() {
        let theme = dark_theme();
        let props = SliderProps {
            label: None,
            min: 0.0,
            max: 1.0,
            step: 0.1,
            value: 0.5,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "slider",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 2);
        assert_eq!(build.children[0].id(), "slider::track");
    }
}
