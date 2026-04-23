use crate::gesture::Gesture;
use crate::icon::IconId;
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
pub struct ButtonProps {
    pub label: Cow<'static, str>,
    pub icon: Option<IconId>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
}

impl ButtonProps {
    pub fn new(label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
            size: ControlSize::default(),
            density: Density::default(),
        }
    }

    pub fn icon_only(icon: impl Into<IconId>) -> Self {
        Self {
            label: Cow::Borrowed(""),
            icon: Some(icon.into()),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<IconId>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }
}

impl WidgetProps for ButtonProps {
    fn widget_type(&self) -> &'static str {
        "Button"
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
        use crate::renderer::Border;
        use crate::tree::layout::{Align, Justify};

        let theme = cx.theme;
        let metrics = theme.control_metrics(self.size, self.density);
        let visual = theme.button_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });
        let anatomy = Anatomy::new(id);
        let mut children = Vec::new();
        if let Some(icon) = &self.icon {
            children.push(
                ui::icon(
                    anatomy.part("icon"),
                    icon.clone(),
                    metrics.icon_size,
                    visual.text,
                )
                .build(),
            );
        }
        if !self.label.is_empty() {
            children.push(
                ui::text(
                    anatomy.label(),
                    self.label.to_string(),
                    TextStyle {
                        color: visual.text,
                        size: metrics.font_size,
                        ..theme.text_style_body_sm()
                    },
                )
                .build(),
            );
        }

        build::row()
            .align_items(Align::Center)
            .justify_content(Justify::Center)
            .gap(metrics.gap)
            .fixed_height(metrics.height)
            .padding_symmetric(metrics.padding_y, metrics.padding_x)
            .gesture(Gesture::Tap)
            .background(visual.background)
            .border(Border {
                width: metrics.border_width,
                color: visual.border.unwrap_or(theme.colors.border),
            })
            .radius_all(metrics.radius)
            .children(children)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, Size};
    use crate::tree::Desc;

    #[test]
    fn button_uses_control_metrics_for_default_size() {
        let theme = dark_theme();
        let props = ButtonProps {
            label: Cow::Borrowed("Run"),
            icon: None,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        };

        let build = props.build(
            "button",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.height, Size::Fixed(36.0));
        assert_eq!(build.style.padding, Edges::symmetric(8.0, 12.0));
    }

    #[test]
    fn button_can_use_compact_small_metrics() {
        let theme = dark_theme();
        let props = ButtonProps {
            label: Cow::Borrowed("Run"),
            icon: None,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let build = props.build(
            "button",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.style.height, Size::Fixed(24.0));
        assert_eq!(build.style.padding, Edges::symmetric(4.0, 6.0));
    }

    #[test]
    fn button_builds_leading_icon_leaf() {
        let theme = dark_theme();
        let props = ButtonProps::new("Add").with_icon(crate::icon::names::PLUS);

        let build = props.build(
            "button",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 2);
        let Desc::Leaf { id, style, kind } = &build.children[0] else {
            panic!("button icon should be a leaf");
        };
        assert_eq!(id.as_ref(), "button::icon");
        assert_eq!(style.width, Size::Fixed(16.0));
        assert_eq!(style.height, Size::Fixed(16.0));
        let crate::tree::layout::LeafKind::Icon { spec } = kind else {
            panic!("button icon should use LeafKind::Icon");
        };
        assert_eq!(spec.id, crate::icon::IconId::from(crate::icon::names::PLUS));
    }

    #[test]
    fn icon_only_button_omits_empty_label_leaf() {
        let theme = dark_theme();
        let props = ButtonProps::icon_only(crate::icon::names::XMARK);

        let build = props.build(
            "button",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(build.children.len(), 1);
        assert_eq!(build.children[0].id(), "button::icon");
    }
}
