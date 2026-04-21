use super::{
    canvas_node_stable_id, CanvasNodeLayout, CanvasPortConnectionState, CanvasPortGroupView,
    CanvasPortSide, CanvasPortView,
};
use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Position, Size,
};
use crate::tree::Desc;
use std::borrow::Cow;

const PIN_SIZE: f32 = 32.0;
const PIN_ROW_HEIGHT: f32 = 32.0;
const PIN_ROW_GAP: f32 = 12.0;
const PIN_LABEL_GAP: f32 = 10.0;
const PIN_COLUMN_WIDTH: f32 = 120.0;
const PIN_COLUMN_GAP: f32 = 24.0;
const NODE_PADDING: f32 = 24.0;
const NODE_LABEL_TOP: f32 = -37.0;
const NODE_LABEL_HEIGHT: f32 = 32.0;
const NODE_ROW_HEIGHT: f32 = 36.0;
const NODE_ROW_GAP: f32 = 12.0;

#[derive(Clone, Debug)]
pub struct CanvasNodeView {
    pub owner_id: String,
    pub title: String,
    pub subtitle: String,
    pub category: String,
    pub params: Vec<CanvasNodeParamView>,
    pub input_group: CanvasPortGroupView,
    pub output_group: CanvasPortGroupView,
    pub inputs: Vec<CanvasPortView>,
    pub outputs: Vec<CanvasPortView>,
    pub selected: bool,
    pub layout: CanvasNodeLayout,
}

#[derive(Clone, Debug)]
pub struct CanvasNodeParamView {
    pub name: String,
    pub kind: String,
    pub value: String,
}

pub fn node_card(view: &CanvasNodeView, theme: &Theme) -> Desc {
    let card_id = canvas_node_stable_id(&view.owner_id);
    let label_id = format!("{card_id}::label");
    let label_dot_id = format!("{card_id}::label_dot");
    let label_text_id = format!("{card_id}::label_text");
    let card_body_id = format!("{card_id}::card");
    let body_id = format!("{card_id}::body");
    let card_x = PIN_COLUMN_WIDTH + PIN_COLUMN_GAP;

    let children = vec![
        pin_column(
            &view.owner_id,
            CanvasPortSide::Input,
            0.0,
            &view.inputs,
            theme,
        ),
        Desc::Container {
            id: Cow::Owned(card_body_id),
            style: BoxStyle {
                position: Position::Absolute { x: card_x, y: 0.0 },
                width: Size::Fixed(view.layout.rect.w),
                height: Size::Fixed(card_height(view)),
                padding: Edges::all(NODE_PADDING),
                direction: Direction::Column,
                gap: NODE_ROW_GAP,
                hittable: Some(true),
                ..BoxStyle::default()
            },
            decoration: Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: if view.selected { 2.0 } else { 1.0 },
                    color: if view.selected {
                        theme.colors.text
                    } else {
                        theme.colors.border
                    },
                }),
                radius: [24.0; 4],
                shadow: None,
            }),
            children: vec![
                node_body(&body_id, view, theme),
                Desc::Container {
                    id: Cow::Owned(label_id),
                    style: BoxStyle {
                        position: Position::Absolute {
                            x: 0.0,
                            y: NODE_LABEL_TOP,
                        },
                        width: Size::Auto,
                        height: Size::Fixed(NODE_LABEL_HEIGHT),
                        direction: Direction::Row,
                        align_items: Align::Center,
                        gap: PIN_LABEL_GAP,
                        hittable: Some(false),
                        ..BoxStyle::default()
                    },
                    decoration: None,
                    children: vec![
                        Desc::Leaf {
                            id: Cow::Owned(label_dot_id),
                            style: BoxStyle {
                                width: Size::Fixed(PIN_SIZE),
                                height: Size::Fixed(PIN_SIZE),
                                ..BoxStyle::default()
                            },
                            kind: LeafKind::Circle {
                                radius: PIN_SIZE * 0.5,
                                fill: Some(category_color(&view.category)),
                                stroke: None,
                            },
                        },
                        Desc::Leaf {
                            id: Cow::Owned(label_text_id),
                            style: BoxStyle {
                                width: Size::Auto,
                                height: Size::Auto,
                                ..BoxStyle::default()
                            },
                            kind: LeafKind::Text {
                                content: view.title.clone(),
                                style: theme.text_style_label_sm(),
                            },
                        },
                    ],
                },
            ],
        },
        pin_column(
            &view.owner_id,
            CanvasPortSide::Output,
            card_x + view.layout.rect.w + PIN_COLUMN_GAP,
            &view.outputs,
            theme,
        ),
    ];

    Desc::Container {
        id: Cow::Owned(card_id),
        style: BoxStyle {
            position: Position::Absolute {
                x: view.layout.rect.x - card_x,
                y: view.layout.rect.y,
            },
            width: Size::Fixed(card_x + view.layout.rect.w + PIN_COLUMN_GAP + PIN_COLUMN_WIDTH),
            height: Size::Fixed(outer_height(view)),
            padding: Edges::all(0.0),
            gap: 0.0,
            direction: Direction::Row,
            hittable: Some(true),
            gestures: vec![Gesture::Tap, Gesture::Drag],
            ..BoxStyle::default()
        },
        decoration: None,
        children,
    }
}

