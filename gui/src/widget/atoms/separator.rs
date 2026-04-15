use crate::tree::layout::{BoxStyle, Decoration, Size};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeparatorOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeparatorProps {
    pub orientation: SeparatorOrientation,
}

impl WidgetProps for SeparatorProps {
    fn widget_type(&self) -> &'static str {
        "Separator"
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
        let tokens = cx.theme.components.separator;
        let (width, height) = match self.orientation {
            SeparatorOrientation::Horizontal => (Size::Fill, Size::Fixed(tokens.thickness)),
            SeparatorOrientation::Vertical => (Size::Fixed(tokens.thickness), Size::Fill),
        };

        WidgetBuild {
            style: BoxStyle {
                width,
                height,
                hittable: Some(false),
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(tokens.color),
                border: None,
                radius: [0.0; 4],
                shadow: None,
            }),
            children: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn horizontal_separator_uses_theme_thickness() {
        let theme = dark_theme();
        let props = SeparatorProps {
            orientation: SeparatorOrientation::Horizontal,
        };

        let build = props.build(
            "sep",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.width, Size::Fill);
        assert_eq!(
            build.style.height,
            Size::Fixed(theme.components.separator.thickness)
        );
    }
}
