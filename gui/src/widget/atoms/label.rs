use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, LeafKind, Size};
use crate::tree::Desc;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelVariant {
    Body,
    Caption,
    Title,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LabelProps {
    pub text: Cow<'static, str>,
    pub variant: LabelVariant,
    pub muted: bool,
}

impl WidgetProps for LabelProps {
    fn widget_type(&self) -> &'static str {
        "Label"
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
        let (font_size, color) = label_style(cx.theme, self.variant, self.muted);

        WidgetBuild {
            style: BoxStyle {
                width: Size::Auto,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Leaf {
                id: Cow::Owned(format!("{id}::text")),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: self.text.to_string(),
                    font_size,
                    color,
                },
            }],
        }
    }
}

fn label_style(theme: &Theme, variant: LabelVariant, muted: bool) -> (f32, crate::renderer::Color) {
    let font_size = match variant {
        LabelVariant::Body => theme.text.body_md,
        LabelVariant::Caption => theme.text.label_sm,
        LabelVariant::Title => theme.text.title_sm,
    };
    let color = if muted {
        theme.colors.text_muted
    } else {
        theme.colors.text
    };
    (font_size, color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn label_uses_variant_typography() {
        let theme = dark_theme();
        let props = LabelProps {
            text: Cow::Borrowed("Hello"),
            variant: LabelVariant::Title,
            muted: false,
        };

        let build = props.build(
            "label",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        match &build.children[0] {
            Desc::Leaf {
                kind: LeafKind::Text { font_size, .. },
                ..
            } => {
                assert_eq!(*font_size, theme.text.title_sm);
            }
            _ => panic!("expected text leaf"),
        }
    }
}
