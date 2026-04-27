use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density, TextInputTheme, Theme};
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::anatomy::Anatomy;
use crate::widget::build;
use crate::widget::props::{WidgetBuild, WidgetBuildCx, WidgetProps};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBoxMode {
    SingleLine,
    MultiLine { min_rows: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBoxFont {
    Body,
    Mono,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextBoxProps {
    pub label: Option<Cow<'static, str>>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
    pub size: ControlSize,
    pub density: Density,
    pub mode: TextBoxMode,
    pub font: TextBoxFont,
}

impl TextBoxProps {
    pub(crate) fn value_style(&self, theme: &Theme, tokens: TextInputTheme) -> TextStyle {
        text_box_value_style(theme, tokens, self.font)
    }

    pub(crate) fn build_text_box(&self, id: &str, cx: &WidgetBuildCx<'_>) -> WidgetBuild {
        use crate::renderer::Border;
        use crate::tree::layout::{Align, TextLayout, TextOverflow};

        let theme = cx.theme;
        let tokens = theme.text_field_metrics(self.size, self.density);
        let visual = theme.text_input_visual(if self.disabled {
            crate::interaction::WidgetVisualState::Disabled
        } else {
            crate::interaction::WidgetVisualState::Normal
        });
        let anatomy = Anatomy::new(id);
        let text_style = TextStyle {
            color: visual.text,
            ..self.value_style(theme, tokens)
        };
        let mut children = Vec::new();

        if let Some(label) = &self.label {
            children.push(
                ui::text(
                    anatomy.label(),
                    label.to_string(),
                    TextStyle {
                        color: theme.colors.text_muted,
                        size: tokens.label_size,
                        ..theme.text_style_label_sm()
                    },
                )
                .auto_width()
                .auto_height()
                .build(),
            );
        }

        match self.mode {
            TextBoxMode::SingleLine => {
                children.push(
                    ui::row(anatomy.field())
                        .fill_width()
                        .fixed_height(tokens.field_height)
                        .padding_symmetric(tokens.padding_y, tokens.padding_x)
                        .align_items(Align::Center)
                        .hittable(true)
                        .background(visual.background)
                        .border(Border {
                            width: tokens.border_width,
                            color: visual.border.unwrap_or(theme.colors.border),
                        })
                        .radius_all(tokens.radius)
                        .child(
                            ui::text_with_layout(
                                anatomy.part("value"),
                                String::new(),
                                text_style,
                                TextLayout {
                                    overflow: TextOverflow::Clip,
                                    ..Default::default()
                                },
                            )
                            .fill_width()
                            .auto_height()
                            .flex_shrink(1.0),
                        )
                        .build(),
                );

                build::column()
                    .fill_width()
                    .gap(tokens.gap)
                    .auto_height()
                    .children(children)
                    .build()
            }
            TextBoxMode::MultiLine { min_rows } => {
                let line_height = tokens.value_size * text_style.line_height;
                let field_min_height =
                    min_rows.max(1) as f32 * line_height + tokens.padding_y * 2.0;
                children.push(
                    ui::row(anatomy.field())
                        .fill_width()
                        .fill_height()
                        .min_height(field_min_height)
                        .flex_grow(1.0)
                        .padding_symmetric(tokens.padding_y, tokens.padding_x)
                        .align_items(Align::Start)
                        .hittable(true)
                        .background(visual.background)
                        .border(Border {
                            width: tokens.border_width,
                            color: visual.border.unwrap_or(theme.colors.border),
                        })
                        .radius_all(tokens.radius)
                        .child(
                            ui::text_with_layout(
                                anatomy.part("value"),
                                String::new(),
                                text_style,
                                TextLayout {
                                    overflow: TextOverflow::Clip,
                                    ..Default::default()
                                },
                            )
                            .fill_width()
                            .auto_height()
                            .flex_shrink(1.0),
                        )
                        .build(),
                );

                build::column()
                    .fill_width()
                    .gap(tokens.gap)
                    .fill_height()
                    .min_height(field_min_height)
                    .flex_grow(1.0)
                    .children(children)
                    .build()
            }
        }
    }
}

impl WidgetProps for TextBoxProps {
    fn widget_type(&self) -> &'static str {
        "TextBox"
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
        self.build_text_box(id, cx)
    }
}

pub(crate) fn text_box_value_style(
    theme: &Theme,
    tokens: TextInputTheme,
    font: TextBoxFont,
) -> TextStyle {
    let base = match font {
        TextBoxFont::Body => theme.text_style_body_sm(),
        TextBoxFont::Mono => theme.text_style_mono_md(),
    };
    TextStyle {
        size: tokens.value_size,
        ..base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::tree::layout::{Edges, Size};
    use crate::tree::Desc;

    #[test]
    fn single_line_field_uses_value_anchor_not_layout_text() {
        let theme = dark_theme();
        let props = TextBoxProps {
            label: None,
            value: Cow::Borrowed("long text belongs to runtime paint"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
            mode: TextBoxMode::SingleLine,
            font: TextBoxFont::Body,
        };

        let build = props.build(
            "box",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        let Desc::Container {
            style, children, ..
        } = &build.children[0]
        else {
            panic!("expected field container");
        };
        assert_eq!(style.width, Size::Fill);
        assert_eq!(style.height, Size::Fixed(24.0));
        assert_eq!(style.padding, Edges::symmetric(4.0, 6.0));

        let Desc::Leaf {
            kind: crate::tree::layout::LeafKind::Text { content, .. },
            ..
        } = &children[0]
        else {
            panic!("expected value text leaf");
        };
        assert!(content.is_empty());
    }

    #[test]
    fn multiline_field_fills_parent_height_with_row_minimum() {
        let theme = dark_theme();
        let props = TextBoxProps {
            label: None,
            value: Cow::Borrowed("runtime wrapped text"),
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
            mode: TextBoxMode::MultiLine { min_rows: 5 },
            font: TextBoxFont::Body,
        };

        let build = props.build(
            "box",
            &WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        let Desc::Container { style, .. } = &build.children[0] else {
            panic!("expected field container");
        };
        assert_eq!(style.width, Size::Fill);
        assert_eq!(style.height, Size::Fill);
        assert!(style.min_height >= 5.0 * 12.0);
    }
}
