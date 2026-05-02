use crate::icon::names;
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density};
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::anatomy::Anatomy;
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct DropdownProps {
    pub label: Option<Cow<'static, str>>,
    pub options: Vec<Cow<'static, str>>,
    pub selected: usize,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DropdownOptionProps {
    pub label: Cow<'static, str>,
    pub selected: bool,
    pub highlighted: bool,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl DropdownOptionProps {
    pub(crate) fn marker_icon(&self) -> Option<crate::icon::IconName> {
        if self.selected {
            Some(names::CHECK)
        } else if self.highlighted {
            Some(names::NAV_ARROW_RIGHT)
        } else {
            None
        }
    }
}

impl WidgetProps for DropdownOptionProps {
    fn widget_type(&self) -> &'static str {
        "DropdownOption"
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
        use crate::gesture::Gesture;
        use crate::interaction::WidgetVisualState;
        use crate::renderer::Border;
        use crate::tree::layout::{Align, Justify};

        let theme = cx.theme;
        let metrics = theme.control_metrics(self.size, self.density);
        let visual_state = if self.disabled {
            WidgetVisualState::Disabled
        } else if self.highlighted {
            WidgetVisualState::Focused
        } else {
            WidgetVisualState::Normal
        };
        let visual = theme.button_visual(visual_state);
        let marker_color = if self.disabled {
            theme.colors.text_disabled
        } else if self.selected {
            theme.colors.accent
        } else {
            theme.colors.text_muted
        };
        let anatomy = Anatomy::new(id);

        let mut marker_slot = ui::row(anatomy.part("marker"))
            .fixed_width(metrics.icon_size)
            .fixed_height(metrics.icon_size)
            .align_items(Align::Center)
            .justify_content(Justify::Center);
        if let Some(icon) = self.marker_icon() {
            marker_slot = marker_slot.child(ui::icon(
                anatomy.part("marker_icon"),
                icon,
                metrics.icon_size,
                marker_color,
            ));
        }

        build::row()
            .align_items(Align::Center)
            .gap(metrics.gap)
            .fixed_height(metrics.height)
            .padding_symmetric(metrics.padding_y, metrics.padding_x)
            .gesture(Gesture::Tap)
            .hittable(true)
            .background(visual.background)
            .border(Border {
                width: metrics.border_width,
                color: visual.border.unwrap_or(theme.colors.border),
            })
            .radius_all(metrics.radius)
            .children(vec![
                marker_slot.build(),
                ui::text(
                    anatomy.label(),
                    self.label.to_string(),
                    TextStyle {
                        color: visual.text,
                        size: metrics.font_size,
                        ..theme.text_style_body_sm()
                    },
                )
                .auto_width()
                .auto_height()
                .build(),
            ])
            .build()
    }
}

impl WidgetProps for DropdownProps {
    fn widget_type(&self) -> &'static str {
        "Dropdown"
    }
    fn role(&self) -> crate::widget::WidgetRole {
        crate::widget::WidgetRole::Dropdown
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
        use crate::gesture::Gesture;
        use crate::renderer::Border;
        use crate::tree::layout::Align;

        let theme = cx.theme;
        let metrics = theme.control_metrics(self.size, self.density);
        let visual = theme.dropdown_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });

        let selected_text = self
            .options
            .get(self.selected)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let anatomy = Anatomy::new(id);

        let mut children = Vec::new();
        if let Some(label) = &self.label {
            children.push(
                ui::text(
                    anatomy.label(),
                    label.to_string(),
                    TextStyle {
                        color: theme.colors.text_muted,
                        size: metrics.label_font_size,
                        ..theme.text_style_label_sm()
                    },
                )
                .auto_width()
                .auto_height()
                .build(),
            );
        }
        children.push(
            ui::row(anatomy.field())
                .gap(metrics.gap)
                .align_items(Align::Center)
                .fixed_height(metrics.height)
                .padding_symmetric(metrics.padding_y, metrics.padding_x)
                .gesture(Gesture::Tap)
                .hittable(true)
                .background(visual.background)
                .border(Border {
                    width: metrics.border_width,
                    color: visual.border.unwrap_or(theme.colors.border),
                })
                .radius_all(metrics.radius)
                .children(vec![
                    ui::text(
                        anatomy.part("selected"),
                        selected_text,
                        TextStyle {
                            color: visual.text,
                            size: metrics.font_size,
                            ..theme.text_style_body_sm()
                        },
                    )
                    .auto_width()
                    .auto_height()
                    .build(),
                    ui::container(anatomy.part("spacer")).flex_grow(1.0).build(),
                    ui::icon(
                        anatomy.part("arrow"),
                        names::NAV_ARROW_DOWN,
                        metrics.icon_size,
                        theme.colors.text_muted,
                    )
                    .build(),
                ])
                .build(),
        );

        build::column()
            .gap((metrics.gap / 2.0).max(2.0))
            .auto_height()
            .children(children)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, LeafKind, Size};
    use crate::tree::Desc;

    #[test]
    fn dropdown_field_uses_control_metrics() {
        let theme = dark_theme();
        let props = DropdownProps {
            label: Some(Cow::Borrowed("Mode")),
            options: vec![Cow::Borrowed("Normal")],
            selected: 0,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "dropdown",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        match &build.children[1] {
            Desc::Container { style, .. } => {
                assert_eq!(style.height, Size::Fixed(24.0));
                assert_eq!(style.padding, Edges::symmetric(4.0, 6.0));
            }
            _ => panic!("expected dropdown field container"),
        }
    }

    #[test]
    fn dropdown_can_hide_label() {
        let theme = dark_theme();
        let props = DropdownProps {
            label: None,
            options: vec![Cow::Borrowed("Normal")],
            selected: 0,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "dropdown",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "dropdown::field");
    }

    #[test]
    fn dropdown_field_uses_chevron_icon() {
        let theme = dark_theme();
        let props = DropdownProps {
            label: None,
            options: vec![Cow::Borrowed("Normal")],
            selected: 0,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "dropdown",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        let Desc::Container { children, .. } = &build.children[0] else {
            panic!("dropdown field should be a container");
        };
        let Desc::Leaf { id, style, kind } = &children[2] else {
            panic!("dropdown arrow should be an icon leaf");
        };
        assert_eq!(id.as_ref(), "dropdown::arrow");
        assert_eq!(style.width, Size::Fixed(12.0));
        assert_eq!(style.height, Size::Fixed(12.0));
        let LeafKind::Icon { spec } = kind else {
            panic!("dropdown arrow should use LeafKind::Icon");
        };
        assert_eq!(spec.id, crate::icon::IconId::from(names::NAV_ARROW_DOWN));
    }

    #[test]
    fn dropdown_option_uses_icon_marker_without_label_prefix() {
        let theme = dark_theme();
        let props = DropdownOptionProps {
            label: Cow::Borrowed("Normal"),
            selected: true,
            highlighted: true,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "option",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 2);
        let Desc::Container { children, .. } = &build.children[0] else {
            panic!("option marker slot should be a container");
        };
        let Desc::Leaf { kind, .. } = &children[0] else {
            panic!("selected option should render marker icon leaf");
        };
        let LeafKind::Icon { spec } = kind else {
            panic!("selected option marker should use LeafKind::Icon");
        };
        assert_eq!(spec.id, crate::icon::IconId::from(names::CHECK));

        let Desc::Leaf { kind, .. } = &build.children[1] else {
            panic!("option label should be a text leaf");
        };
        let LeafKind::Text { content, .. } = kind else {
            panic!("option label should use text");
        };
        assert_eq!(content, "Normal");
        assert!(!content.starts_with('✓'));
        assert!(!content.starts_with('›'));
    }
}
