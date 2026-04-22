use crate::interaction::WidgetVisualState;
use crate::renderer::{Border, Color, Shadow};
use crate::tree::layout::{BoxStyle, Decoration};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceProps {
    pub style: BoxStyle,
    pub background: Option<Color>,
    pub border: Option<Border>,
    pub radius: [f32; 4],
    pub shadow: Option<Shadow>,
    pub selected: bool,
    pub disabled: bool,
}

impl SurfaceProps {
    pub fn card(style: BoxStyle) -> Self {
        Self {
            style,
            background: None,
            border: None,
            radius: [0.0; 4],
            shadow: None,
            selected: false,
            disabled: false,
        }
    }
}

impl WidgetProps for SurfaceProps {
    fn widget_type(&self) -> &'static str {
        "Surface"
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

    fn build(&self, _id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        let theme = cx.theme;
        let visual = if self.disabled {
            WidgetVisualState::Disabled
        } else if self.selected {
            WidgetVisualState::Focused
        } else {
            WidgetVisualState::Normal
        };
        let fallback = theme.button_visual(visual);

        WidgetBuild {
            style: self.style.clone(),
            decoration: Some(Decoration {
                background: Some(self.background.unwrap_or(fallback.background)),
                border: Some(
                    self.border.unwrap_or(Border {
                        width: theme
                            .control_metrics(Default::default(), Default::default())
                            .border_width,
                        color: fallback.border.unwrap_or(theme.colors.border),
                    }),
                ),
                radius: self.radius,
                shadow: self.shadow,
            }),
            children: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;
    use crate::tree::layout::{BoxStyle, Size};

    #[test]
    fn surface_builds_decoration_from_props() {
        let theme = light_theme();
        let props = SurfaceProps {
            style: BoxStyle {
                width: Size::Fixed(10.0),
                height: Size::Fixed(12.0),
                ..BoxStyle::default()
            },
            background: Some(theme.colors.surface),
            border: None,
            radius: [8.0; 4],
            shadow: None,
            selected: false,
            disabled: false,
        };

        let build = props.build(
            "surface",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.width, Size::Fixed(10.0));
        assert_eq!(build.decoration.unwrap().radius, [8.0; 4]);
    }
}
