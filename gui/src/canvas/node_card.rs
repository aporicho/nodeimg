use super::{CanvasPortConnectionState, CanvasPortSide};
use crate::canvas::node_spec::{
    node_render_spec, NodeBodyRowSpec, NodeHeaderSpec, NodePortGroupTriggerSpec, NodePortSpec,
    NodeRenderSpec,
};
use crate::canvas::node_style::NodeCardMetrics;
use crate::canvas::node_template::CanvasNodeRenderView;
use crate::gesture::Gesture;
use crate::icon::names;
use crate::renderer::{Border, Color};
use crate::theme::Theme;
use crate::tree::layout::{Align, Justify, LeafKind, Overflow, TextLayout, TextOverflow};
use crate::tree::Desc;
use crate::ui::{self, DecorationBuilder, StyleBuilder};
use crate::widget::param_control::{
    param_control, param_control_layout_policy, ParamControlLayoutPolicy,
};
use std::borrow::Cow;

pub fn node_card_from_render_view(view: &CanvasNodeRenderView, theme: &Theme) -> Desc {
    let spec = node_render_spec(&view.template, &view.state, theme);
    node_card_from_spec(&spec, theme)
}

pub(crate) fn node_card_from_spec(spec: &NodeRenderSpec, theme: &Theme) -> Desc {
    let metrics = spec.metrics;
    tracing::debug!(
        target: "gui::canvas::node_resize",
        owner_id = %spec.layout.owner_id,
        card_id = %spec.card_id,
        layout_x = spec.layout.rect.x,
        layout_y = spec.layout.rect.y,
        layout_w = spec.layout.rect.w,
        layout_h = spec.layout.rect.h,
        metrics_w = metrics.card_width,
        metrics_h = metrics.card_height,
        resizable = true,
        "build canvas node card"
    );
    let children = vec![
        pin_column(
            &spec.input_column_id,
            &spec.input_trigger,
            CanvasPortSide::Input,
            &spec.inputs,
            theme,
            metrics,
        ),
        ui::column(Cow::Owned(spec.card_id.clone()))
            .relative()
            .fixed_width(metrics.card_width)
            .fixed_height(metrics.card_height)
            .padding_all(metrics.card_padding)
            .justify_content(Justify::Start)
            .align_items(Align::Start)
            .gap(metrics.row_gap)
            .overflow(Overflow::Visible)
            .hittable(true)
            .resizable(true)
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
            &spec.output_trigger,
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
        .draggable(true)
        .children(children)
        .build()
}

fn pin_column(
    id: &str,
    trigger: &NodePortGroupTriggerSpec,
    side: CanvasPortSide,
    ports: &[NodePortSpec],
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Desc {
    let mut children = ports
        .iter()
        .map(|port| pin_row(side, port, theme, metrics))
        .collect::<Vec<_>>();
    if !ports.is_empty() {
        children.push(port_group_trigger(trigger, theme, metrics));
    }

    ui::column(id.to_string())
        .relative()
        .auto_width()
        .auto_height()
        .justify_content(Justify::Start)
        .align_items(Align::Start)
        .gap(metrics.pin_row_gap)
        .overflow(Overflow::Visible)
        .hittable(false)
        .children(children)
        .build()
}

fn port_group_trigger(
    trigger: &NodePortGroupTriggerSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Desc {
    let diameter = metrics.port_group_trigger_diameter;
    let (x, y) = port_group_trigger_offset(trigger.side, metrics);
    let border_color = if trigger.open {
        theme.colors.accent
    } else {
        theme.colors.border
    };
    let icon_color = if trigger.open {
        theme.colors.accent
    } else {
        theme.colors.text_muted
    };

    ui::row(Cow::Owned(trigger.id.clone()))
        .absolute_xy(x, y)
        .fixed_width(diameter)
        .fixed_height(diameter)
        .justify_content(Justify::Center)
        .align_items(Align::Center)
        .hittable(true)
        .gesture(Gesture::Tap)
        .background(theme.colors.surface)
        .border(Border {
            width: if trigger.open { 2.0 } else { 1.5 },
            color: border_color,
        })
        .radius_all(diameter * 0.5)
        .child(ui::icon(
            format!("{}::icon", trigger.id),
            names::PLUS,
            metrics.port_group_trigger_icon_size,
            icon_color,
        ))
        .build()
}

fn port_group_trigger_offset(side: CanvasPortSide, metrics: NodeCardMetrics) -> (f32, f32) {
    let diameter = metrics.port_group_trigger_diameter;
    let dot_center_offset = (metrics.pin_dot_diameter - diameter) * 0.5;
    let x = match side {
        CanvasPortSide::Input => dot_center_offset,
        CanvasPortSide::Output => {
            metrics.pin_label_width + metrics.pin_label_gap + dot_center_offset
        }
    };
    let y = -diameter - metrics.pin_row_gap;
    (x, y)
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
        .fill_height()
        .flex_grow(1.0)
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
        NodeBodyRowSpec::Summary { id, text_id, text } => {
            fixed_body_row(id.clone(), theme, metrics)
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
                .build()
        }
        NodeBodyRowSpec::Param {
            id,
            control_id,
            control,
        } => {
            let policy = param_control_layout_policy(control, theme, metrics.control);
            control_body_row(id.clone(), theme, metrics, policy)
                .child(param_control(
                    Cow::Owned(control_id.clone()),
                    control,
                    theme,
                    metrics.control,
                ))
                .build()
        }
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

fn fixed_body_row(
    id: impl Into<Cow<'static, str>>,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> ui::ContainerBuilder {
    base_body_row(id, theme, metrics, Align::Center).fixed_height(metrics.param_row_height)
}

fn control_body_row(
    id: impl Into<Cow<'static, str>>,
    theme: &Theme,
    metrics: NodeCardMetrics,
    policy: ParamControlLayoutPolicy,
) -> ui::ContainerBuilder {
    let row = base_body_row(id, theme, metrics, policy.row_align);
    if policy.fills_parent_height() {
        row.fill_height()
            .min_height(policy.min_height().max(metrics.param_row_height))
            .flex_grow(1.0)
    } else {
        row.fixed_height(metrics.param_row_height.max(policy.min_height()))
    }
}

fn base_body_row(
    id: impl Into<Cow<'static, str>>,
    theme: &Theme,
    metrics: NodeCardMetrics,
    align_items: Align,
) -> ui::ContainerBuilder {
    let row = ui::row(id)
        .fill_width()
        .justify_content(Justify::Start)
        .align_items(align_items)
        .gap(metrics.param_label_gap)
        .background(theme.colors.canvas_bg)
        .radius_all(metrics.row_radius);
    row
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
    use crate::canvas::node_template::{
        CanvasNodeInstanceState, CanvasNodeParamTemplate, CanvasNodePortState,
        CanvasNodePortTemplate, CanvasNodeTemplate,
    };
    use crate::gesture::arena_from_resize_hit;
    use crate::gesture::GestureSignal;
    use crate::renderer::{Rect, TextMeasurer};
    use crate::theme::light_theme;
    use crate::tree::layout::{Direction, Edges, Position, Size};
    use crate::tree::{hit_test, layout, reconcile, resize_hit_at_screen_point, Tree};
    use crate::widget::mapping::ParamControlSpec;
    use crate::widget::props::WidgetBuildCx;

    #[test]
    fn node_card_uses_canvas_node_stable_id() {
        assert_eq!(
            node_card_from_render_view(&node_render_view(false, false), &light_theme()).id(),
            "canvas_node::engine_node::7"
        );
    }

    #[test]
    fn node_card_declares_draggable_root() {
        let Desc::Container { style, .. } =
            node_card_from_render_view(&node_render_view(false, false), &light_theme())
        else {
            panic!("node card should build a container");
        };
        assert!(style.draggable);
    }

    #[test]
    fn node_card_shell_uses_root_layout_origin() {
        let Desc::Container { style, .. } =
            node_card_from_render_view(&node_render_view(false, false), &light_theme())
        else {
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
        let Desc::Container { children, .. } =
            node_card_from_render_view(&node_render_view(true, true), &light_theme())
        else {
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
    fn node_card_uses_figma_flow_structure() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);

        let Desc::Container {
            style, children, ..
        } = node_card_from_render_view(&node_render_view(true, true), &theme)
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
        assert_eq!(input_style.position, Position::relative());
        assert_eq!(input_style.width, Size::Auto);
        assert_eq!(input_style.height, Size::Auto);
        assert_eq!(input_style.direction, Direction::Column);
        assert_eq!(input_style.gap, metrics.pin_row_gap);
        assert_eq!(input_style.overflow, Overflow::Visible);

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
        assert_eq!(card_style.height, Size::Fixed(metrics.card_height));
        assert_eq!(card_style.padding, Edges::all(metrics.card_padding));
        assert_eq!(card_style.direction, Direction::Column);
        assert_eq!(card_style.gap, metrics.row_gap);
        assert_eq!(card_style.overflow, Overflow::Visible);
        assert!(card_style.resizable);
        assert_eq!(
            decoration.as_ref().unwrap().radius,
            [metrics.card_radius; 4]
        );
    }

    #[test]
    fn node_card_title_is_absolute_card_child() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);

        let Desc::Container { children, .. } =
            node_card_from_render_view(&node_render_view(true, true), &theme)
        else {
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

        let Desc::Container { children, .. } =
            node_card_from_render_view(&node_render_view(true, true), &theme)
        else {
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
    fn node_card_port_group_trigger_uses_plus_icon() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);

        let Desc::Container { children, .. } =
            node_card_from_render_view(&node_render_view(true, true), &theme)
        else {
            panic!("node card should build a container");
        };
        let trigger = find_desc(
            &children[0],
            "canvas_node::engine_node::7::port_group::input::trigger",
        )
        .expect("input pin column should contain a port group trigger");
        let Desc::Container {
            style,
            decoration,
            children: trigger_children,
            ..
        } = trigger
        else {
            panic!("port group trigger should be a container");
        };
        assert_eq!(
            style.width,
            Size::Fixed(metrics.port_group_trigger_diameter)
        );
        assert_eq!(
            style.height,
            Size::Fixed(metrics.port_group_trigger_diameter)
        );
        assert_eq!(
            style.position,
            Position::absolute_xy(
                (metrics.pin_dot_diameter - metrics.port_group_trigger_diameter) * 0.5,
                -metrics.port_group_trigger_diameter - metrics.pin_row_gap
            )
        );
        assert_eq!(
            decoration.as_ref().unwrap().radius,
            [metrics.port_group_trigger_diameter * 0.5; 4]
        );

        let Desc::Leaf {
            id,
            style: icon_style,
            kind,
        } = &trigger_children[0]
        else {
            panic!("port group trigger visual should be an icon leaf");
        };
        assert_eq!(
            id.as_ref(),
            "canvas_node::engine_node::7::port_group::input::trigger::icon"
        );
        assert_eq!(
            icon_style.width,
            Size::Fixed(metrics.port_group_trigger_icon_size)
        );
        let LeafKind::Icon { spec } = kind else {
            panic!("port group trigger should use LeafKind::Icon");
        };
        assert_eq!(spec.id, crate::icon::IconId::from(names::PLUS));
    }

    #[test]
    fn node_card_param_rows_use_fixed_control_height() {
        let theme = light_theme();
        let metrics = NodeCardMetrics::from_theme(&theme);

        let Desc::Container { children, .. } =
            node_card_from_render_view(&node_render_view(false, true), &theme)
        else {
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
    fn node_card_param_rows_render_only_the_control() {
        let theme = light_theme();
        let Desc::Container { children, .. } =
            node_card_from_render_view(&node_render_view(false, true), &theme)
        else {
            panic!("node card should build a container");
        };
        let param = find_desc(&children[1], "canvas_node::engine_node::7::body::param::0")
            .expect("card body should contain the first param row");
        let Desc::Container {
            children: row_children,
            ..
        } = param
        else {
            panic!("param row should be a container");
        };

        assert_eq!(row_children.len(), 1);
        assert_eq!(
            row_children[0].id(),
            "canvas_node::engine_node::7::body::param::0::control"
        );
        assert!(find_desc(param, "canvas_node::engine_node::7::body::param::0::name").is_none());
        assert!(find_desc(param, "canvas_node::engine_node::7::body::param::0::value").is_none());
    }

    #[test]
    fn node_card_text_area_rows_fill_card_body_height() {
        let theme = light_theme();
        let mut view = node_render_view(false, false);
        view.template.params = vec![CanvasNodeParamTemplate::new(
            "prompt",
            "prompt",
            "string",
            "text",
            ParamControlSpec::TextArea {
                value: "line".to_string(),
                min_rows: 5,
            },
        )];
        view.state.layout.rect.h = 240.0;

        let Desc::Container { children, .. } = node_card_from_render_view(&view, &theme) else {
            panic!("node card should build a container");
        };
        let body = find_desc(&children[1], "canvas_node::engine_node::7::body")
            .expect("card should contain a body");
        let Desc::Container {
            style: body_style, ..
        } = body
        else {
            panic!("body should be a container");
        };
        assert_eq!(body_style.height, Size::Fill);
        assert_eq!(body_style.flex_grow, 1.0);

        let param = find_desc(&children[1], "canvas_node::engine_node::7::body::param::0")
            .expect("card body should contain the text area param row");
        let Desc::Container { style, .. } = param else {
            panic!("param row should be a container");
        };
        assert_eq!(style.height, Size::Fill);
        assert_eq!(style.flex_grow, 1.0);
        assert_eq!(style.align_items, Align::Stretch);
    }

    #[test]
    fn node_card_from_render_view_matches_explicit_spec_builder() {
        let theme = light_theme();
        let view = node_render_view(true, true);
        let spec = node_render_spec(&view.template, &view.state, &theme);

        let facade = node_card_from_render_view(&view, &theme);
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

    #[test]
    fn node_card_edge_resize_survives_internal_widget_hit_chain() {
        let theme = light_theme();
        let desc = node_card_from_render_view(&node_render_view(false, true), &theme);
        let mut tree = Tree::new();
        reconcile(
            &mut tree,
            desc,
            WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );
        let root = tree.root().expect("root");
        let mut measurer = TextMeasurer::new();
        layout(
            &mut tree,
            root,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1000.0,
                h: 1000.0,
            },
            &mut |text, style| measurer.measure_with_style(text, style),
        );
        let card = tree
            .iter()
            .find_map(|(_, node)| {
                (node.id.as_ref() == "canvas_node::engine_node::7::card").then_some(node.rect)
            })
            .expect("card rect");
        let x = card.x + card.w - 2.0;
        let y = card.y + card.h - 2.0;
        let chain = hit_test(&tree, root, x, y);
        assert!(chain.iter().any(|id| {
            tree.get(id)
                .is_some_and(|node| node.id.as_ref() == "canvas_node::engine_node::7::card")
        }));

        let hit = resize_hit_at_screen_point(&tree, root, x, y, None).expect("resize hit");
        let mut arena = arena_from_resize_hit(&tree, hit, x, y).expect("resize arena");
        let signal = arena
            .pointer_move(x + 12.0, y + 12.0)
            .expect("resize start");

        match signal {
            GestureSignal::ResizeStart { id, .. } => {
                assert_eq!(id, "canvas_node::engine_node::7::card");
            }
            other => panic!("expected resize start, got {:?}", other),
        }
    }

    fn node_render_view(include_ports: bool, include_params: bool) -> CanvasNodeRenderView {
        let inputs = if include_ports {
            vec![CanvasNodePortTemplate::new(
                "prompt",
                "prompt",
                CanvasPortSide::Input,
            )]
        } else {
            Vec::new()
        };
        let outputs = if include_ports {
            vec![CanvasNodePortTemplate::new(
                "image",
                "image",
                CanvasPortSide::Output,
            )]
        } else {
            Vec::new()
        };
        let params = if include_params {
            vec![CanvasNodeParamTemplate::new(
                "prompt",
                "prompt",
                "string",
                "text",
                ParamControlSpec::default(),
            )]
        } else {
            Vec::new()
        };
        let port_states = inputs
            .iter()
            .chain(outputs.iter())
            .map(|port| CanvasNodePortState {
                key: port.key.clone(),
                side: port.side,
                connection_state: CanvasPortConnectionState::Idle,
            })
            .collect();

        CanvasNodeRenderView {
            template: CanvasNodeTemplate {
                type_id: "image_gen".to_string(),
                title: "Image".to_string(),
                subtitle: "image_gen".to_string(),
                category: "image/generation".to_string(),
                inputs,
                outputs,
                params,
            },
            state: CanvasNodeInstanceState {
                owner_id: "engine_node::7".to_string(),
                layout: super::super::CanvasNodeLayout {
                    owner_id: "engine_node::7".to_string(),
                    rect: Rect {
                        x: 10.0,
                        y: 20.0,
                        w: 220.0,
                        h: 96.0,
                    },
                    z_index: 0,
                    collapsed: false,
                    user_min_height: None,
                },
                selected: false,
                input_group: Default::default(),
                output_group: Default::default(),
                port_states,
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
