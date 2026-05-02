use crate::theme::{ControlSize, Density};
use crate::widget::atoms::text_box::{TextBoxFont, TextBoxMode, TextBoxProps};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct TextAreaProps {
    pub label: Option<Cow<'static, str>>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
    pub min_rows: usize,
}

impl WidgetProps for TextAreaProps {
    fn widget_type(&self) -> &'static str {
        "TextArea"
    }

    fn role(&self) -> crate::widget::WidgetRole {
        crate::widget::WidgetRole::TextArea
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
        TextBoxProps {
            label: self.label.clone(),
            value: self.value.clone(),
            disabled: self.disabled,
            size: self.size,
            density: self.density,
            mode: TextBoxMode::MultiLine {
                min_rows: self.min_rows,
            },
            font: TextBoxFont::Body,
        }
        .build_text_box(id, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::Size;
    use crate::tree::Desc;

    #[test]
    fn text_area_uses_minimum_five_rows() {
        let theme = dark_theme();
        let props = TextAreaProps {
            label: None,
            value: Cow::Borrowed("short"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
            min_rows: 5,
        };

        let build = props.build(
            "area",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        let Desc::Container { style, .. } = &build.children[0] else {
            panic!("expected field container");
        };
        assert_eq!(style.width, Size::Fill);
        assert_eq!(style.height, Size::Fill);
        assert!(style.min_height >= 5.0 * 12.0);
    }

    #[test]
    fn text_area_build_height_uses_min_rows_not_content_estimation() {
        let theme = dark_theme();
        let short = TextAreaProps {
            label: None,
            value: Cow::Borrowed("short"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
            min_rows: 5,
        };
        let long = TextAreaProps {
            value: Cow::Borrowed(
                "a very long line that wraps several times after runtime layout measures width",
            ),
            ..short.clone()
        };

        let short_build = short.build(
            "area",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );
        let long_build = long.build(
            "area",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(
            field_min_height(&short_build.children[0]),
            field_min_height(&long_build.children[0])
        );
    }

    fn field_min_height(desc: &Desc) -> f32 {
        let Desc::Container { style, .. } = desc else {
            panic!("expected field container");
        };
        style.min_height
    }
}
