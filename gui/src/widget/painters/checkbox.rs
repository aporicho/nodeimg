use crate::renderer::{Color, RectStyle};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::checkbox::CheckboxProps;
use crate::widget::state::{InteractionStore, WidgetVisualState};

use super::transparent_style;

pub(super) fn visual_override(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionStore>,
    theme: &Theme,
) -> Option<(RectStyle, Color)> {
    let node = tree.get(node_id)?;
    let node_id_str = node.id.as_ref();
    let root_id = node_id_str.split("::").next()?;
    let (root_node_id, root_node) = tree.iter().find(|(_, n)| n.id.as_ref() == root_id)?;
    let NodeKind::Widget(props) = &root_node.kind else {
        return None;
    };
    let checkbox = props.as_any().downcast_ref::<CheckboxProps>()?;
    let visual = interaction
        .map(|state| state.visual_state(root_node_id, checkbox.disabled))
        .unwrap_or(if checkbox.disabled {
            WidgetVisualState::Disabled
        } else {
            WidgetVisualState::Normal
        });
    let checkbox_visual = theme.checkbox_visual(checkbox.checked, visual);
    let tokens = theme.components.checkbox;

    if node_id_str == root_id {
        return Some((transparent_style(), checkbox_visual.text));
    }

    if node_id_str.ends_with("::box") {
        return Some((
            RectStyle {
                color: checkbox_visual.box_background,
                border: checkbox_visual
                    .box_border
                    .map(|color| crate::renderer::Border {
                        width: tokens.border_width,
                        color,
                    }),
                radius: [tokens.radius; 4],
                shadow: None,
            },
            checkbox_visual.text,
        ));
    }

    None
}
