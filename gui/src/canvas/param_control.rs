use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Size, TextLayout,
    TextOverflow,
};
use crate::tree::Desc;
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub enum CanvasNodeParamControl {
    ReadOnly {
        value: String,
    },
    Text {
        value: String,
    },
    Number {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        precision: usize,
    },
    Slider {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    Toggle {
        checked: bool,
    },
    Select {
        options: Vec<String>,
        selected: usize,
    },
    Color {
        rgba: [f32; 4],
    },
    FilePath {
        path: String,
        extensions: Vec<String>,
    },
}

impl Default for CanvasNodeParamControl {
    fn default() -> Self {
        Self::ReadOnly {
            value: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasParamControlMetrics {
    pub control_height: f32,
    pub control_width: f32,
    pub slider_track_height: f32,
    pub slider_thumb_size: f32,
    pub swatch_size: f32,
    pub gap: f32,
    pub padding_x: f32,
    pub radius: f32,
}

impl CanvasParamControlMetrics {
    pub fn from_theme(_theme: &Theme) -> Self {
        Self {
            control_height: 24.0,
            control_width: 128.0,
            slider_track_height: 6.0,
            slider_thumb_size: 12.0,
            swatch_size: 18.0,
            gap: 8.0,
            padding_x: 8.0,
            radius: 6.0,
        }
    }
}

pub fn param_control(
    id: impl Into<Cow<'static, str>>,
    control: &CanvasNodeParamControl,
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let id = id.into();
    match control {
        CanvasNodeParamControl::ReadOnly { value } => read_only(id, value, theme, metrics),
        CanvasNodeParamControl::Text { value } => framed_text(id, value, theme, metrics),
        CanvasNodeParamControl::Number {
            value, precision, ..
        } => framed_text(id, &format_number(*value, *precision), theme, metrics),
        CanvasNodeParamControl::Slider {
            value,
            min,
            max,
            step,
            ..
        } => slider(id, *value, *min, *max, *step, theme, metrics),
        CanvasNodeParamControl::Toggle { checked } => toggle(id, *checked, theme, metrics),
        CanvasNodeParamControl::Select { options, selected } => {
            let value = options.get(*selected).map(String::as_str).unwrap_or("");
            select(id, value, theme, metrics)
        }
        CanvasNodeParamControl::Color { rgba } => color_value(id, *rgba, theme, metrics),
        CanvasNodeParamControl::FilePath { path, extensions } => {
            let value = if path.is_empty() {
                format!("*.{}", extensions.join(", *."))
            } else {
                path.clone()
            };
            file_path(id, &value, theme, metrics)
        }
    }
}

fn read_only(
    id: Cow<'static, str>,
    value: &str,
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let base = id.to_string();
    Desc::Container {
        id,
        style: control_row_style(metrics),
        decoration: None,
        children: vec![text_leaf(
            child_id(&base, "value"),
            value,
            theme.colors.text_muted,
            theme,
            Size::Fill,
        )],
    }
}

fn framed_text(
    id: Cow<'static, str>,
    value: &str,
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let base = id.to_string();
    Desc::Container {
        id,
        style: framed_control_style(metrics),
        decoration: framed_control_decoration(theme, metrics),
        children: vec![text_leaf(
            child_id(&base, "value"),
            value,
            theme.colors.text,
            theme,
            Size::Fill,
        )],
    }
}

fn slider(
    id: Cow<'static, str>,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let base = id.to_string();
    let range = max - min;
    let ratio = if range > 0.0 {
        ((value - min) / range).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let spacer_ratio = (1.0 - ratio).max(0.0);
    let value_text = if step >= 1.0 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.2}")
    };

    Desc::Container {
        id,
        style: BoxStyle {
            width: Size::Fixed(metrics.control_width),
            height: Size::Fixed(metrics.control_height),
            direction: Direction::Row,
            align_items: Align::Center,
            gap: metrics.gap,
            gestures: vec![Gesture::Tap, Gesture::Drag],
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![
            Desc::Container {
                id: child_id(&base, "track"),
                style: BoxStyle {
                    width: Size::Fill,
                    height: Size::Fixed(metrics.slider_track_height),
                    direction: Direction::Row,
                    align_items: Align::Center,
                    gestures: vec![Gesture::Tap, Gesture::Drag],
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(theme.colors.border),
                    border: None,
                    radius: [metrics.slider_track_height * 0.5; 4],
                    shadow: None,
                }),
                children: vec![
                    Desc::Container {
                        id: child_id(&base, "fill"),
                        style: BoxStyle {
                            flex_grow: ratio,
                            height: Size::Fill,
                            ..BoxStyle::default()
                        },
                        decoration: Some(Decoration {
                            background: Some(theme.colors.accent),
                            border: None,
                            radius: [metrics.slider_track_height * 0.5; 4],
                            shadow: None,
                        }),
                        children: Vec::new(),
                    },
                    Desc::Container {
                        id: child_id(&base, "thumb"),
                        style: BoxStyle {
                            width: Size::Fixed(metrics.slider_thumb_size),
                            height: Size::Fixed(metrics.slider_thumb_size),
                            gestures: vec![Gesture::Tap, Gesture::Drag],
                            ..BoxStyle::default()
                        },
                        decoration: Some(Decoration {
                            background: Some(theme.colors.surface),
                            border: Some(Border {
                                width: 1.0,
                                color: theme.colors.accent,
                            }),
                            radius: [metrics.slider_thumb_size * 0.5; 4],
                            shadow: None,
                        }),
                        children: Vec::new(),
                    },
                    Desc::Container {
                        id: child_id(&base, "spacer"),
                        style: BoxStyle {
                            flex_grow: spacer_ratio,
                            height: Size::Fill,
                            ..BoxStyle::default()
                        },
                        decoration: None,
                        children: Vec::new(),
                    },
                ],
            },
            text_leaf(
                child_id(&base, "value"),
                &value_text,
                theme.colors.text_muted,
                theme,
                Size::Fixed(34.0),
            ),
        ],
    }
}

fn toggle(
    id: Cow<'static, str>,
    checked: bool,
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let base = id.to_string();
    Desc::Container {
        id,
        style: control_row_style(metrics),
        decoration: None,
        children: vec![
            Desc::Container {
                id: child_id(&base, "box"),
                style: BoxStyle {
                    width: Size::Fixed(metrics.swatch_size),
                    height: Size::Fixed(metrics.swatch_size),
                    align_items: Align::Center,
                    justify_content: Justify::Center,
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(if checked {
                        theme.colors.accent
                    } else {
                        theme.colors.surface
                    }),
                    border: Some(Border {
                        width: 1.0,
                        color: theme.colors.border,
                    }),
                    radius: [4.0; 4],
                    shadow: None,
                }),
                children: vec![text_leaf(
                    child_id(&base, "mark"),
                    if checked { "x" } else { "" },
                    theme.colors.surface,
                    theme,
                    Size::Auto,
                )],
            },
            text_leaf(
                child_id(&base, "value"),
                if checked { "On" } else { "Off" },
                theme.colors.text,
                theme,
                Size::Fill,
            ),
        ],
    }
}

fn select(
    id: Cow<'static, str>,
    value: &str,
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let base = id.to_string();
    Desc::Container {
        id,
        style: framed_control_style(metrics),
        decoration: framed_control_decoration(theme, metrics),
        children: vec![
            text_leaf(
                child_id(&base, "value"),
                value,
                theme.colors.text,
                theme,
                Size::Fill,
            ),
            text_leaf(
                child_id(&base, "arrow"),
                "v",
                theme.colors.text_muted,
                theme,
                Size::Auto,
            ),
        ],
    }
}

fn color_value(
    id: Cow<'static, str>,
    rgba: [f32; 4],
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let base = id.to_string();
    Desc::Container {
        id,
        style: control_row_style(metrics),
        decoration: None,
        children: vec![
            Desc::Container {
                id: child_id(&base, "swatch"),
                style: BoxStyle {
                    width: Size::Fixed(metrics.swatch_size),
                    height: Size::Fixed(metrics.swatch_size),
                    ..BoxStyle::default()
                },
                decoration: Some(Decoration {
                    background: Some(Color {
                        r: rgba[0],
                        g: rgba[1],
                        b: rgba[2],
                        a: rgba[3],
                    }),
                    border: Some(Border {
                        width: 1.0,
                        color: theme.colors.border,
                    }),
                    radius: [metrics.radius; 4],
                    shadow: None,
                }),
                children: Vec::new(),
            },
            text_leaf(
                child_id(&base, "value"),
                &format_color(rgba),
                theme.colors.text,
                theme,
                Size::Fill,
            ),
        ],
    }
}

fn file_path(
    id: Cow<'static, str>,
    value: &str,
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let base = id.to_string();
    Desc::Container {
        id,
        style: framed_control_style(metrics),
        decoration: framed_control_decoration(theme, metrics),
        children: vec![
            text_leaf(
                child_id(&base, "value"),
                value,
                theme.colors.text,
                theme,
                Size::Fill,
            ),
            text_leaf(
                child_id(&base, "button"),
                "...",
                theme.colors.text_muted,
                theme,
                Size::Auto,
            ),
        ],
    }
}

fn control_row_style(metrics: CanvasParamControlMetrics) -> BoxStyle {
    BoxStyle {
        width: Size::Fixed(metrics.control_width),
        height: Size::Fixed(metrics.control_height),
        direction: Direction::Row,
        align_items: Align::Center,
        gap: metrics.gap,
        ..BoxStyle::default()
    }
}

fn framed_control_style(metrics: CanvasParamControlMetrics) -> BoxStyle {
    BoxStyle {
        width: Size::Fixed(metrics.control_width),
        height: Size::Fixed(metrics.control_height),
        padding: Edges::symmetric(0.0, metrics.padding_x),
        direction: Direction::Row,
        align_items: Align::Center,
        gap: metrics.gap,
        ..BoxStyle::default()
    }
}

fn framed_control_decoration(
    theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Option<Decoration> {
    Some(Decoration {
        background: Some(theme.colors.surface),
        border: Some(Border {
            width: 1.0,
            color: theme.colors.border,
        }),
        radius: [metrics.radius; 4],
        shadow: None,
    })
}

fn text_leaf(
    id: Cow<'static, str>,
    content: &str,
    color: Color,
    theme: &Theme,
    width: Size,
) -> Desc {
    Desc::Leaf {
        id,
        style: BoxStyle {
            width,
            height: Size::Auto,
            flex_shrink: 1.0,
            ..BoxStyle::default()
        },
        kind: LeafKind::Text {
            content: content.to_string(),
            style: crate::renderer::TextStyle {
                color,
                ..theme.text_style_label_sm()
            },
            layout: TextLayout {
                overflow: TextOverflow::Ellipsis,
                ..Default::default()
            },
        },
    }
}

fn child_id(parent: &str, part: &str) -> Cow<'static, str> {
    Cow::Owned(format!("{parent}::{part}"))
}

fn format_number(value: f32, precision: usize) -> String {
    format!("{value:.precision$}")
}

fn format_color(rgba: [f32; 4]) -> String {
    let r = (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{r:02X}{g:02X}{b:02X}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn slider_param_control_builds_track_thumb_and_value() {
        let theme = light_theme();
        let metrics = CanvasParamControlMetrics::from_theme(&theme);

        let Desc::Container { children, .. } = param_control(
            "control",
            &CanvasNodeParamControl::Slider {
                value: 0.5,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            &theme,
            metrics,
        ) else {
            panic!("slider control should build a container");
        };

        assert_eq!(children[0].id(), "control::track");
        assert_eq!(children[1].id(), "control::value");
    }

    #[test]
    fn color_param_control_formats_hex_value() {
        let theme = light_theme();
        let metrics = CanvasParamControlMetrics::from_theme(&theme);

        let Desc::Container { children, .. } = param_control(
            "control",
            &CanvasNodeParamControl::Color {
                rgba: [1.0, 0.5, 0.0, 1.0],
            },
            &theme,
            metrics,
        ) else {
            panic!("color control should build a container");
        };

        assert_eq!(children[0].id(), "control::swatch");
        assert_eq!(children[1].id(), "control::value");
    }
}
