use super::{
    canvas_node_stable_id, CanvasNodeLayout, CanvasPortConnectionState, CanvasPortGroupView,
    CanvasPortSide, CanvasPortView,
};
use crate::canvas::param_control::{
    param_control, CanvasNodeParamControl, CanvasParamControlMetrics,
};
use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Overflow, Position, Size,
    TextLayout, TextOverflow,
};
use crate::tree::Desc;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq)]
struct NodeCardMetrics {
    card_width: f32,
    pin_diameter: f32,
    pin_label_width: f32,
    title_label_width: f32,
    param_row_height: f32,
    card_padding: f32,
    column_gap: f32,
    row_gap: f32,
    label_gap: f32,
    title_lift: f32,
    card_radius: f32,
    row_radius: f32,
    control: CanvasParamControlMetrics,
}

impl NodeCardMetrics {
    fn from_theme(_theme: &Theme) -> Self {
        Self {
            card_width: 304.0,
            pin_diameter: 32.0,
            pin_label_width: 78.0,
            title_label_width: 262.0,
            param_row_height: 36.0,
            card_padding: 24.0,
            column_gap: 24.0,
            row_gap: 12.0,
            label_gap: 10.0,
            title_lift: 37.0,
            card_radius: 24.0,
            row_radius: 0.0,
            control: CanvasParamControlMetrics::from_theme(_theme),
        }
    }
}

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
    pub control: CanvasNodeParamControl,
}