fn pin_column(
    owner_id: &str,
    side: CanvasPortSide,
    x: f32,
    ports: &[CanvasPortView],
    theme: &Theme,
) -> Desc {
    Desc::Container {
        id: Cow::Owned(format!(
            "canvas_node::{owner_id}::pin_column::{}",
            side.as_str()
        )),
        style: BoxStyle {
            position: Position::Absolute { x, y: 0.0 },
            width: Size::Fixed(PIN_COLUMN_WIDTH),
            height: Size::Auto,
            direction: Direction::Column,
            gap: PIN_ROW_GAP,
            hittable: Some(false),
            ..BoxStyle::default()
        },
        decoration: None,
        children: ports
            .iter()
            .map(|port| pin_row(side, port, theme))
            .collect(),
    }
}

fn pin_row(side: CanvasPortSide, port: &CanvasPortView, theme: &Theme) -> Desc {
    let mut children = vec![pin_dot(port, theme), pin_label(port, theme)];
    if side == CanvasPortSide::Output {
        children.swap(0, 1);
    }

    Desc::Container {
        id: Cow::Owned(format!("{}::pin_item", port.stable_id)),
        style: BoxStyle {
            width: Size::Fixed(PIN_COLUMN_WIDTH),
            height: Size::Fixed(PIN_ROW_HEIGHT),
            direction: Direction::Row,
            align_items: Align::Center,
            gap: PIN_LABEL_GAP,
            hittable: Some(true),
            gestures: vec![Gesture::Tap, Gesture::Drag],
            ..BoxStyle::default()
        },
        decoration: None,
        children,
    }
}

fn pin_dot(port: &CanvasPortView, theme: &Theme) -> Desc {
    Desc::Leaf {
        id: Cow::Owned(port.stable_id.clone()),
        style: BoxStyle {
            width: Size::Fixed(PIN_SIZE),
            height: Size::Fixed(PIN_SIZE),
            hittable: Some(true),
            gestures: vec![Gesture::Tap, Gesture::Drag],
            ..BoxStyle::default()
        },
        kind: LeafKind::Circle {
            radius: PIN_SIZE * 0.5,
            fill: Some(port_state_color(port, theme)),
            stroke: Some(Border {
                width: match port.connection_state {
                    CanvasPortConnectionState::Idle => 1.0,
                    CanvasPortConnectionState::Source
                    | CanvasPortConnectionState::CompatibleTarget
                    | CanvasPortConnectionState::IncompatibleTarget
                    | CanvasPortConnectionState::DropTarget
                    | CanvasPortConnectionState::RejectedDropTarget => 2.0,
                },
                color: port_border_color(port.connection_state, theme),
            }),
        },
    }
}

fn pin_label(port: &CanvasPortView, theme: &Theme) -> Desc {
    Desc::Leaf {
        id: Cow::Owned(format!("{}::pin_label", port.stable_id)),
        style: BoxStyle {
            width: Size::Fill,
            height: Size::Auto,
            ..BoxStyle::default()
        },
        kind: LeafKind::Text {
            content: port.name.clone(),
            style: theme.text_style_label_sm(),
        },
    }
}

