use crate::interaction::{InteractionState, WidgetVisualState};
use crate::renderer::{Color, Point, RectStyle, Renderer, TextStyle};
use crate::theme::{TextInputTheme, Theme};
use crate::tree::paint_helpers::PaintTransform;
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::number_input::NumberInputProps;
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::state::TextInputStore;

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
    let (disabled, tokens) = text_field_visual_spec(props.as_ref(), theme)?;
    let root_visual = interaction
        .map(|state| state.visual_state(root_node_id, disabled))
        .unwrap_or(if disabled {
            WidgetVisualState::Disabled
        } else {
            WidgetVisualState::Normal
        });

    if node_id_str == root_id {
        return Some((
            transparent_style(),
            theme.text_color_for_visual(root_visual),
        ));
    }

    if node_id_str.ends_with("::field") {
        let field_visual = theme.text_input_visual(root_visual);

        return Some((
            RectStyle {
                color: field_visual.background,
                border: field_visual.border.map(|color| crate::renderer::Border {
                    width: tokens.border_width,
                    color,
                }),
                radius: [tokens.radius; 4],
                shadow: None,
            },
            field_visual.text,
        ));
    }

    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_text_leaf(
    tree: &Tree,
    node_id: NodeId,
    renderer: &mut Renderer,
    tf: PaintTransform,
    interaction: Option<&InteractionState>,
    text_inputs: Option<&TextInputStore>,
    theme: &Theme,
    content: &str,
    text_style: &TextStyle,
) -> bool {
    let Some((widget_id, focused)) = text_input_widget_id(tree, node_id, interaction) else {
        return false;
    };
    let Some(runtime) = text_inputs.and_then(|store| store.runtime(widget_id.as_ref())) else {
        return false;
    };
    let clip_rect = tf.apply_rect(runtime.clip_rect());

    renderer.push_clip(clip_rect, 0.0);

    if let Some(selection_rect) = focused
        .then(|| runtime.selection_rect())
        .flatten()
        .map(|rect| tf.apply_rect(rect))
    {
        renderer.draw_rect(
            selection_rect,
            &RectStyle {
                color: theme.selection_color(),
                border: None,
                radius: [theme.components.text_input.selection_radius; 4],
                shadow: None,
            },
        );
    }

    if runtime.has_preedit() {
        let Some((start, end)) = runtime.preedit_range() else {
            return false;
        };
        let Some(preedit_text) = runtime.preedit_text() else {
            renderer.pop_clip();
            return false;
        };
        let prefix = &content[..start];
        let suffix = &content[end..];
        let text_origin = runtime.text_draw_origin();
        let screen_text_origin = tf.apply_point(text_origin);
        let preedit_x = tf
            .apply_point(Point {
                x: runtime.preedit_start_x().unwrap_or(text_origin.x),
                y: text_origin.y,
            })
            .x;
        let suffix_x = preedit_x + runtime.preedit_width().unwrap_or(0.0) * tf.scale;

        if !prefix.is_empty() {
            renderer.draw_text_clipped(
                screen_text_origin,
                prefix,
                &scaled_text_style(*text_style, tf.scale),
                clip_rect,
            );
        }

        if !preedit_text.is_empty() {
            renderer.draw_text_clipped(
                Point {
                    x: preedit_x,
                    y: screen_text_origin.y,
                },
                preedit_text,
                &scaled_text_style(*text_style, tf.scale),
                clip_rect,
            );
        }

        if !suffix.is_empty() {
            renderer.draw_text_clipped(
                Point {
                    x: suffix_x,
                    y: screen_text_origin.y,
                },
                suffix,
                &scaled_text_style(*text_style, tf.scale),
                clip_rect,
            );
        }

        if let Some(underline_rect) = runtime
            .preedit_underline_rect()
            .map(|rect| tf.apply_rect(rect))
        {
            renderer.draw_rect(
                underline_rect,
                &RectStyle {
                    color: theme.preedit_underline_color(),
                    border: None,
                    radius: [0.0; 4],
                    shadow: None,
                },
            );
        }
    } else {
        let text_origin = runtime.text_draw_origin();
        let screen_text_origin = tf.apply_point(text_origin);
        renderer.draw_text_clipped(
            screen_text_origin,
            content,
            &scaled_text_style(*text_style, tf.scale),
            clip_rect,
        );
    }

    if let Some(caret_rect) = focused
        .then(|| runtime.caret_rect())
        .map(|rect| tf.apply_rect(rect))
    {
        renderer.draw_rect(
            caret_rect,
            &RectStyle {
                color: theme.caret_color(),
                border: None,
                radius: [0.0; 4],
                shadow: None,
            },
        );
    }

    renderer.pop_clip();

    true
}

fn text_input_widget_id(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionState>,
) -> Option<(String, bool)> {
    let node = tree.get(node_id)?;
    let node_id_str = node.id.as_ref();
    if !node_id_str.ends_with("::value") {
        return None;
    }

    let root_id = node_id_str.split("::").next()?.to_string();
    let (root_node_id, root_node) = tree.iter().find(|(_, n)| n.id.as_ref() == root_id)?;
    let NodeKind::Widget(props) = &root_node.kind else {
        return None;
    };
    is_text_field_props(props.as_ref())?;
    let focused = interaction
        .map(|state| state.focused() == Some(root_node_id))
        .unwrap_or(false);
    Some((root_id, focused))
}

fn is_text_field_props(props: &dyn crate::widget::props::WidgetProps) -> Option<()> {
    (props.as_any().downcast_ref::<TextInputProps>().is_some()
        || props.as_any().downcast_ref::<NumberInputProps>().is_some())
    .then_some(())
}

fn scaled_text_style(mut style: TextStyle, scale: f32) -> TextStyle {
    style.size *= scale;
    style
}

fn text_field_visual_spec(
    props: &dyn crate::widget::props::WidgetProps,
    theme: &Theme,
) -> Option<(bool, TextInputTheme)> {
    if let Some(text_input) = props.as_any().downcast_ref::<TextInputProps>() {
        return Some((text_input.disabled, theme.components.text_input));
    }

    props
        .as_any()
        .downcast_ref::<NumberInputProps>()
        .map(|number_input| (number_input.disabled, theme.components.number_input))
}