pub fn node_card(view: &CanvasNodeView, theme: &Theme) -> Desc {
    let metrics = NodeCardMetrics::from_theme(theme);
    let card_id = canvas_node_stable_id(&view.owner_id);
    let label_id = format!("{card_id}::label");
    let label_dot_id = format!("{card_id}::label_dot");
    let label_text_id = format!("{card_id}::label_text");
    let card_body_id = format!("{card_id}::card");
    let body_id = format!("{card_id}::body");

    let children = vec![
        pin_column(
            &view.owner_id,
            CanvasPortSide::Input,
            &view.inputs,
            theme,
            metrics,
        ),
        Desc::Container {
            id: Cow::Owned(card_body_id),
            style: BoxStyle {
                position: Position::relative(),
                width: Size::Fixed(metrics.card_width),
                height: Size::Auto,
                padding: Edges::all(metrics.card_padding),
                direction: Direction::Column,
                justify_content: Justify::Start,
                align_items: Align::Start,
                gap: metrics.row_gap,
                overflow: Overflow::Visible,
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
                radius: [metrics.card_radius; 4],
                shadow: None,
            }),
            children: vec![
                node_body(&body_id, view, theme, metrics),
                Desc::Container {
                    id: Cow::Owned(label_id),
                    style: BoxStyle {
                        position: Position::absolute_xy(0.0, -metrics.title_lift),
                        width: Size::Auto,
                        height: Size::Auto,
                        direction: Direction::Row,
                        justify_content: Justify::Start,
                        align_items: Align::Center,
                        gap: metrics.label_gap,
                        hittable: Some(false),
                        ..BoxStyle::default()
                    },
                    decoration: None,
                    children: vec![
                        Desc::Leaf {
                            id: Cow::Owned(label_dot_id),
                            style: BoxStyle {
                                width: Size::Fixed(metrics.pin_diameter),
                                height: Size::Fixed(metrics.pin_diameter),
                                ..BoxStyle::default()
                            },
                            kind: LeafKind::Circle {
                                radius: metrics.pin_diameter * 0.5,
                                fill: Some(category_color(&view.category)),
                                stroke: None,
                            },
                        },
                        Desc::Leaf {
                            id: Cow::Owned(label_text_id),
                            style: BoxStyle {
                                width: Size::Fixed(metrics.title_label_width),
                                height: Size::Auto,
                                flex_shrink: 1.0,
                                ..BoxStyle::default()
                            },
                            kind: LeafKind::Text {
                                content: view.title.clone(),
                                style: theme.text_style_label_sm(),
                                layout: ellipsis_text_layout(),
                            },
                        },
                    ],
                },
            ],
        },
        pin_column(
            &view.owner_id,
            CanvasPortSide::Output,
            &view.outputs,
            theme,
            metrics,
        ),
    ];

    Desc::Container {
        id: Cow::Owned(card_id),
        style: BoxStyle {
            position: Position::absolute_xy(view.layout.rect.x, view.layout.rect.y),
            z_index: view.layout.z_index,
            width: Size::Auto,
            height: Size::Auto,
            padding: Edges::all(0.0),
            gap: metrics.column_gap,
            direction: Direction::Row,
            justify_content: Justify::Start,
            align_items: Align::Center,
            overflow: Overflow::Visible,
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
    ports: &[CanvasPortView],
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Desc {
    Desc::Container {
        id: Cow::Owned(format!(
            "canvas_node::{owner_id}::pin_column::{}",
            side.as_str()
        )),
        style: BoxStyle {
            width: Size::Auto,
            height: Size::Auto,
            direction: Direction::Column,
            justify_content: Justify::Start,
            align_items: Align::Start,
            gap: metrics.row_gap,
            overflow: Overflow::Hidden,
            hittable: Some(false),
            ..BoxStyle::default()
        },
        decoration: None,
        children: ports
            .iter()
            .map(|port| pin_row(side, port, theme, metrics))
            .collect(),
    }
}

fn pin_row(
    side: CanvasPortSide,
    port: &CanvasPortView,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Desc {
    let mut children = vec![
        pin_dot(port, theme, metrics),
        pin_label(port, theme, metrics),
    ];
    if side == CanvasPortSide::Output {
        children.swap(0, 1);
    }

    Desc::Container {
        id: Cow::Owned(format!("{}::pin_item", port.stable_id)),
        style: BoxStyle {
            width: Size::Auto,
            height: Size::Auto,
            direction: Direction::Row,
            justify_content: Justify::Start,
            align_items: Align::Center,
            gap: metrics.label_gap,
            hittable: Some(true),
            gestures: vec![Gesture::Tap, Gesture::Drag],
            ..BoxStyle::default()
        },
        decoration: None,
        children,
    }
}

fn pin_dot(port: &CanvasPortView, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    Desc::Leaf {
        id: Cow::Owned(port.stable_id.clone()),
        style: BoxStyle {
            width: Size::Fixed(metrics.pin_diameter),
            height: Size::Fixed(metrics.pin_diameter),
            hittable: Some(true),
            gestures: vec![Gesture::Tap, Gesture::Drag],
            ..BoxStyle::default()
        },
        kind: LeafKind::Circle {
            radius: metrics.pin_diameter * 0.5,
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

fn pin_label(port: &CanvasPortView, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    Desc::Leaf {
        id: Cow::Owned(format!("{}::pin_label", port.stable_id)),
        style: BoxStyle {
            width: Size::Fixed(metrics.pin_label_width),
            height: Size::Auto,
            flex_shrink: 1.0,
            ..BoxStyle::default()
        },
        kind: LeafKind::Text {
            content: port.name.clone(),
            style: theme.text_style_label_sm(),
            layout: ellipsis_text_layout(),
        },
    }
}

fn node_body(id: &str, view: &CanvasNodeView, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    let mut children = Vec::new();
    if view.params.is_empty() {
        children.push(summary_row(id, view, theme, metrics));
    } else {
        children.extend(
            view.params
                .iter()
                .enumerate()
                .map(|(index, param)| param_row(id, index, param, theme, metrics)),
        );
    }

    Desc::Container {
        id: Cow::Owned(id.to_string()),
        style: BoxStyle {
            width: Size::Fill,
            height: Size::Auto,
            direction: Direction::Column,
            gap: metrics.row_gap,
            ..BoxStyle::default()
        },
        decoration: None,
        children,
    }
}

fn summary_row(id: &str, view: &CanvasNodeView, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    Desc::Container {
        id: Cow::Owned(format!("{id}::summary")),
        style: body_row_style(metrics),
        decoration: body_row_decoration(theme, metrics),
        children: vec![Desc::Leaf {
            id: Cow::Owned(format!("{id}::summary::text")),
            style: BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
            kind: LeafKind::Text {
                content: view.subtitle.clone(),
                style: theme.text_style_label_sm(),
                layout: ellipsis_text_layout(),
            },
        }],
    }
}

fn param_row(
    id: &str,
    index: usize,
    param: &CanvasNodeParamView,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Desc {
    Desc::Container {
        id: Cow::Owned(format!("{id}::param::{index}")),
        style: body_row_style(metrics),
        decoration: body_row_decoration(theme, metrics),
        children: vec![
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::param::{index}::name")),
                style: BoxStyle {
                    width: Size::Fill,
                    height: Size::Auto,
                    flex_shrink: 1.0,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: param.name.clone(),
                    style: theme.text_style_label_sm(),
                    layout: ellipsis_text_layout(),
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
                    layout: ellipsis_text_layout(),
                },
            },
            param_control(
                Cow::Owned(format!("{id}::param::{index}::control")),
                &param.control,
                theme,
                metrics.control,
            ),
        ],
    }
}

fn body_row_style(metrics: NodeCardMetrics) -> BoxStyle {
    BoxStyle {
        width: Size::Fill,
        height: Size::Fixed(metrics.param_row_height),
        direction: Direction::Row,
        justify_content: Justify::Start,
        align_items: Align::Center,
        gap: metrics.label_gap,
        ..BoxStyle::default()
    }
}

fn body_row_decoration(theme: &Theme, metrics: NodeCardMetrics) -> Option<Decoration> {
    Some(Decoration {
        background: Some(theme.colors.canvas_bg),
        border: None,
        radius: [metrics.row_radius; 4],
        shadow: None,
    })
}

fn ellipsis_text_layout() -> TextLayout {
    TextLayout {
        overflow: TextOverflow::Ellipsis,
        ..Default::default()
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
    fn node_card_shell_uses_root_layout_origin() {
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

        assert_eq!(style.position, Position::absolute_xy(10.0, 20.0));
        assert_eq!(style.z_index, 0);
        assert_eq!(style.width, Size::Auto);
        assert_eq!(style.height, Size::Auto);
        assert_eq!(style.direction, Direction::Row);
        assert_eq!(style.justify_content, Justify::Start);
        assert_eq!(style.align_items, Align::Center);
        assert_eq!(style.overflow, Overflow::Visible);
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
                control: CanvasNodeParamControl::default(),
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
                control: CanvasNodeParamControl::default(),
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

    #[test]
    fn node_card_uses_figma_flow_structure() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);
        let view = node_view_with_ports();

        let Desc::Container {
            style, children, ..
        } = node_card(&view, &theme)
        else {
            panic!("node card should build a container");
        };

        assert_eq!(style.gap, metrics.column_gap);
        assert_eq!(children.len(), 3);
        assert_eq!(
            children[0].id(),
            "canvas_node::engine_node::7::pin_column::input"
        );
        assert_eq!(children[1].id(), "canvas_node::engine_node::7::card");
        assert_eq!(
            children[2].id(),
            "canvas_node::engine_node::7::pin_column::output"
        );

        let Desc::Container {
            style: input_style, ..
        } = &children[0]
        else {
            panic!("input pin column should be a container");
        };
        assert_eq!(input_style.position, Position::Flow);
        assert_eq!(input_style.width, Size::Auto);
        assert_eq!(input_style.height, Size::Auto);
        assert_eq!(input_style.direction, Direction::Column);
        assert_eq!(input_style.gap, metrics.row_gap);
        assert_eq!(input_style.overflow, Overflow::Hidden);

        let Desc::Container {
            style: card_style,
            decoration,
            ..
        } = &children[1]
        else {
            panic!("center node card should be a container");
        };
        assert_eq!(card_style.position, Position::relative());
        assert_eq!(card_style.width, Size::Fixed(metrics.card_width));
        assert_eq!(card_style.height, Size::Auto);
        assert_eq!(card_style.padding, Edges::all(metrics.card_padding));
        assert_eq!(card_style.direction, Direction::Column);
        assert_eq!(card_style.gap, metrics.row_gap);
        assert_eq!(card_style.overflow, Overflow::Visible);
        assert_eq!(
            decoration.as_ref().unwrap().radius,
            [metrics.card_radius; 4]
        );
    }

    #[test]
    fn node_card_title_is_absolute_card_child() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);
        let view = node_view_with_ports();

        let Desc::Container { children, .. } = node_card(&view, &theme) else {
            panic!("node card should build a container");
        };
        let Desc::Container {
            children: card_children,
            ..
        } = &children[1]
        else {
            panic!("center node card should be a container");
        };
        let title = card_children
            .iter()
            .find(|child| child.id() == "canvas_node::engine_node::7::label")
            .expect("card should contain an absolute title row");
        let Desc::Container {
            style, children, ..
        } = title
        else {
            panic!("title should be a container");
        };

        assert_eq!(
            style.position,
            Position::absolute_xy(0.0, -metrics.title_lift)
        );
        assert_eq!(style.direction, Direction::Row);
        assert_eq!(style.align_items, Align::Center);
        assert_eq!(style.gap, metrics.label_gap);
        assert_eq!(children[0].id(), "canvas_node::engine_node::7::label_dot");
        assert_eq!(children[1].id(), "canvas_node::engine_node::7::label_text");
    }

    #[test]
    fn node_card_pin_rows_keep_side_order_and_metrics() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);
        let view = node_view_with_ports();

        let Desc::Container { children, .. } = node_card(&view, &theme) else {
            panic!("node card should build a container");
        };
        let input_row = first_pin_row(&children[0]);
        let output_row = first_pin_row(&children[2]);

        let Desc::Container {
            style: input_style,
            children: input_children,
            ..
        } = input_row
        else {
            panic!("input pin row should be a container");
        };
        assert_eq!(input_style.direction, Direction::Row);
        assert_eq!(input_style.align_items, Align::Center);
        assert_eq!(input_style.gap, metrics.label_gap);
        assert_eq!(
            input_children[0].id(),
            "canvas_node::engine_node::7::port::input::prompt"
        );
        assert_eq!(
            input_children[1].id(),
            "canvas_node::engine_node::7::port::input::prompt::pin_label"
        );

        let Desc::Container {
            children: output_children,
            ..
        } = output_row
        else {
            panic!("output pin row should be a container");
        };
        assert_eq!(
            output_children[0].id(),
            "canvas_node::engine_node::7::port::output::image::pin_label"
        );
        assert_eq!(
            output_children[1].id(),
            "canvas_node::engine_node::7::port::output::image"
        );

        assert_pin_dot_metrics(&input_children[0], metrics);
        assert_pin_dot_metrics(&output_children[1], metrics);
    }

    #[test]
    fn node_card_param_rows_use_fixed_control_height() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);
        let view = CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            category: "image/generation".to_string(),
            params: vec![CanvasNodeParamView {
                name: "prompt".to_string(),
                kind: "string".to_string(),
                value: "text".to_string(),
                control: CanvasNodeParamControl::default(),
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

        let Desc::Container { children, .. } = node_card(&view, &theme) else {
            panic!("node card should build a container");
        };
        let param = find_desc(&children[1], "canvas_node::engine_node::7::body::param::0")
            .expect("card body should contain the first param row");
        let Desc::Container { style, .. } = param else {
            panic!("param row should be a container");
        };

        assert_eq!(style.width, Size::Fill);
        assert_eq!(style.height, Size::Fixed(metrics.param_row_height));
        assert_eq!(style.direction, Direction::Row);
        assert_eq!(style.align_items, Align::Center);
        assert_eq!(style.gap, metrics.label_gap);
    }

    fn node_view_with_ports() -> CanvasNodeView {
        CanvasNodeView {
            owner_id: "engine_node::7".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            category: "image/generation".to_string(),
            params: vec![CanvasNodeParamView {
                name: "prompt".to_string(),
                kind: "string".to_string(),
                value: "text".to_string(),
                control: CanvasNodeParamControl::default(),
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
        }
    }

    fn first_pin_row(column: &Desc) -> &Desc {
        let Desc::Container { children, .. } = column else {
            panic!("pin column should be a container");
        };
        children.first().expect("pin column should contain a row")
    }

    fn assert_pin_dot_metrics(desc: &Desc, metrics: NodeCardMetrics) {
        let Desc::Leaf { style, kind, .. } = desc else {
            panic!("pin dot should be a leaf");
        };
        assert_eq!(style.width, Size::Fixed(metrics.pin_diameter));
        assert_eq!(style.height, Size::Fixed(metrics.pin_diameter));
        assert!(matches!(
            kind,
            LeafKind::Circle { radius, .. } if *radius == metrics.pin_diameter * 0.5
        ));
    }

    fn find_desc<'a>(desc: &'a Desc, id: &str) -> Option<&'a Desc> {
        if desc.id() == id {
            return Some(desc);
        }
        match desc {
            Desc::Container { children, .. } => {
                children.iter().find_map(|child| find_desc(child, id))
            }
            Desc::Leaf { .. } | Desc::Widget { .. } => None,
        }
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
