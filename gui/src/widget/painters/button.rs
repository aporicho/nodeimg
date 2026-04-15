use crate::renderer::{Color, RectStyle};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind};
use crate::widget::atoms::button::ButtonProps;
use crate::widget::state::{InteractionStore, WidgetVisualState};

pub(super) fn visual_override(
    node_id: NodeId,
    kind: &NodeKind,
    interaction: Option<&InteractionStore>,
    theme: &Theme,
) -> Option<(RectStyle, Color)> {
    let NodeKind::Widget(props) = kind else {
        return None;
    };
    let button = props.as_any().downcast_ref::<ButtonProps>()?;
    let visual = interaction
        .map(|state| state.visual_state(node_id, button.disabled))
        .unwrap_or(if button.disabled {
            WidgetVisualState::Disabled
        } else {
            WidgetVisualState::Normal
        });

    let button_visual = theme.button_visual(visual);
    let tokens = theme.components.button;

    Some((
        RectStyle {
            color: button_visual.background,
            border: button_visual.border.map(|color| crate::renderer::Border {
                width: tokens.border_width,
                color,
            }),
            radius: [tokens.radius; 4],
            shadow: None,
        },
        button_visual.text,
    ))
}