fn node_body(id: &str, view: &CanvasNodeView, theme: &Theme) -> Desc {
    let mut children = Vec::new();
    if view.params.is_empty() {
        children.push(Desc::Leaf {
            id: Cow::Owned(format!("{id}::summary")),
            style: BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            kind: LeafKind::Text {
                content: view.subtitle.clone(),
                style: theme.text_style_label_sm(),
            },
        });
    } else {
        children.extend(
            view.params
                .iter()
                .take(5)
                .enumerate()
                .map(|(index, param)| param_row(id, index, param, theme)),
        );
    }

    Desc::Container {
        id: Cow::Owned(id.to_string()),
        style: BoxStyle {
            width: Size::Fill,
            height: Size::Auto,
            direction: Direction::Column,
            gap: NODE_ROW_GAP,
            ..BoxStyle::default()
        },
        decoration: None,
        children,
    }
}

fn param_row(id: &str, index: usize, param: &CanvasNodeParamView, theme: &Theme) -> Desc {
    Desc::Container {
        id: Cow::Owned(format!("{id}::param::{index}")),
        style: BoxStyle {
            width: Size::Fill,
            height: Size::Fixed(NODE_ROW_HEIGHT),
            direction: Direction::Row,
            align_items: Align::Center,
            gap: PIN_LABEL_GAP,
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(theme.colors.canvas_bg),
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }),
        children: vec![
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::param::{index}::name")),
                style: BoxStyle {
                    width: Size::Fill,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: param.name.clone(),
                    style: theme.text_style_label_sm(),
                },
            },
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::param::{index}::value")),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: if param.value.is_empty() {
                        param.kind.clone()
                    } else {
                        param.value.clone()
                    },
                    style: theme.text_style_label_sm(),
                },
            },
        ],
    }
}

fn card_height(view: &CanvasNodeView) -> f32 {
    if view.layout.collapsed {
        48.0
    } else {
        view.layout.rect.h
    }
}

fn outer_height(view: &CanvasNodeView) -> f32 {
    card_height(view).max(pin_column_height(view.inputs.len().max(view.outputs.len())))
}

fn pin_column_height(count: usize) -> f32 {
    if count == 0 {
        0.0
    } else {
        count as f32 * PIN_ROW_HEIGHT + count.saturating_sub(1) as f32 * PIN_ROW_GAP
    }
}

fn port_state_color(port: &CanvasPortView, theme: &Theme) -> Color {
    match port.connection_state {
        CanvasPortConnectionState::CompatibleTarget => theme.colors.accent,
        CanvasPortConnectionState::DropTarget => theme.colors.accent,
        CanvasPortConnectionState::IncompatibleTarget => incompatible_port_color(),
        CanvasPortConnectionState::RejectedDropTarget => incompatible_port_color(),
        CanvasPortConnectionState::Source | CanvasPortConnectionState::Idle => {
            port_color(port.side, theme)
        }
    }
}

fn port_border_color(state: CanvasPortConnectionState, theme: &Theme) -> Color {
    match state {
        CanvasPortConnectionState::CompatibleTarget => theme.colors.accent,
        CanvasPortConnectionState::DropTarget => theme.colors.accent,
        CanvasPortConnectionState::IncompatibleTarget => incompatible_port_color(),
        CanvasPortConnectionState::RejectedDropTarget => incompatible_port_color(),
        CanvasPortConnectionState::Source => theme.colors.text,
        CanvasPortConnectionState::Idle => theme.colors.surface,
    }
}

fn incompatible_port_color() -> Color {
    Color {
        r: 0.863,
        g: 0.149,
        b: 0.149,
        a: 1.0,
    }
}

fn port_color(side: CanvasPortSide, theme: &Theme) -> Color {
    match side {
        CanvasPortSide::Input => theme.colors.text_muted,
        CanvasPortSide::Output => theme.colors.accent,
    }
}

