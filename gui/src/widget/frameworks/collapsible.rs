use crate::gesture::Gesture;
use crate::icon::names;
use crate::renderer::TextStyle;
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::build;
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

    fn role(&self) -> crate::widget::WidgetRole {
        crate::widget::WidgetRole::Collapsible
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
        let chevron = if self.expanded {
            names::NAV_ARROW_DOWN
        } else {
            names::NAV_ARROW_RIGHT
        };

        let mut children = vec![ui::row(format!("{id}::header"))
            .align_items(crate::tree::layout::Align::Center)
            .gap(tokens.gap)
            .padding_symmetric(tokens.header_padding_y, tokens.header_padding_x)
            .gesture(Gesture::Tap)
            .background(tokens.header_background)
            .radius([tokens.radius, tokens.radius, 0.0, 0.0])
            .children(vec![
                ui::icon(
                    format!("{id}::chevron"),
                    chevron,
                    tokens.title_font_size,
                    tokens.title_text,
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

        build::column()
            .background(tokens.background)
            .border(crate::renderer::Border {
                width: tokens.border_width,
                color: tokens.border,
            })
            .radius_all(tokens.radius)
            .children(children)
            .build()
    }
}

fn desc_clone(d: &Desc) -> Desc {
    d.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::LeafKind;

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

    #[test]
    fn collapsible_uses_chevron_icons_for_state() {
        let theme = dark_theme();
        let collapsed = CollapsibleProps {
            title: Cow::Borrowed("Advanced"),
            expanded: false,
            disabled: false,
            content: vec![],
        }
        .build(
            "advanced",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );
        let expanded = CollapsibleProps {
            title: Cow::Borrowed("Advanced"),
            expanded: true,
            disabled: false,
            content: vec![],
        }
        .build(
            "advanced",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(header_icon_id(&collapsed), names::NAV_ARROW_RIGHT.as_str());
        assert_eq!(header_icon_id(&expanded), names::NAV_ARROW_DOWN.as_str());
    }

    fn header_icon_id(build: &WidgetBuild) -> String {
        let Desc::Container { children, .. } = &build.children[0] else {
            panic!("collapsible header should be a container");
        };
        let Desc::Leaf { kind, .. } = &children[0] else {
            panic!("collapsible chevron should be an icon leaf");
        };
        let LeafKind::Icon { spec } = kind else {
            panic!("collapsible chevron should use LeafKind::Icon");
        };
        spec.id.as_str().to_string()
    }
}
