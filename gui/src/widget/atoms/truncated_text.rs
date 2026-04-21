use crate::tree::layout::{BoxStyle, LeafKind, Size, TextAlign, TextLayout, TextOverflow};
use crate::tree::Desc;
use crate::widget::atoms::label::{label_style, LabelVariant};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct TruncatedTextProps {
    pub text: Cow<'static, str>,
    pub variant: LabelVariant,
    pub muted: bool,
    pub overflow: TextOverflow,
    pub align: TextAlign,
    pub width: Size,
}

impl Default for TruncatedTextProps {
    fn default() -> Self {
        Self {
            text: Cow::Borrowed(""),
            variant: LabelVariant::Body,
            muted: false,
            overflow: TextOverflow::Ellipsis,
            align: TextAlign::Start,
            width: Size::Fill,
        }
    }
}

impl WidgetProps for TruncatedTextProps {
    fn widget_type(&self) -> &'static str {
        "TruncatedText"
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
        WidgetBuild {
            style: BoxStyle {
                width: self.width,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Leaf {
                id: Cow::Owned(format!("{id}::text")),
                style: BoxStyle {
                    width: self.width,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: self.text.to_string(),
                    style: label_style(cx.theme, self.variant, self.muted),
                    layout: TextLayout {
                        overflow: self.overflow,
                        align: self.align,
                    },
                },
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn truncated_text_defaults_to_fill_ellipsis_start() {
        let props = TruncatedTextProps::default();

        assert_eq!(props.width, Size::Fill);
        assert_eq!(props.overflow, TextOverflow::Ellipsis);
        assert_eq!(props.align, TextAlign::Start);
    }

    #[test]
    fn truncated_text_builds_text_leaf_with_requested_layout() {
        let theme = dark_theme();
        let props = TruncatedTextProps {
            text: Cow::Borrowed("Long label"),
            variant: LabelVariant::Caption,
            muted: true,
            overflow: TextOverflow::Clip,
            align: TextAlign::End,
            width: Size::Fixed(72.0),
        };

        let build = props.build(
            "truncated",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.width, Size::Fixed(72.0));
        match &build.children[0] {
            Desc::Leaf {
                style,
                kind: LeafKind::Text {
                    content, layout, ..
                },
                ..
            } => {
                assert_eq!(style.width, Size::Fixed(72.0));
                assert_eq!(content, "Long label");
                assert_eq!(layout.overflow, TextOverflow::Clip);
                assert_eq!(layout.align, TextAlign::End);
            }
            _ => panic!("expected text leaf"),
        }
    }
}
