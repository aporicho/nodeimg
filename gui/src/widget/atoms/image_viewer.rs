use crate::tree::layout::{BoxStyle, Decoration, LeafKind, Size, TextureHandle};
use crate::tree::Desc;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ImageViewerProps {
    pub texture: TextureHandle,
    pub height: f32,
}

impl WidgetProps for ImageViewerProps {
    fn widget_type(&self) -> &'static str {
        "ImageViewer"
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
        let tokens = cx.theme.components.image_viewer;

        WidgetBuild {
            style: BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(self.height),
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(tokens.background),
                border: Some(crate::renderer::Border {
                    width: tokens.border_width,
                    color: tokens.border,
                }),
                radius: [tokens.radius; 4],
                shadow: None,
            }),
            children: vec![Desc::Leaf {
                id: Cow::Owned(format!("{id}::image")),
                style: BoxStyle {
                    width: Size::Fill,
                    height: Size::Fill,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Image {
                    texture: self.texture,
                    tint: None,
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
    fn image_viewer_builds_image_leaf() {
        let theme = dark_theme();
        let props = ImageViewerProps {
            texture: TextureHandle(7),
            height: 120.0,
        };

        let build = props.build(
            "viewer",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        match &build.children[0] {
            Desc::Leaf {
                kind: LeafKind::Image { texture, .. },
                ..
            } => assert_eq!(*texture, TextureHandle(7)),
            _ => panic!("expected image leaf"),
        }
    }
}
