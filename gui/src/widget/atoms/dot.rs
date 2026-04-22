use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::tree::layout::{BoxStyle, Decoration, Size};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct DotProps {
    pub diameter: f32,
    pub fill: Color,
    pub border: Option<Border>,
    pub hittable: bool,
    pub gestures: Vec<Gesture>,
}

impl WidgetProps for DotProps {
    fn widget_type(&self) -> &'static str {
        "Dot"
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

    fn build(&self, _id: &str, _cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        WidgetBuild {
            style: BoxStyle {
                width: Size::Fixed(self.diameter),
                height: Size::Fixed(self.diameter),
                hittable: Some(self.hittable),
                gestures: self.gestures.clone(),
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(self.fill),
                border: self.border,
                radius: [self.diameter * 0.5; 4],
                shadow: None,
            }),
            children: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Color;
    use crate::theme::light_theme;
    use crate::tree::layout::Size;

    #[test]
    fn dot_builds_fixed_circle() {
        let theme = light_theme();
        let props = DotProps {
            diameter: 10.0,
            fill: Color::WHITE,
            border: None,
            hittable: true,
            gestures: Vec::new(),
        };
        let build = props.build(
            "dot",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.width, Size::Fixed(10.0));
        assert_eq!(build.decoration.unwrap().radius, [5.0; 4]);
    }
}
