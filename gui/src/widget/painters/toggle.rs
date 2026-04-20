use crate::interaction::{InteractionState, WidgetVisualState};
use crate::renderer::{Color, RectStyle};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::toggle::ToggleProps;

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
    let toggle = props.as_any().downcast_ref::<ToggleProps>()?;
    let root_visual = interaction
        .map(|state| state.visual_state(root_node_id, toggle.disabled))
        .unwrap_or(if toggle.disabled {
            WidgetVisualState::Disabled
        } else {
            WidgetVisualState::Normal
        });

    if node_id_str.ends_with("::track") {
        let toggle_visual = theme.toggle_visual(toggle.value, root_visual);
        let tokens = theme.components.toggle;
        return Some((
            RectStyle {
                color: toggle_visual.track_background,
                border: toggle_visual
                    .track_border
                    .map(|color| crate::renderer::Border { width: 1.0, color }),
                radius: [tokens.track_radius; 4],
                shadow: None,
            },
            toggle_visual.text,
        ));
    }

    if node_id_str == root_id {
        return Some((
            transparent_style(),
            theme.text_color_for_visual(root_visual),
        ));
    }

    None
}
