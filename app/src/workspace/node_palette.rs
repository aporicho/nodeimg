use gui::action::NODE_LIBRARY_ADD_PREFIX;
use gui::renderer::Border;
use gui::theme::Theme;
use gui::tree::layout::{BoxStyle, Decoration, Direction, Edges, Size};
use gui::tree::Desc;
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
    Desc::Container {
        id: Cow::Borrowed("node_palette"),
        style: BoxStyle {
            width: Size::Fixed(320.0),
            height: Size::Auto,
            direction: Direction::Column,
            gap: theme.spacing.sm,
            padding: Edges::all(theme.spacing.sm),
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(theme.colors.surface),
            border: Some(Border {
                width: 1.0,
                color: theme.colors.border,
            }),
            radius: [theme.radii.md; 4],
            shadow: None,
        }),
        children: vec![
            Desc::Widget {
                id: Cow::Borrowed("node_palette::title"),
                props: Box::new(LabelProps {
                    text: Cow::Borrowed("Node Library"),
                    variant: LabelVariant::Title,
                    muted: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("node_palette::items"),
                props: Box::new(ScrollAreaProps {
                    height: 300.0,
                    content: library_content(state),
                }),
            },
        ],
    }
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

        category_items.push(Desc::Widget {
            id: Cow::Owned(format!("{NODE_LIBRARY_ADD_PREFIX}{}", item.type_id)),
            props: Box::new(ButtonProps {
                label: Cow::Owned(format!("{}  [{}]", item.name, item.source)),
                icon: None,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            }),
        });
    }

    if let Some(category) = current_category {
        content.push(category_group(category, category_items));
    }

    if content.is_empty() {
        content.push(Desc::Widget {
            id: Cow::Borrowed("node_palette_empty"),
            props: Box::new(LabelProps {
                text: Cow::Borrowed("No nodes available"),
                variant: LabelVariant::Caption,
                muted: true,
            }),
        });
    }

    content
}

fn category_group(category: &str, items: Vec<Desc>) -> Desc {
    Desc::Widget {
        id: Cow::Owned(format!("node_palette::category::{category}")),
        props: Box::new(GroupProps {
            title: Cow::Owned(category.to_string()),
            content: items,
        }),
    }
}
