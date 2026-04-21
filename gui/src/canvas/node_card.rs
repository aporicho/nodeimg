use super::{canvas_node_stable_id, CanvasNodeLayout, CanvasPortSide, CanvasPortView};
use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Decoration, Direction, Edges, LeafKind, Position, Size};
use crate::tree::Desc;
use std::borrow::Cow;

const PORT_SIZE: f32 = 8.0;
const PORT_TOP: f32 = 34.0;
const PORT_GAP: f32 = 18.0;

#[derive(Clone, Debug)]
pub struct CanvasNodeView {
    pub owner_id: String,
    pub title: String,
    pub subtitle: String,
    pub inputs: Vec<CanvasPortView>,
    pub outputs: Vec<CanvasPortView>,
    pub layout: CanvasNodeLayout,
}

pub fn node_card(view: &CanvasNodeView, theme: &Theme) -> Desc {
    let card_id = canvas_node_stable_id(&view.owner_id);
    let title_id = format!("{card_id}::title");
    let subtitle_id = format!("{card_id}::subtitle");

    let mut children = vec![
        Desc::Leaf {
            id: Cow::Owned(title_id),
            style: BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            kind: LeafKind::Text {
                content: view.title.clone(),
                style: theme.text_style_title_sm(),
            },
        },
        Desc::Leaf {
            id: Cow::Owned(subtitle_id),
            style: BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                ..BoxStyle::default()
            },
            kind: LeafKind::Text {
                content: view.subtitle.clone(),
                style: theme.text_style_label_sm(),
            },
        },
    ];
    children.extend(view.inputs.iter().map(|port| port_leaf(port, view, theme)));
    children.extend(view.outputs.iter().map(|port| port_leaf(port, view, theme)));

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
            gap: 4.0,
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

fn port_leaf(port: &CanvasPortView, view: &CanvasNodeView, theme: &Theme) -> Desc {
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
            .any(|child| child.id() == "canvas_node::engine_node::7::port::input::prompt"));
        assert!(children
            .iter()
            .any(|child| child.id() == "canvas_node::engine_node::7::port::output::image"));
    }
}
