use super::{
    CanvasNodeLayout, CanvasPortConnectionState, CanvasPortGroupView, CanvasPortSide,
    CanvasPortView,
};
use crate::canvas::node_spec::{
    node_render_spec, NodeBodyRowSpec, NodeCardMetrics, NodeHeaderSpec, NodePortSpec,
    NodeRenderSpec,
};
use crate::canvas::param_control::{param_control, CanvasNodeParamControl};
use crate::gesture::Gesture;
use crate::renderer::{Border, Color};
use crate::theme::Theme;
use crate::tree::layout::{Align, Justify, LeafKind, Overflow, TextLayout, TextOverflow};
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use std::borrow::Cow;

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
    let spec = node_render_spec(view, theme);
    node_card_from_spec(&spec, theme)
}

pub(crate) fn node_card_from_spec(spec: &NodeRenderSpec, theme: &Theme) -> Desc {
    let metrics = spec.metrics;
    let children = vec![
        pin_column(
            &spec.input_column_id,
            CanvasPortSide::Input,
            &spec.inputs,
            theme,
            metrics,
        ),
        ui::column(Cow::Owned(spec.card_id.clone()))
            .relative()
            .fixed_width(metrics.card_width)
            .auto_height()
            .padding_all(metrics.card_padding)
            .justify_content(Justify::Start)
            .align_items(Align::Start)
            .gap(metrics.row_gap)
            .overflow(Overflow::Visible)
            .hittable(true)
            .background(theme.colors.surface)
            .border(Border {
                width: if spec.selected { 2.0 } else { 1.0 },
                color: if spec.selected {
                    theme.colors.text
                } else {
                    theme.colors.border
                },
            })
            .radius_all(metrics.card_radius)
            .children(vec![
                node_body(&spec.body, theme, metrics),
                node_header(&spec.header, theme, metrics),
            ])
            .build(),
        pin_column(
            &spec.output_column_id,
            CanvasPortSide::Output,
            &spec.outputs,
            theme,
            metrics,
        ),
    ];

    ui::row(Cow::Owned(spec.id.clone()))
        .absolute_xy(spec.layout.rect.x, spec.layout.rect.y)
        .z_index(spec.layout.z_index)
        .auto_width()
        .auto_height()
        .padding_all(0.0)
        .gap(metrics.column_gap)
        .justify_content(Justify::Start)
        .align_items(Align::Center)
        .overflow(Overflow::Visible)
        .hittable(true)
        .gesture(Gesture::Tap)
        .gesture(Gesture::Drag)
        .children(children)
        .build()
}

