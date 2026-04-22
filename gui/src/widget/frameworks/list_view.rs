use crate::tree::layout::{BoxStyle, Size};
use crate::tree::Desc;
use crate::ui::{self, StyleBuilder};
use crate::widget::frameworks::scroll_area::ScrollAreaProps;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

pub struct ListViewProps {
    pub height: f32,
    pub items: Vec<Desc>,
}

impl Clone for ListViewProps {
    fn clone(&self) -> Self {
        Self {
            height: self.height,
            items: self.items.iter().map(desc_clone).collect(),
        }
    }
}

impl fmt::Debug for ListViewProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ListViewProps")
            .field("height", &self.height)
            .field("items_len", &self.items.len())
            .finish()
    }
}

impl WidgetProps for ListViewProps {
    fn widget_type(&self) -> &'static str {
        "ListView"
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

    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        let list_tokens = cx.theme.components.list_view;

        WidgetBuild {
            style: BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![ui::widget(
                Cow::Owned(format!("{id}::scroll")),
                ScrollAreaProps {
                    height: self.height,
                    content: vec![ui::column(format!("{id}::items"))
                        .fill_width()
                        .auto_height()
                        .gap(list_tokens.item_gap)
                        .children(self.items.iter().cloned())
                        .build()],
                },
            )
            .build()],
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
    fn list_view_wraps_scroll_area() {
        let theme = dark_theme();
        let props = ListViewProps {
            height: 96.0,
            items: vec![],
        };
        let build = props.build(
            "list",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        match &build.children[0] {
            Desc::Widget(widget) => assert_eq!(widget.id(), "list::scroll"),
            _ => panic!("expected scroll area child"),
        }
    }
}
