use super::{
    canvas_node_stable_id, canvas_port_group_trigger_id, CanvasNodeLayout, CanvasPortGroupView,
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

const PORT_SIZE: f32 = 8.0;
const PORT_TOP: f32 = 34.0;
const PORT_GAP: f32 = 18.0;
const NODE_LABEL_HEIGHT: f32 = 16.0;
const PORT_TRIGGER_SIZE: f32 = 20.0;

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
    let body_id = format!("{card_id}::body");

    let mut children = vec![
        Desc::Container {
            id: Cow::Owned(label_id),
            style: BoxStyle {
                position: Position::Absolute { x: 2.0, y: -20.0 },
                width: Size::Fixed(view.layout.rect.w),
                height: Size::Fixed(NODE_LABEL_HEIGHT),
                direction: Direction::Row,
                align_items: Align::Center,
                gap: 5.0,
                hittable: Some(false),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![
                Desc::Leaf {
                    id: Cow::Owned(label_dot_id),
                    style: BoxStyle {
                        width: Size::Fixed(6.0),
                        height: Size::Fixed(6.0),
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Circle {
                        radius: 3.0,
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
        node_body(&body_id, view, theme),
    ];
    children.push(port_group(
        &view.owner_id,
        CanvasPortSide::Input,
        &view.input_group,
        &view.inputs,
        view,
        theme,
    ));
    children.push(port_group(
        &view.owner_id,
        CanvasPortSide::Output,
        &view.output_group,
        &view.outputs,
        view,
        theme,
    ));
    children.extend(
        view.inputs
            .iter()
            .map(|port| port_anchor(port, view, theme)),
    );
    children.extend(
        view.outputs
            .iter()
            .map(|port| port_anchor(port, view, theme)),
    );

    Desc::Container {
        id: Cow::Owned(card_id),
        style: BoxStyle {
            position: Position::Absolute {
                x: view.layout.rect.x,
                y: view.layout.rect.y,
            },
            width: Size::Fixed(view.layout.rect.w),
            height: Size::Fixed(if view.layout.collapsed {
                48.0
            } else {
                view.layout.rect.h
            }),
            padding: Edges::all(10.0),
            gap: 0.0,
            direction: Direction::Column,
            hittable: Some(true),
            gestures: vec![Gesture::Drag],
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(theme.colors.surface),
            border: Some(Border {
                width: 1.0,
                color: theme.colors.border,
            }),
            radius: [theme.radii.md; 4],
            shadow: None,
        }),
        children,
    }
}

fn port_group(
    owner_id: &str,
    side: CanvasPortSide,
    group: &CanvasPortGroupView,
    ports: &[CanvasPortView],
    view: &CanvasNodeView,
    theme: &Theme,
) -> Desc {
    let trigger_x = match side {
        CanvasPortSide::Input => -PORT_TRIGGER_SIZE - 6.0,
        CanvasPortSide::Output => view.layout.rect.w + 6.0,
    };
    let trigger_y = view.layout.rect.h * 0.5 - PORT_TRIGGER_SIZE * 0.5;
    let list_x = match side {
        CanvasPortSide::Input => trigger_x - 98.0,
        CanvasPortSide::Output => trigger_x + PORT_TRIGGER_SIZE + 6.0,
    };
    let list_y = trigger_y - (ports.len().max(1) as f32 * 20.0) * 0.5 + 10.0;

    let mut children = vec![port_trigger(owner_id, side, trigger_x, trigger_y, theme)];
    if group.open {
        children.push(pin_list(side, ports, list_x, list_y, theme));
    }

    Desc::Container {
        id: Cow::Owned(super::canvas_port_group_stable_id(owner_id, side)),
        style: BoxStyle {
            position: Position::Absolute { x: 0.0, y: 0.0 },
            width: Size::Fixed(0.0),
            height: Size::Fixed(0.0),
            hittable: Some(false),
            ..BoxStyle::default()
        },
        decoration: None,
        children,
    }
}

fn port_trigger(owner_id: &str, side: CanvasPortSide, x: f32, y: f32, theme: &Theme) -> Desc {
    Desc::Leaf {
        id: Cow::Owned(canvas_port_group_trigger_id(owner_id, side)),
        style: BoxStyle {
            position: Position::Absolute { x, y },
            width: Size::Fixed(PORT_TRIGGER_SIZE),
            height: Size::Fixed(PORT_TRIGGER_SIZE),
            hittable: Some(true),
            gestures: vec![Gesture::Tap],
            ..BoxStyle::default()
        },
        kind: LeafKind::Circle {
            radius: PORT_TRIGGER_SIZE * 0.5,
            fill: Some(theme.colors.surface),
            stroke: Some(Border {
                width: 1.5,
                color: theme.colors.border,
            }),
        },
    }
}

fn pin_list(side: CanvasPortSide, ports: &[CanvasPortView], x: f32, y: f32, theme: &Theme) -> Desc {
    Desc::Container {
        id: Cow::Owned(format!(
            "{}::pin_list",
            ports
                .first()
                .map(|port| port.stable_id.as_str())
                .unwrap_or(side.as_str())
        )),
        style: BoxStyle {
            position: Position::Absolute { x, y },
            width: Size::Fixed(96.0),
            height: Size::Auto,
            padding: Edges::symmetric(6.0, 10.0),
            direction: Direction::Column,
            gap: 4.0,
            hittable: Some(true),
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(theme.colors.surface),
            border: Some(Border {
                width: 1.0,
                color: theme.colors.border,
            }),
            radius: [theme.radii.md; 4],
            shadow: None,
        }),
        children: ports
            .iter()
            .map(|port| pin_item(side, port, theme))
            .collect(),
    }
}

fn pin_item(side: CanvasPortSide, port: &CanvasPortView, theme: &Theme) -> Desc {
    let mut children = vec![
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
        },
        Desc::Leaf {
            id: Cow::Owned(format!("{}::pin_dot", port.stable_id)),
            style: BoxStyle {
                width: Size::Fixed(10.0),
                height: Size::Fixed(10.0),
                ..BoxStyle::default()
            },
            kind: LeafKind::Circle {
                radius: 5.0,
                fill: Some(port_color(port.side, theme)),
                stroke: Some(Border {
                    width: 1.0,
                    color: theme.colors.surface,
                }),
            },
        },
    ];
    if side == CanvasPortSide::Output {
        children.swap(0, 1);
    }

    Desc::Container {
        id: Cow::Owned(format!("{}::pin_item", port.stable_id)),
        style: BoxStyle {
            width: Size::Fill,
            height: Size::Fixed(14.0),
            direction: Direction::Row,
            align_items: Align::Center,
            gap: 6.0,
            hittable: Some(true),
            ..BoxStyle::default()
        },
        decoration: None,
        children,
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
            height: Size::Fill,
            direction: Direction::Column,
            gap: 4.0,
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
            height: Size::Fixed(16.0),
            direction: Direction::Row,
            align_items: Align::Center,
            gap: 6.0,
            ..BoxStyle::default()
        },
        decoration: None,
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

fn port_anchor(port: &CanvasPortView, view: &CanvasNodeView, theme: &Theme) -> Desc {
    let x = match port.side {
        CanvasPortSide::Input => -14.0,
        CanvasPortSide::Output => view.layout.rect.w - 14.0,
    };
    let y = if port.count <= 1 {
        view.layout.rect.h * 0.5 - PORT_SIZE * 0.5
    } else {
        PORT_TOP + port.index as f32 * PORT_GAP
    };

    Desc::Leaf {
        id: Cow::Owned(port.stable_id.clone()),
        style: BoxStyle {
            position: Position::Absolute { x, y },
            width: Size::Fixed(PORT_SIZE),
            height: Size::Fixed(PORT_SIZE),
            hittable: Some(false),
            ..BoxStyle::default()
        },
        kind: LeafKind::Circle {
            radius: PORT_SIZE * 0.5,
            fill: Some(port_color(port.side, theme)),
            stroke: Some(Border {
                width: 1.0,
                color: theme.colors.surface,
            }),
        },
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
            }],
            outputs: vec![CanvasPortView {
                name: "image".to_string(),
                stable_id: "canvas_node::engine_node::7::port::output::image".to_string(),
                side: CanvasPortSide::Output,
                index: 0,
                count: 1,
            }],
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
            .any(|child| child.id() == "canvas_node::engine_node::7::port_group::input"));
        assert!(children.iter().any(|child| contains_desc_id(
            child,
            "canvas_node::engine_node::7::port::input::prompt::pin_list"
        )));
        assert!(children
            .iter()
            .any(|child| child.id() == "canvas_node::engine_node::7::port::input::prompt"));
        assert!(children
            .iter()
            .any(|child| child.id() == "canvas_node::engine_node::7::port::output::image"));
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
            .any(|child| child.id() == "canvas_node::engine_node::7::label"));
        assert!(children
            .iter()
            .any(|child| child.id() == "canvas_node::engine_node::7::body"));
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
