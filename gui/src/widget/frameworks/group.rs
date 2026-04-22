use crate::renderer::TextStyle;
use crate::tree::layout::LeafKind;
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::build;
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

        let title = ui::row(format!("{id}::titlebar"))
            .padding_symmetric(tokens.title_padding_y, tokens.title_padding_x)
            .child(ui::leaf(
                format!("{id}::title"),
                LeafKind::Text {
                    content: self.title.to_string(),
                    style: TextStyle {
                        color: tokens.title_text,
                        size: tokens.title_font_size,
                        ..theme.text_style_label_sm()
                    },
                    layout: Default::default(),
                },
            ))
            .build();

        let content = ui::column(format!("{id}::content"))
            .gap(tokens.gap)
            .children(self.content.iter().cloned())
            .build();

        build::column()
            .gap(tokens.title_gap)
            .padding_all(tokens.padding)
            .background(tokens.background)
            .border(crate::renderer::Border {
                width: tokens.border_width,
                color: tokens.border,
            })
            .radius_all(tokens.radius)
            .children([title, content])
            .build()
    }
}

fn desc_clone(d: &Desc) -> Desc {
    d.clone()
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
