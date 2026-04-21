use crate::renderer::TextStyle;
use crate::tree::layout::{BoxStyle, Decoration, Direction, Edges, LeafKind};
use crate::tree::Desc;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

pub struct GroupProps {
    pub title: Cow<'static, str>,
    pub content: Vec<Desc>,
}

impl Clone for GroupProps {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            content: self.content.iter().map(desc_clone).collect(),
        }
    }
}

impl fmt::Debug for GroupProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("GroupProps")
            .field("title", &self.title)
            .field("content_len", &self.content.len())
            .finish()
    }
}

impl WidgetProps for GroupProps {
    fn widget_type(&self) -> &'static str {
        "Group"
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
            .is_some_and(|o| self.title == o.title)
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        let theme = cx.theme;
        let tokens = theme.components.group;

        let title = Desc::Container {
            id: Cow::Owned(format!("{id}::titlebar")),
            style: BoxStyle {
                direction: Direction::Row,
                padding: Edges::symmetric(tokens.title_padding_y, tokens.title_padding_x),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Leaf {
                id: Cow::Owned(format!("{id}::title")),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: self.title.to_string(),
                    style: TextStyle {
                        color: tokens.title_text,
                        size: tokens.title_font_size,
                        ..theme.text_style_label_sm()
                    },
                    layout: Default::default(),
                },
            }],
        };

        let content = Desc::Container {
            id: Cow::Owned(format!("{id}::content")),
            style: BoxStyle {
                direction: Direction::Column,
                gap: tokens.gap,
                ..BoxStyle::default()
            },
            decoration: None,
            children: self.content.iter().map(desc_clone).collect(),
        };

        WidgetBuild {
            style: BoxStyle {
                direction: Direction::Column,
                gap: tokens.title_gap,
                padding: Edges::all(tokens.padding),
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
            children: vec![title, content],
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
    use crate::theme::{dark_theme, Theme};

    fn build_cx<'a>(theme: &'a Theme) -> WidgetBuildCx<'a> {
        WidgetBuildCx {
            theme,
            force_rebuild: false,
        }
    }

    #[test]
    fn group_builds_title_and_content_sections() {
        let theme = dark_theme();
        let props = GroupProps {
            title: Cow::Borrowed("Parameters"),
            content: vec![],
        };

        let build = props.build("group", &build_cx(&theme));

        assert_eq!(build.children.len(), 2);
        match &build.children[0] {
            Desc::Container { id, .. } => assert_eq!(id.as_ref(), "group::titlebar"),
            _ => panic!("expected title container"),
        }
        match &build.children[1] {
            Desc::Container { id, .. } => assert_eq!(id.as_ref(), "group::content"),
            _ => panic!("expected content container"),
        }
    }
}