fn pin_column(
    id: &str,
    side: CanvasPortSide,
    ports: &[NodePortSpec],
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Desc {
    ui::column(id.to_string())
        .auto_width()
        .auto_height()
        .justify_content(Justify::Start)
        .align_items(Align::Start)
        .gap(metrics.pin_row_gap)
        .overflow(Overflow::Hidden)
        .hittable(false)
        .children(
            ports
                .iter()
                .map(|port| pin_row(side, port, theme, metrics))
                .collect::<Vec<_>>(),
        )
        .build()
}

fn pin_row(
    side: CanvasPortSide,
    port: &NodePortSpec,
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

    ui::row(Cow::Owned(port.row_id.clone()))
        .auto_width()
        .auto_height()
        .justify_content(Justify::Start)
        .align_items(Align::Center)
        .gap(metrics.pin_label_gap)
        .hittable(true)
        .gesture(Gesture::Tap)
        .gesture(Gesture::Drag)
        .children(children)
        .build()
}

fn pin_dot(port: &NodePortSpec, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    ui::leaf(
        Cow::Owned(port.dot_id.clone()),
        LeafKind::Circle {
            radius: metrics.pin_dot_diameter * 0.5,
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
    )
    .fixed_width(metrics.pin_dot_diameter)
    .fixed_height(metrics.pin_dot_diameter)
    .hittable(true)
    .gesture(Gesture::Tap)
    .gesture(Gesture::Drag)
    .build()
}

fn pin_label(port: &NodePortSpec, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    ui::leaf(
        Cow::Owned(port.label_id.clone()),
        LeafKind::Text {
            content: port.name.clone(),
            style: theme.text_style_label_sm(),
            layout: ellipsis_text_layout(),
        },
    )
    .fixed_width(metrics.pin_label_width)
    .auto_height()
    .flex_shrink(1.0)
    .build()
}

fn node_body(
    body: &crate::canvas::node_spec::NodeBodySpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Desc {
    ui::column(Cow::Owned(body.id.clone()))
        .fill_width()
        .auto_height()
        .gap(metrics.row_gap)
        .children(
            body.rows
                .iter()
                .map(|row| body_row_from_spec(row, theme, metrics))
                .collect::<Vec<_>>(),
        )
        .build()
}

fn body_row_from_spec(row: &NodeBodyRowSpec, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    match row {
        NodeBodyRowSpec::Summary { id, text_id, text } => body_row(id.clone(), theme, metrics)
            .child(
                ui::leaf(
                    text_id.clone(),
                    LeafKind::Text {
                        content: text.clone(),
                        style: theme.text_style_label_sm(),
                        layout: ellipsis_text_layout(),
                    },
                )
                .fill_width()
                .auto_height()
                .flex_shrink(1.0),
            )
            .build(),
        NodeBodyRowSpec::Param {
            id,
            name_id,
            value_id,
            control_id,
            name,
            value,
            control,
        } => body_row(id.clone(), theme, metrics)
            .children(vec![
                ui::leaf(
                    name_id.clone(),
                    LeafKind::Text {
                        content: name.clone(),
                        style: theme.text_style_label_sm(),
                        layout: ellipsis_text_layout(),
                    },
                )
                .fill_width()
                .auto_height()
                .flex_shrink(1.0)
                .build(),
                ui::leaf(
                    value_id.clone(),
                    LeafKind::Text {
                        content: value.clone(),
                        style: theme.text_style_label_sm(),
                        layout: ellipsis_text_layout(),
                    },
                )
                .auto_width()
                .auto_height()
                .build(),
                param_control(
                    Cow::Owned(control_id.clone()),
                    control,
                    theme,
                    metrics.control,
                ),
            ])
            .build(),
    }
}

fn node_header(header: &NodeHeaderSpec, theme: &Theme, metrics: NodeCardMetrics) -> Desc {
    ui::row(Cow::Owned(header.row_id.clone()))
        .absolute_xy(0.0, -metrics.title_lift)
        .auto_width()
        .auto_height()
        .justify_content(Justify::Start)
        .align_items(Align::Center)
        .gap(metrics.title_label_gap)
        .hittable(false)
        .children(vec![
            ui::leaf(
                Cow::Owned(header.dot_id.clone()),
                LeafKind::Circle {
                    radius: metrics.title_dot_diameter * 0.5,
                    fill: Some(header.category_color),
                    stroke: None,
                },
            )
            .fixed_width(metrics.title_dot_diameter)
            .fixed_height(metrics.title_dot_diameter)
            .build(),
            ui::leaf(
                Cow::Owned(header.text_id.clone()),
                LeafKind::Text {
                    content: header.title.clone(),
                    style: theme.text_style_label_sm(),
                    layout: ellipsis_text_layout(),
                },
            )
            .fixed_width(metrics.title_label_width)
            .auto_height()
            .flex_shrink(1.0)
            .build(),
        ])
        .build()
}

fn body_row(
    id: impl Into<Cow<'static, str>>,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> ui::ContainerBuilder {
    ui::row(id)
        .fill_width()
        .fixed_height(metrics.param_row_height)
        .justify_content(Justify::Start)
        .align_items(Align::Center)
        .gap(metrics.param_label_gap)
        .background(theme.colors.canvas_bg)
        .radius_all(metrics.row_radius)
}

fn ellipsis_text_layout() -> TextLayout {
    TextLayout {
        overflow: TextOverflow::Ellipsis,
        ..Default::default()
    }
}

fn port_state_color(port: &NodePortSpec, theme: &Theme) -> Color {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::theme::light_theme;
    use crate::tree::layout::{Direction, Edges, Position, Size};

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
        assert_eq!(input_style.gap, metrics.pin_row_gap);
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
        assert_eq!(style.gap, metrics.title_label_gap);
        assert_eq!(children[0].id(), "canvas_node::engine_node::7::label_dot");
        assert_eq!(children[1].id(), "canvas_node::engine_node::7::label_text");
        let Desc::Leaf {
            style: dot_style,
            kind: dot_kind,
            ..
        } = &children[0]
        else {
            panic!("title dot should be a leaf");
        };
        assert_eq!(dot_style.width, Size::Fixed(metrics.title_dot_diameter));
        assert_eq!(dot_style.height, Size::Fixed(metrics.title_dot_diameter));
        assert!(matches!(
            dot_kind,
            LeafKind::Circle { radius, .. } if *radius == metrics.title_dot_diameter * 0.5
        ));
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
        assert_eq!(input_style.gap, metrics.pin_label_gap);
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
        assert_eq!(style.gap, metrics.param_label_gap);
    }

    #[test]
    fn node_card_facade_matches_explicit_spec_builder() {
        let theme = light_theme();
        let view = node_view_with_ports();
        let spec = node_render_spec(&view, &theme);

        let facade = node_card(&view, &theme);
        let explicit = node_card_from_spec(&spec, &theme);

        assert_eq!(facade.id(), explicit.id());
        let Desc::Container {
            children: facade_children,
            ..
        } = facade
        else {
            panic!("facade should build a container");
        };
        let Desc::Container {
            children: explicit_children,
            ..
        } = explicit
        else {
            panic!("explicit builder should build a container");
        };
        let facade_ids = facade_children
            .iter()
            .map(|child| child.id().to_string())
            .collect::<Vec<_>>();
        let explicit_ids = explicit_children
            .iter()
            .map(|child| child.id().to_string())
            .collect::<Vec<_>>();
        assert_eq!(facade_ids, explicit_ids);
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
        assert_eq!(style.width, Size::Fixed(metrics.pin_dot_diameter));
        assert_eq!(style.height, Size::Fixed(metrics.pin_dot_diameter));
        assert!(matches!(
            kind,
            LeafKind::Circle { radius, .. } if *radius == metrics.pin_dot_diameter * 0.5
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
            Desc::Leaf { .. } | Desc::Widget(_) => None,
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
            Desc::Leaf { .. } | Desc::Widget(_) => false,
        }
    }
}
