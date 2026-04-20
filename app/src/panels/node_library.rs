use super::{PanelBuildContext, NODE_LIBRARY_ADD_PREFIX};
use gui::panel::{PanelConfig, PanelDeclaration, PanelId};
use gui::renderer::Rect;
use gui::tree::Desc;
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::frameworks::group::GroupProps;
use std::borrow::Cow;

pub(crate) fn panel(ctx: &PanelBuildContext<'_>) -> PanelDeclaration {
    let state = ctx.node_library;

    PanelDeclaration {
        config: PanelConfig {
            id: PanelId::new(state.panel_id.clone()),
            title: Cow::Borrowed("Node Library"),
            default_rect: Rect {
                x: state.x,
                y: state.y,
                w: 300.0,
                h: 360.0,
            },
            min_size: [260.0, 240.0],
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: state.open,
        },
        content: if state.open {
            library_content(ctx)
        } else {
            Vec::new()
        },
    }
}

fn library_content(ctx: &PanelBuildContext<'_>) -> Vec<Desc> {
    let mut content = Vec::new();
    let mut current_category: Option<&str> = None;
    let mut category_items = Vec::new();

    for item in &ctx.node_library.items {
        if current_category != Some(item.category.as_str()) {
            if let Some(category) = current_category {
                content.push(category_group(category, std::mem::take(&mut category_items)));
            }
            current_category = Some(&item.category);
        }

        category_items.push(Desc::Widget {
            id: Cow::Owned(format!("{NODE_LIBRARY_ADD_PREFIX}{}", item.type_id)),
            props: Box::new(ButtonProps {
                label: Cow::Owned(format!("{}  [{}]", item.name, item.source)),
                icon: None,
                disabled: false,
            }),
        });
    }

    if let Some(category) = current_category {
        content.push(category_group(category, category_items));
    }

    if content.is_empty() {
        content.push(Desc::Widget {
            id: Cow::Borrowed("node_library_empty"),
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
        id: Cow::Owned(format!("node_library::category::{category}")),
        props: Box::new(GroupProps {
            title: Cow::Owned(category.to_string()),
            content: items,
        }),
    }
}
