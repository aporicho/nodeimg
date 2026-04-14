use super::layout::LeafKind;
use super::node::{NodeId, NodeKind};
use super::paint_helpers::{
    bezier_control_points, find_node_by_str_id, grid_cells, rect_center_left, rect_center_right,
    PaintTransform,
};
use super::tree::Tree;
use crate::renderer::{Color, Point, RectStyle, Renderer, TextStyle};
use crate::theme::Theme;
use crate::widget::atoms::button::ButtonProps;
use crate::widget::atoms::slider::SliderProps;
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::atoms::toggle::ToggleProps;
use crate::widget::state::{InteractionStore, TextInputStore, WidgetVisualState};

/// 连线宽度（local 空间像素，paint 时按 scale 缩放）
const CONNECTION_WIDTH: f32 = 2.0;
pub fn paint(
    tree: &Tree,
    root: NodeId,
    renderer: &mut Renderer,
    interaction: Option<&InteractionStore>,
    text_inputs: Option<&TextInputStore>,
    theme: &Theme,
) {
    paint_node(
        tree,
        root,
        renderer,
        PaintTransform::identity(),
        interaction,
        text_inputs,
        theme,
        None,
    );
}

fn paint_node(
    tree: &Tree,
    node_id: NodeId,
    renderer: &mut Renderer,
    tf: PaintTransform,
    interaction: Option<&InteractionStore>,
    text_inputs: Option<&TextInputStore>,
    theme: &Theme,
    inherited_text_color: Option<Color>,
) {
    let Some(node) = tree.get(node_id) else {
        return;
    };

    let screen_rect = tf.apply_rect(node.rect);
    let mut child_text_color = inherited_text_color;

    if let Some((style, text_color)) = widget_visual_override(tree, node_id, interaction, theme) {
        renderer.draw_rect(screen_rect, &style);
        child_text_color = Some(text_color);
    } else if let Some(dec) = &node.decoration {
        renderer.draw_rect(
            screen_rect,
            &RectStyle {
                color: dec.background.unwrap_or(Color::TRANSPARENT),
                border: dec.border,
                radius: dec.radius,
                shadow: dec.shadow,
            },
        );
    }

    // 2. Leaf 分发
    if let NodeKind::Leaf(leaf) = &node.kind {
        match leaf {
            LeafKind::Text {
                content,
                font_size,
                color,
            } => {
                if paint_text_input_value_leaf(
                    tree,
                    node_id,
                    renderer,
                    tf,
                    interaction,
                    text_inputs,
                    theme,
                    content,
                    *font_size,
                    child_text_color.unwrap_or(*color),
                    screen_rect,
                ) {
                    return;
                }
                if let Some(selection_rect) =
                    text_input_selection_rect(tree, node_id, tf, interaction, text_inputs)
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
                renderer.draw_text(
                    Point {
                        x: screen_rect.x,
                        y: screen_rect.y,
                    },
                    content,
                    &TextStyle {
                        color: child_text_color.unwrap_or(*color),
                        size: *font_size * tf.scale,
                    },
                );
                if let Some(caret_rect) =
                    text_input_caret_rect(tree, node_id, tf, interaction, text_inputs)
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
            }
            LeafKind::Grid {
                spacing,
                dot_color,
                dot_size,
            } => {
                for p in grid_cells(node.rect, *spacing) {
                    let sp = tf.apply_point(p);
                    renderer.draw_circle(sp, *dot_size * tf.scale, *dot_color);
                }
            }
            LeafKind::Connection { from_port, to_port } => {
                let Some(from_rect) = find_node_by_str_id(tree, from_port.as_ref()) else {
                    return;
                };
                let Some(to_rect) = find_node_by_str_id(tree, to_port.as_ref()) else {
                    return;
                };
                let from_p = tf.apply_point(rect_center_right(from_rect));
                let to_p = tf.apply_point(rect_center_left(to_rect));
                let ctrl = bezier_control_points(from_p, to_p);
                renderer.draw_curve(ctrl, CONNECTION_WIDTH * tf.scale, theme.colors.connection);
            }
            _ => {}
        }
    }

    // 3. 复合 Transform 并递归子节点
    let transform_opt = node.style.transform;
    let children: Vec<NodeId> = node.children.clone();
    // node 借用在此处结束（NLL），后面可以重新借 tree
    let child_tf = match transform_opt {
        Some(ref tf_decl) => tf.compose(tf_decl),
        None => tf,
    };
    for child_id in children {
        paint_node(
            tree,
            child_id,
            renderer,
            child_tf,
            interaction,
            text_inputs,
            theme,
            child_text_color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_text_input_value_leaf(
    tree: &Tree,
    node_id: NodeId,
    renderer: &mut Renderer,
    tf: PaintTransform,
    interaction: Option<&InteractionStore>,
    text_inputs: Option<&TextInputStore>,
    theme: &Theme,
    content: &str,
    font_size: f32,
    text_color: Color,
    _screen_rect: crate::renderer::Rect,
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
                &TextStyle {
                    color: text_color,
                    size: font_size * tf.scale,
                },
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
                &TextStyle {
                    color: text_color,
                    size: font_size * tf.scale,
                },
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
                &TextStyle {
                    color: text_color,
                    size: font_size * tf.scale,
                },
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
            &TextStyle {
                color: text_color,
                size: font_size * tf.scale,
            },
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

fn widget_visual_override(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionStore>,
    theme: &Theme,
) -> Option<(RectStyle, Color)> {
    let node = tree.get(node_id)?;
    button_visual_override(node_id, &node.kind, interaction, theme)
        .or_else(|| toggle_visual_override(tree, node_id, interaction, theme))
        .or_else(|| slider_visual_override(tree, node_id, interaction, theme))
        .or_else(|| text_input_visual_override(tree, node_id, interaction, theme))
}

fn button_visual_override(
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

fn toggle_visual_override(
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

fn slider_visual_override(
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

fn text_input_visual_override(
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
    let text_input = props.as_any().downcast_ref::<TextInputProps>()?;
    let root_visual = interaction
        .map(|state| state.visual_state(root_node_id, text_input.disabled))
        .unwrap_or(if text_input.disabled {
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
        let tokens = theme.components.text_input;

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

fn text_input_selection_rect(
    tree: &Tree,
    node_id: NodeId,
    tf: PaintTransform,
    interaction: Option<&InteractionStore>,
    text_inputs: Option<&TextInputStore>,
) -> Option<crate::renderer::Rect> {
    let (widget_id, focused) = text_input_widget_id(tree, node_id, interaction)?;
    if !focused {
        return None;
    }
    let runtime = text_inputs?.runtime(widget_id.as_ref())?;
    Some(tf.apply_rect(runtime.selection_rect()?))
}

fn text_input_caret_rect(
    tree: &Tree,
    node_id: NodeId,
    tf: PaintTransform,
    interaction: Option<&InteractionStore>,
    text_inputs: Option<&TextInputStore>,
) -> Option<crate::renderer::Rect> {
    let (widget_id, focused) = text_input_widget_id(tree, node_id, interaction)?;
    if !focused {
        return None;
    }
    let runtime = text_inputs?.runtime(widget_id.as_ref())?;
    Some(tf.apply_rect(runtime.caret_rect()))
}

fn text_input_widget_id(
    tree: &Tree,
    node_id: NodeId,
    interaction: Option<&InteractionStore>,
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
    props.as_any().downcast_ref::<TextInputProps>()?;
    let focused = interaction
        .map(|state| state.focused() == Some(root_node_id))
        .unwrap_or(false);
    Some((root_id, focused))
}

fn transparent_style() -> RectStyle {
    RectStyle {
        color: Color::TRANSPARENT,
        border: None,
        radius: [0.0; 4],
        shadow: None,
    }
}
