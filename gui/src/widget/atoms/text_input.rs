use crate::theme::{ControlSize, Density};
use crate::widget::atoms::text_box::{TextBoxFont, TextBoxMode, TextBoxProps};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct TextInputProps {
    pub label: Option<Cow<'static, str>>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl WidgetProps for TextInputProps {
    fn widget_type(&self) -> &'static str {
        "TextInput"
    }
    fn role(&self) -> crate::widget::WidgetRole {
        crate::widget::WidgetRole::TextInput
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
            mode: TextBoxMode::SingleLine,
            font: TextBoxFont::Body,
        }
        .build_text_box(id, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, Size};
    use crate::tree::Desc;

    #[test]
    fn text_input_field_uses_control_metrics() {
        let theme = dark_theme();
        let props = TextInputProps {
            label: Some(Cow::Borrowed("Prompt")),
            value: Cow::Borrowed("hello"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "input",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.width, Size::Fill);
        match &build.children[1] {
            Desc::Container { style, .. } => {
                assert_eq!(style.width, Size::Fill);
                assert_eq!(style.height, Size::Fixed(24.0));
                assert_eq!(style.padding, Edges::symmetric(4.0, 6.0));
            }
            _ => panic!("expected field container"),
        }
    }

    #[test]
    fn text_input_can_hide_label() {
        let theme = dark_theme();
        let props = TextInputProps {
            label: None,
            value: Cow::Borrowed("hello"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "input",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "input::field");
    }
}
