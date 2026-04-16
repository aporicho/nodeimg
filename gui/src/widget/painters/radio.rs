use crate::interaction::{InteractionState, WidgetVisualState};
use crate::renderer::{Color, RectStyle};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::radio::RadioProps;

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
    let radio = props.as_any().downcast_ref::<RadioProps>()?;
    let visual = interaction
        .map(|state| state.visual_state(root_node_id, radio.disabled))
        .unwrap_or(if radio.disabled {
            WidgetVisualState::Disabled
        } else {
            WidgetVisualState::Normal
        });
    let radio_visual = theme.radio_visual(radio.selected, visual);
    let tokens = theme.components.radio;

    if node_id_str == root_id {
        return Some((transparent_style(), radio_visual.text));
    }

    if node_id_str.ends_with("::ring") {
        return Some((
            RectStyle {
                color: radio_visual.ring_background,
                border: radio_visual
                    .ring_border
                    .map(|color| crate::renderer::Border {
                        width: tokens.border_width,
                        color,
                    }),
                radius: [tokens.ring_size / 2.0; 4],
                shadow: None,
            },
            radio_visual.text,
        ));
    }

    if node_id_str.ends_with("::dot") {
        let tokens = theme.components.radio;
        return Some((
            RectStyle {
                color: radio_visual.dot,
                border: None,
                radius: [tokens.dot_size / 2.0; 4],
                shadow: None,
            },
            radio_visual.text,
        ));
    }

    None
}
