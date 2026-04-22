use gui::action::NODE_LIBRARY_ADD_PREFIX;
use gui::renderer::Border;
use gui::theme::Theme;
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::frameworks::group::GroupProps;
use gui::widget::frameworks::scroll_area::ScrollAreaProps;
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodePaletteState {
    pub(crate) items: Vec<NodePaletteItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodePaletteItem {
    pub(crate) type_id: String,
    pub(crate) name: String,
    pub(crate) category: String,
    pub(crate) source: String,
}

pub(crate) fn overlay_content(state: &NodePaletteState, theme: &Theme) -> Desc {
    ui::column("node_palette")
        .fixed_width(320.0)
        .auto_height()
        .gap(theme.spacing.sm)
        .padding_all(theme.spacing.sm)
        .background(theme.colors.surface)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(theme.radii.md)
        .children(vec![
            ui::widget(
                Cow::Borrowed("node_palette::title"),
                LabelProps {
                    text: Cow::Borrowed("Node Library"),
                    variant: LabelVariant::Title,
                    muted: false,
                },
            )
            .build(),
            ui::widget(
                Cow::Borrowed("node_palette::items"),
                ScrollAreaProps {
                    height: 300.0,
                    content: library_content(state),
                },
            )
            .build(),
        ])
        .build()
}

fn library_content(state: &NodePaletteState) -> Vec<Desc> {
    let mut content = Vec::new();
    let mut current_category: Option<&str> = None;
    let mut category_items = Vec::new();

    for item in &state.items {
        if current_category != Some(item.category.as_str()) {
            if let Some(category) = current_category {
                content.push(category_group(
                    category,
                    std::mem::take(&mut category_items),
                ));
            }
            current_category = Some(&item.category);
        }

        category_items.push(
            ui::widget(
                Cow::Owned(format!("{NODE_LIBRARY_ADD_PREFIX}{}", item.type_id)),
                ButtonProps {
                    label: Cow::Owned(format!("{}  [{}]", item.name, item.source)),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )
            .build(),
        );
    }

    if let Some(category) = current_category {
        content.push(category_group(category, category_items));
    }

    if content.is_empty() {
        content.push(
            ui::widget(
                Cow::Borrowed("node_palette_empty"),
                LabelProps {
                    text: Cow::Borrowed("No nodes available"),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            )
            .build(),
        );
    }

    content
}

fn category_group(category: &str, items: Vec<Desc>) -> Desc {
    ui::widget(
        Cow::Owned(format!("node_palette::category::{category}")),
        GroupProps {
            title: Cow::Owned(category.to_string()),
            content: items,
        },
    )
    .build()
}
