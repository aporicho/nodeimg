use crate::tree::layout::{BoxStyle, Direction, Size};
use crate::tree::Desc;
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
            children: vec![Desc::Widget {
                id: Cow::Owned(format!("{id}::scroll")),
                props: Box::new(ScrollAreaProps {
                    height: self.height,
                    content: vec![Desc::Container {
                        id: Cow::Owned(format!("{id}::items")),
                        style: BoxStyle {
                            width: Size::Fill,
                            height: Size::Auto,
                            direction: Direction::Column,
                            gap: list_tokens.item_gap,
                            ..BoxStyle::default()
                        },
                        decoration: None,
                        children: self.items.iter().map(desc_clone).collect(),
                    }],
                }),
            }],
        }
    }
}

fn desc_clone(d: &Desc) -> Desc {
    match d {
        Desc::Container {
            id,
            style,
            decoration,
            children,
        } => Desc::Container {
            id: id.clone(),
            style: style.clone(),
            decoration: decoration.clone(),
            children: children.iter().map(desc_clone).collect(),
        },
        Desc::Leaf { id, style, kind } => Desc::Leaf {
            id: id.clone(),
            style: style.clone(),
            kind: kind.clone(),
        },
        Desc::Widget { id, props } => Desc::Widget {
            id: id.clone(),
            props: props.clone_box(),
        },
    }
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
            Desc::Widget { id, .. } => assert_eq!(id.as_ref(), "list::scroll"),
            _ => panic!("expected scroll area child"),
        }
    }
}
