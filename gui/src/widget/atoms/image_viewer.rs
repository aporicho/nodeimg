use crate::renderer::ImageStyle;
use crate::tree::layout::{LeafKind, TextureHandle};
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ImageViewerProps {
    pub texture: TextureHandle,
    pub height: f32,
    pub style: ImageStyle,
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

        build::root()
            .fill_width()
            .fixed_height(self.height)
            .background(tokens.background)
            .border(crate::renderer::Border {
                width: tokens.border_width,
                color: tokens.border,
            })
            .radius_all(tokens.radius)
            .child(
                ui::leaf(
                    format!("{id}::image"),
                    LeafKind::Image {
                        texture: self.texture,
                        style: self.style,
                    },
                )
                .fill_width()
                .fill_height(),
            )
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::ImageFilter;
    use crate::theme::dark_theme;
    use crate::tree::layout::LeafKind;
    use crate::tree::Desc;

    #[test]
    fn image_viewer_builds_image_leaf() {
        let theme = dark_theme();
        let props = ImageViewerProps {
            texture: TextureHandle(7),
            height: 120.0,
            style: ImageStyle::default().with_filter(ImageFilter::Nearest),
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
                kind: LeafKind::Image { texture, style },
                ..
            } => {
                assert_eq!(*texture, TextureHandle(7));
                assert_eq!(
                    *style,
                    ImageStyle::default().with_filter(ImageFilter::Nearest)
                );
            }
            _ => panic!("expected image leaf"),
        }
    }
}
