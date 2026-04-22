use crate::gesture::Gesture;
use crate::renderer::TextStyle;
use crate::tree::layout::{BoxStyle, Direction};
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

pub struct CollapsibleProps {
    pub title: Cow<'static, str>,
    pub expanded: bool,
    pub disabled: bool,
    pub content: Vec<Desc>,
}

impl Clone for CollapsibleProps {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            expanded: self.expanded,
            disabled: self.disabled,
            content: self.content.iter().map(desc_clone).collect(),
        }
    }
}

impl fmt::Debug for CollapsibleProps {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CollapsibleProps")
            .field("title", &self.title)
            .field("expanded", &self.expanded)
            .field("disabled", &self.disabled)
            .field("content_len", &self.content.len())
            .finish()
    }
}

impl WidgetProps for CollapsibleProps {
    fn widget_type(&self) -> &'static str {
        "Collapsible"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn WidgetProps> {
        Box::new(self.clone())
    }

    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any().downcast_ref::<Self>().is_some_and(|o| {
            self.title == o.title && self.expanded == o.expanded && self.disabled == o.disabled
        })
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn build(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        let tokens = cx.theme.components.collapsible;
        let chevron = if self.expanded { "▾" } else { "▸" };

        let mut children = vec![ui::row(format!("{id}::header"))
            .align_items(crate::tree::layout::Align::Center)
            .gap(tokens.gap)
            .padding_symmetric(tokens.header_padding_y, tokens.header_padding_x)
            .gesture(Gesture::Tap)
            .background(tokens.header_background)
            .radius([tokens.radius, tokens.radius, 0.0, 0.0])
            .children(vec![
                ui::leaf(
                    format!("{id}::chevron"),
                    crate::tree::layout::LeafKind::Text {
                        content: chevron.to_string(),
                        style: TextStyle {
                            color: tokens.title_text,
                            size: tokens.title_font_size,
                            ..cx.theme.text_style_label_sm()
                        },
                        layout: Default::default(),
                    },
                )
                .build(),
                ui::leaf(
                    format!("{id}::title"),
                    crate::tree::layout::LeafKind::Text {
                        content: self.title.to_string(),
                        style: TextStyle {
                            color: tokens.title_text,
                            size: tokens.title_font_size,
                            ..cx.theme.text_style_label_sm()
                        },
                        layout: Default::default(),
                    },
                )
                .build(),
            ])
            .build()];

        if self.expanded {
            children.push(
                ui::column(format!("{id}::content"))
                    .gap(tokens.gap)
                    .padding_all(tokens.content_padding)
                    .children(self.content.iter().cloned())
                    .build(),
            );
        }

        WidgetBuild {
            style: BoxStyle {
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
            children,
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
    fn collapsible_hides_content_when_collapsed() {
        let theme = dark_theme();
        let props = CollapsibleProps {
            title: Cow::Borrowed("Advanced"),
            expanded: false,
            disabled: false,
            content: vec![],
        };

        let build = props.build(
            "advanced",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
    }
}
