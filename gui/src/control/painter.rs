use crate::control::state::{TextBoxRuntime, TextBoxStore};
use crate::interaction::InteractionState;
use crate::paint::ClipShape;
use crate::renderer::{Point, Rect, RectStyle, TextStyle};
use crate::theme::Theme;
use crate::tree::paint_target::PaintTarget;
use crate::tree::{NodeId, Tree};

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_text_leaf_override(
    tree: &Tree,
    node_id: NodeId,
    target: &mut dyn PaintTarget,
    node_rect: Rect,
    interaction: Option<&InteractionState>,
    text_boxes: Option<&TextBoxStore>,
    theme: &Theme,
    text_style: &TextStyle,
) -> bool {
    let Some((control_id, focused)) = text_box_control_id(tree, node_id, interaction) else {
        return false;
    };
    let Some(runtime) = text_boxes.and_then(|store| store.text_box(control_id.as_ref())) else {
        tracing::trace!(
            target: "gui::control::text_box",
            control_id = %control_id,
            has_store = text_boxes.is_some(),
            "skip text box paint leaf: runtime missing"
        );
        return false;
    };
    let clip_rect = rect_to_node_local(runtime.clip_rect(), node_rect);

    target.push_clip(ClipShape::Rect(clip_rect));

    if focused {
        for rect in runtime
            .selection_rects()
            .into_iter()
            .map(|rect| rect_to_node_local(rect, node_rect))
        {
            target.draw_rect(
                rect,
                RectStyle {
                    color: theme.selection_color(),
                    border: None,
                    radius: [theme.components.text_input.selection_radius; 4],
                    shadow: None,
                },
            );
        }
    }

    if runtime.has_preedit() && !runtime.is_multiline() {
        paint_single_line_preedit(runtime, target, node_rect, clip_rect, text_style, theme);
    } else {
        paint_runtime_text(runtime, target, node_rect, clip_rect, text_style);
        if let Some((point, preedit_text)) = runtime.preedit_origin_and_text() {
            target.draw_text_clipped(
                Point {
                    x: point.x - node_rect.x,
                    y: point.y - node_rect.y,
                },
                preedit_text,
                *text_style,
                clip_rect,
            );
        }
    }

    if focused {
        let caret = rect_to_node_local(runtime.caret_rect(), node_rect);
        target.draw_rect(
            caret,
            RectStyle {
                color: theme.caret_color(),
                border: None,
                radius: [0.0; 4],
                shadow: None,
            },
        );
    }

    target.pop_clip();
    true
}

fn paint_runtime_text(
    runtime: &TextBoxRuntime,
    target: &mut dyn PaintTarget,
    node_rect: Rect,
    clip_rect: Rect,
    text_style: &TextStyle,
) {
    for line in &runtime.layout().lines {
        let content = &runtime.editor().text()[line.start..line.end];
        if content.is_empty() {
            continue;
        }
        let origin = runtime.visible_line_origin(line.y);
        target.draw_text_clipped(
            Point {
                x: origin.x - node_rect.x,
                y: origin.y - node_rect.y,
            },
            content,
            *text_style,
            clip_rect,
        );
    }
}

fn paint_single_line_preedit(
    runtime: &TextBoxRuntime,
    target: &mut dyn PaintTarget,
    node_rect: Rect,
    clip_rect: Rect,
    text_style: &TextStyle,
    theme: &Theme,
) {
    let content = runtime.editor().text();
    let Some((start, end)) = runtime.preedit_range() else {
        return;
    };
    let Some(preedit_text) = runtime.preedit_text() else {
        return;
    };
    let start = clamp_text_index(content, start);
    let end = clamp_text_index(content, end).max(start);
    let prefix = &content[..start];
    let suffix = &content[end..];
    let text_origin = runtime.text_draw_origin();
    let local_text_origin = point_to_node_local(text_origin, node_rect);
    let preedit_x = runtime.preedit_start_x().unwrap_or(text_origin.x) - node_rect.x;
    let suffix_x = preedit_x + runtime.preedit_width().unwrap_or(0.0);

    if !prefix.is_empty() {
        target.draw_text_clipped(local_text_origin, prefix, *text_style, clip_rect);
    }
    if !preedit_text.is_empty() {
        target.draw_text_clipped(
            Point {
                x: preedit_x,
                y: local_text_origin.y,
            },
            preedit_text,
            *text_style,
            clip_rect,
        );
    }
    if !suffix.is_empty() {
        target.draw_text_clipped(
            Point {
                x: suffix_x,
                y: local_text_origin.y,
            },
            suffix,
            *text_style,
            clip_rect,
        );
    }
    if let Some(underline_rect) = runtime
        .preedit_underline_rect()
        .map(|rect| rect_to_node_local(rect, node_rect))
    {
        target.draw_rect(
            underline_rect,
            RectStyle {
                color: theme.preedit_underline_color(),
                border: None,
                radius: [0.0; 4],
                shadow: None,
            },
        );
    }
}

fn text_box_control_id(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionState>,
) -> Option<(String, bool)> {
    let node = tree.get(node_id)?;
    let node_id_str = node.id.as_ref();
    if !node_id_str.ends_with("::value") {
        return None;
    }
    let control_id = retained_text_box_owner(tree, node_id_str)?;
    let root_node_id = tree.node_by_str(control_id)?;
    let focused = interaction
        .map(|state| state.focused() == Some(root_node_id))
        .unwrap_or(false);
    Some((control_id.to_string(), focused))
}

fn retained_text_box_owner<'a>(tree: &Tree, node_id: &'a str) -> Option<&'a str> {
    for candidate in part_candidates(node_id) {
        let node = tree.get(tree.node_by_str(candidate)?)?;
        if node.props.semantic_role.is_some_and(|role| {
            role.is_text_input()
                || role.is_text_area()
                || matches!(role, crate::control::ControlRole::NumberInput)
        }) {
            return Some(candidate);
        }
    }
    None
}

fn part_candidates(node_id: &str) -> impl Iterator<Item = &str> {
    [
        Some(node_id),
        node_id.strip_suffix("::field"),
        node_id.strip_suffix("::value"),
    ]
    .into_iter()
    .flatten()
}

fn clamp_text_index(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn point_to_node_local(point: Point, node_rect: Rect) -> Point {
    Point {
        x: point.x - node_rect.x,
        y: point.y - node_rect.y,
    }
}

fn rect_to_node_local(rect: Rect, node_rect: Rect) -> Rect {
    Rect {
        x: rect.x - node_rect.x,
        y: rect.y - node_rect.y,
        w: rect.w,
        h: rect.h,
    }
}
