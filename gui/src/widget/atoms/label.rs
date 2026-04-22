use crate::renderer::TextStyle;
use crate::theme::Theme;
use crate::ui::{self, StyleBuilder};
use crate::widget::build;
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
        let style = label_style(cx.theme, self.variant, self.muted);

        build::root()
            .auto_width()
            .auto_height()
            .child(
                ui::text(format!("{id}::text"), self.text.to_string(), style)
                    .auto_width()
                    .auto_height(),
            )
            .build()
    }
}

pub(super) fn label_style(theme: &Theme, variant: LabelVariant, muted: bool) -> TextStyle {
    let base = match variant {
        LabelVariant::Body => theme.text_style_body_md(),
        LabelVariant::Caption => theme.text_style_label_sm(),
        LabelVariant::Title => theme.text_style_title_sm(),
    };
    TextStyle {
        color: if muted {
            theme.colors.text_muted
        } else {
            theme.colors.text
        },
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::LeafKind;
    use crate::tree::Desc;

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
                kind: LeafKind::Text { style, .. },
                ..
            } => {
                assert_eq!(style.size, theme.text.title_sm);
            }
            _ => panic!("expected text leaf"),
        }
    }
}
