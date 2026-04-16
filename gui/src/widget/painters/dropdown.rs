use crate::interaction::{InteractionState, WidgetVisualState};
use crate::renderer::{Color, RectStyle};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::dropdown::DropdownProps;

use super::transparent_style;

pub(super) fn visual_override(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionState>,
    theme: &Theme,
) -> Option<(RectStyle, Color)> {
    let node = tree.get(node_id)?;
    let node_id_str = node.id.as_ref();
    let root_id = node_id_str.split("::").next()?;
    let (root_node_id, root_node) = tree.iter().find(|(_, n)| n.id.as_ref() == root_id)?;
    let NodeKind::Widget(props) = &root_node.kind else {
        return None;
    };
    let dropdown = props.as_any().downcast_ref::<DropdownProps>()?;
    let visual = interaction
        .map(|state| state.visual_state(root_node_id, dropdown.disabled))
        .unwrap_or(if dropdown.disabled {
            WidgetVisualState::Disabled
        } else {
            WidgetVisualState::Normal
        });
    let surface = theme.dropdown_visual(visual);
    let tokens = theme.components.dropdown;

    if node_id_str == root_id {
        return Some((transparent_style(), surface.text));
    }

    if node_id_str.ends_with("::field") {
        return Some((
            RectStyle {
                color: surface.background,
                border: surface.border.map(|color| crate::renderer::Border {
                    width: tokens.border_width,
                    color,
                }),
                radius: [tokens.radius; 4],
                shadow: None,
            },
            surface.text,
        ));
    }

    None
}
