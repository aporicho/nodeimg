use crate::interaction::{InteractionState, WidgetVisualState};
use crate::renderer::{Color, RectStyle};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::slider::SliderProps;

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
    let slider = props.as_any().downcast_ref::<SliderProps>()?;
    let root_visual = interaction
        .map(|state| state.visual_state(root_node_id, slider.disabled))
        .unwrap_or(if slider.disabled {
            WidgetVisualState::Disabled
        } else {
            WidgetVisualState::Normal
        });

    if node_id_str.ends_with("::track") {
        let slider_visual = theme.slider_visual(root_visual);
        let tokens = theme.components.slider;
        return Some((
            RectStyle {
                color: slider_visual.track_background,
                border: slider_visual
                    .track_border
                    .map(|color| crate::renderer::Border { width: 1.0, color }),
                radius: [tokens.track_radius; 4],
                shadow: None,
            },
            slider_visual.text,
        ));
    }

    if node_id_str.ends_with("::fill") {
        let slider_visual = theme.slider_visual(root_visual);
        let tokens = theme.components.slider;
        return Some((
            RectStyle {
                color: slider_visual.fill,
                border: None,
                radius: [tokens.track_radius; 4],
                shadow: None,
            },
            slider_visual.text,
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
