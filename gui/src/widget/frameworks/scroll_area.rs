use crate::tree::layout::{BoxStyle, Direction, Edges, Overflow, Size};
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::fmt;

pub struct ScrollAreaProps {
    pub height: f32,
    pub content: Vec<Desc>,
}

impl Clone for ScrollAreaProps {
    fn clone(&self) -> Self {
        Self {
            height: self.height,
            content: self.content.iter().map(desc_clone).collect(),
        }
    }
}

impl fmt::Debug for ScrollAreaProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ScrollAreaProps")
            .field("height", &self.height)
            .field("content_len", &self.content.len())
            .finish()
    }
}

impl WidgetProps for ScrollAreaProps {
    fn widget_type(&self) -> &'static str {
        "ScrollArea"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn WidgetProps> {
        Box::new(self.clone())
    }

    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|o| self.height == o.height)
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, _id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        let tokens = cx.theme.components.scroll_area;

        WidgetBuild {
            style: BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(self.height),
                overflow: Overflow::Scroll,
                padding: Edges::all(tokens.padding),
                direction: Direction::Column,
                ..BoxStyle::default()
            },
            decoration: ui::container("_")
                .background(tokens.background)
                .border(crate::renderer::Border {
                    width: tokens.border_width,
                    color: tokens.border,
                })
                .radius_all(tokens.radius)
                .build_decoration(),
            children: self.content.clone(),
        }
    }
}

fn desc_clone(d: &Desc) -> Desc {
    d.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;

    #[test]
    fn scroll_area_sets_overflow_scroll() {
        let theme = dark_theme();
        let props = ScrollAreaProps {
            height: 120.0,
            content: vec![],
        };
        let build = props.build(
            "scroll",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.overflow, Overflow::Scroll);
        assert_eq!(build.style.height, Size::Fixed(120.0));
    }
}