fn category_color(category: &str) -> Color {
    match category.split('/').next().unwrap_or(category) {
        "image" => Color {
            r: 0.976,
            g: 0.451,
            b: 0.086,
            a: 1.0,
        },
        "ai" => Color {
            r: 0.741,
            g: 0.094,
            b: 0.365,
            a: 1.0,
        },
        "io" => Color {
            r: 0.235,
            g: 0.510,
            b: 0.965,
            a: 1.0,
        },
        _ => Color {
            r: 0.388,
            g: 0.400,
            b: 0.945,
            a: 1.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::theme::light_theme;

    #[test]
    fn node_card_uses_canvas_node_stable_id() {
        let view = CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            category: "image/generation".to_string(),
            params: Vec::new(),
            input_group: CanvasPortGroupView::default(),
            output_group: CanvasPortGroupView::default(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            selected: false,
            layout: CanvasNodeLayout {
                owner_id: "engine_node::7".to_string(),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 220.0,
                    h: 96.0,
                },
                z_index: 0,
                collapsed: false,
            },
        };

        assert_eq!(
            node_card(&view, &light_theme()).id(),
            "canvas_node::engine_node::7"
        );
    }

    #[test]
    fn node_card_declares_drag_gesture() {
        let view = CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            category: "image/generation".to_string(),
            params: Vec::new(),
            input_group: CanvasPortGroupView::default(),
            output_group: CanvasPortGroupView::default(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            selected: false,
            layout: CanvasNodeLayout {
                owner_id: "engine_node::7".to_string(),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 220.0,
                    h: 96.0,
                },
                z_index: 0,
                collapsed: false,
            },
        };

        let Desc::Container { style, .. } = node_card(&view, &light_theme()) else {
            panic!("node card should build a container");
        };
        assert!(style.gestures.contains(&Gesture::Drag));
    }

    #[test]
    fn node_card_builds_port_leaves() {
        let view = CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            category: "image/generation".to_string(),
            params: vec![CanvasNodeParamView {
                name: "prompt".to_string(),
                kind: "string".to_string(),
                value: "text".to_string(),
            }],
            input_group: CanvasPortGroupView { open: true },
            output_group: CanvasPortGroupView::default(),
            inputs: vec![CanvasPortView {
                name: "prompt".to_string(),
                stable_id: "canvas_node::engine_node::7::port::input::prompt".to_string(),
                side: CanvasPortSide::Input,
                index: 0,
                count: 1,
                connection_state: CanvasPortConnectionState::Idle,
            }],
            outputs: vec![CanvasPortView {
                name: "image".to_string(),
                stable_id: "canvas_node::engine_node::7::port::output::image".to_string(),
                side: CanvasPortSide::Output,
                index: 0,
                count: 1,
                connection_state: CanvasPortConnectionState::Idle,
            }],
            selected: false,
            layout: CanvasNodeLayout {
                owner_id: "engine_node::7".to_string(),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 220.0,
                    h: 96.0,
                },
                z_index: 0,
                collapsed: false,
            },
        };

        let Desc::Container { children, .. } = node_card(&view, &light_theme()) else {
            panic!("node card should build a container");
        };

        assert!(children
            .iter()
            .any(|child| child.id() == "canvas_node::engine_node::7::pin_column::input"));
        assert!(children
            .iter()
            .any(|child| child.id() == "canvas_node::engine_node::7::pin_column::output"));
        assert!(children.iter().any(|child| contains_desc_id(
            child,
            "canvas_node::engine_node::7::port::input::prompt"
        )));
        assert!(children.iter().any(|child| contains_desc_id(
            child,
            "canvas_node::engine_node::7::port::output::image"
        )));
    }

    #[test]
    fn node_card_builds_function_label_and_params() {
        let view = CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            category: "image/generation".to_string(),
            params: vec![CanvasNodeParamView {
                name: "prompt".to_string(),
                kind: "string".to_string(),
                value: "text".to_string(),
            }],
            input_group: CanvasPortGroupView::default(),
            output_group: CanvasPortGroupView::default(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            selected: false,
            layout: CanvasNodeLayout {
                owner_id: "engine_node::7".to_string(),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 220.0,
                    h: 96.0,
                },
                z_index: 0,
                collapsed: false,
            },
        };

        let Desc::Container { children, .. } = node_card(&view, &light_theme()) else {
            panic!("node card should build a container");
        };

        assert!(children
            .iter()
            .any(|child| contains_desc_id(child, "canvas_node::engine_node::7::label")));
        assert!(children
            .iter()
            .any(|child| contains_desc_id(child, "canvas_node::engine_node::7::body")));
    }

    fn contains_desc_id(desc: &Desc, id: &str) -> bool {
        if desc.id() == id {
            return true;
        }
        match desc {
            Desc::Container { children, .. } => {
                children.iter().any(|child| contains_desc_id(child, id))
            }
            Desc::Leaf { .. } | Desc::Widget { .. } => false,
        }
    }
}
