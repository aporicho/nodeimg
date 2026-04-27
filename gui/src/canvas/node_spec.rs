use super::{
    canvas_node_stable_id, canvas_port_group_trigger_id, CanvasNodeLayout,
    CanvasPortConnectionState, CanvasPortSide,
};
use crate::canvas::node_style::NodeCardMetrics;
use crate::canvas::node_template::{CanvasNodeInstanceState, CanvasNodeTemplate};
use crate::renderer::Color;
use crate::theme::Theme;
use crate::widget::mapping::ParamControlSpec;
use crate::widget::param_control::param_control_min_height;

#[derive(Debug, Clone)]
pub(crate) struct NodeRenderSpec {
    pub id: String,
    pub selected: bool,
    pub layout: CanvasNodeLayout,
    pub metrics: NodeCardMetrics,
    pub input_column_id: String,
    pub output_column_id: String,
    pub input_trigger: NodePortGroupTriggerSpec,
    pub output_trigger: NodePortGroupTriggerSpec,
    pub card_id: String,
    pub header: NodeHeaderSpec,
    pub body: NodeBodySpec,
    pub inputs: Vec<NodePortSpec>,
    pub outputs: Vec<NodePortSpec>,
}

#[derive(Debug, Clone)]
pub(crate) struct NodePortGroupTriggerSpec {
    pub id: String,
    pub side: CanvasPortSide,
    pub open: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct NodeHeaderSpec {
    pub row_id: String,
    pub dot_id: String,
    pub text_id: String,
    pub title: String,
    pub category_color: Color,
}

#[derive(Debug, Clone)]
pub(crate) struct NodeBodySpec {
    pub id: String,
    pub rows: Vec<NodeBodyRowSpec>,
}

#[derive(Debug, Clone)]
pub(crate) enum NodeBodyRowSpec {
    Summary {
        id: String,
        text_id: String,
        text: String,
    },
    Param {
        id: String,
        control_id: String,
        control: ParamControlSpec,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct NodePortSpec {
    pub row_id: String,
    pub dot_id: String,
    pub label_id: String,
    pub name: String,
    pub side: CanvasPortSide,
    pub connection_state: CanvasPortConnectionState,
}

pub(crate) fn node_render_spec(
    template: &CanvasNodeTemplate,
    state: &CanvasNodeInstanceState,
    theme: &Theme,
) -> NodeRenderSpec {
    let mut metrics = NodeCardMetrics::for_layout(theme, state.layout.rect);
    let id = canvas_node_stable_id(&state.owner_id);
    let body_id = format!("{id}::body");
    let rows = body_rows(&body_id, template);
    metrics.card_height = metrics
        .card_height
        .max(node_content_min_height(&rows, metrics, theme));

    NodeRenderSpec {
        id: id.clone(),
        selected: state.selected,
        layout: state.layout.clone(),
        metrics,
        input_column_id: pin_column_id(&state.owner_id, CanvasPortSide::Input),
        output_column_id: pin_column_id(&state.owner_id, CanvasPortSide::Output),
        input_trigger: port_group_trigger_spec(state, CanvasPortSide::Input),
        output_trigger: port_group_trigger_spec(state, CanvasPortSide::Output),
        card_id: format!("{id}::card"),
        header: NodeHeaderSpec {
            row_id: format!("{id}::label"),
            dot_id: format!("{id}::label_dot"),
            text_id: format!("{id}::label_text"),
            title: template.title.clone(),
            category_color: category_color(&template.category),
        },
        body: NodeBodySpec {
            id: body_id.clone(),
            rows,
        },
        inputs: template
            .inputs
            .iter()
            .map(|port| {
                port_spec(
                    port.name.clone(),
                    &state.port_id(port.side, &port.key),
                    port.side,
                    state.connection_state(port.side, &port.key),
                )
            })
            .collect(),
        outputs: template
            .outputs
            .iter()
            .map(|port| {
                port_spec(
                    port.name.clone(),
                    &state.port_id(port.side, &port.key),
                    port.side,
                    state.connection_state(port.side, &port.key),
                )
            })
            .collect(),
    }
}

fn node_content_min_height(
    rows: &[NodeBodyRowSpec],
    metrics: NodeCardMetrics,
    theme: &Theme,
) -> f32 {
    let body_height = rows
        .iter()
        .map(|row| row_min_height(row, metrics, theme))
        .sum::<f32>()
        + metrics.row_gap * rows.len().saturating_sub(1) as f32;
    body_height + metrics.card_padding * 2.0
}

fn row_min_height(row: &NodeBodyRowSpec, metrics: NodeCardMetrics, theme: &Theme) -> f32 {
    match row {
        NodeBodyRowSpec::Param { control, .. } => {
            param_control_min_height(control, theme, metrics.control).max(metrics.param_row_height)
        }
        _ => metrics.param_row_height,
    }
}

fn port_group_trigger_spec(
    state: &CanvasNodeInstanceState,
    side: CanvasPortSide,
) -> NodePortGroupTriggerSpec {
    NodePortGroupTriggerSpec {
        id: canvas_port_group_trigger_id(&state.owner_id, side),
        side,
        open: state.port_group(side).open,
    }
}

fn pin_column_id(owner_id: &str, side: CanvasPortSide) -> String {
    format!("canvas_node::{owner_id}::pin_column::{}", side.as_str())
}

fn port_spec(
    name: String,
    stable_id: &str,
    side: CanvasPortSide,
    connection_state: CanvasPortConnectionState,
) -> NodePortSpec {
    NodePortSpec {
        row_id: format!("{stable_id}::pin_item"),
        dot_id: stable_id.to_string(),
        label_id: format!("{stable_id}::pin_label"),
        name,
        side,
        connection_state,
    }
}

fn body_rows(body_id: &str, template: &CanvasNodeTemplate) -> Vec<NodeBodyRowSpec> {
    if template.params.is_empty() {
        return vec![NodeBodyRowSpec::Summary {
            id: format!("{body_id}::summary"),
            text_id: format!("{body_id}::summary::text"),
            text: template.subtitle.clone(),
        }];
    }

    template
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let id = format!("{body_id}::param::{index}");
            NodeBodyRowSpec::Param {
                control_id: format!("{id}::control"),
                id,
                control: param.control.clone(),
            }
        })
        .collect()
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
    use crate::canvas::node_template::{
        CanvasNodeParamTemplate, CanvasNodePortState, CanvasNodePortTemplate,
    };
    use crate::renderer::Rect;
    use crate::theme::light_theme;
    use crate::widget::mapping::ParamControlSpec;

    #[test]
    fn spec_root_id_uses_canvas_node_stable_id() {
        let (template, state) = template_and_state_with_ports();
        let spec = node_render_spec(&template, &state, &light_theme());

        assert_eq!(spec.id, "canvas_node::engine_node::7");
        assert_eq!(spec.card_id, "canvas_node::engine_node::7::card");
    }

    #[test]
    fn spec_preserves_port_order_and_ids() {
        let (template, state) = template_and_state_with_ports();
        let spec = node_render_spec(&template, &state, &light_theme());

        assert_eq!(
            spec.input_column_id,
            "canvas_node::engine_node::7::pin_column::input"
        );
        assert_eq!(
            spec.inputs[0].dot_id,
            "canvas_node::engine_node::7::port::input::prompt"
        );
        assert_eq!(
            spec.inputs[0].label_id,
            "canvas_node::engine_node::7::port::input::prompt::pin_label"
        );
        assert_eq!(
            spec.outputs[0].row_id,
            "canvas_node::engine_node::7::port::output::image::pin_item"
        );
    }

    #[test]
    fn param_row_spec_only_carries_the_control_binding() {
        let (mut template, state) = template_and_state_with_ports();
        template.params[0].name = "Prompt".to_string();
        template.params[0].kind = "string".to_string();
        template.params[0].default_value.clear();

        let spec = node_render_spec(&template, &state, &light_theme());

        let NodeBodyRowSpec::Param {
            id,
            control_id,
            control,
        } = &spec.body.rows[0]
        else {
            panic!("expected param row");
        };
        assert_eq!(id, "canvas_node::engine_node::7::body::param::0");
        assert_eq!(
            control_id,
            "canvas_node::engine_node::7::body::param::0::control"
        );
        assert!(matches!(control, ParamControlSpec::ReadOnly { .. }));
    }

    #[test]
    fn header_spec_includes_category_color() {
        let (template, state) = template_and_state_with_ports();
        let spec = node_render_spec(&template, &state, &light_theme());

        assert_eq!(
            spec.header.category_color,
            Color {
                r: 0.976,
                g: 0.451,
                b: 0.086,
                a: 1.0,
            }
        );
    }

    #[test]
    fn template_and_instance_state_are_separate_models() {
        let (template, state) = template_and_state_with_ports();

        assert_eq!(template.title, "Image");
        assert_eq!(template.inputs[0].key, "prompt");
        assert_eq!(template.outputs[0].side, CanvasPortSide::Output);
        assert_eq!(state.owner_id, "engine_node::7");
        assert_eq!(state.port_states.len(), 2);
        assert_eq!(
            state.connection_state(CanvasPortSide::Input, "prompt"),
            CanvasPortConnectionState::Idle
        );
    }

    fn template_and_state_with_ports() -> (CanvasNodeTemplate, CanvasNodeInstanceState) {
        let template = CanvasNodeTemplate {
            type_id: "image_gen".to_string(),
            title: "Image".to_string(),
            subtitle: "image_gen".to_string(),
            category: "image/generation".to_string(),
            inputs: vec![CanvasNodePortTemplate::new(
                "prompt",
                "prompt",
                CanvasPortSide::Input,
            )],
            outputs: vec![CanvasNodePortTemplate::new(
                "image",
                "image",
                CanvasPortSide::Output,
            )],
            params: vec![CanvasNodeParamTemplate::new(
                "prompt",
                "prompt",
                "string",
                "text",
                ParamControlSpec::default(),
            )],
        };
        let state = CanvasNodeInstanceState {
            owner_id: "engine_node::7".to_string(),
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
                user_min_height: None,
            },
            input_group: Default::default(),
            output_group: Default::default(),
            port_states: vec![
                CanvasNodePortState {
                    key: "prompt".to_string(),
                    side: CanvasPortSide::Input,
                    connection_state: CanvasPortConnectionState::Idle,
                },
                CanvasNodePortState {
                    key: "image".to_string(),
                    side: CanvasPortSide::Output,
                    connection_state: CanvasPortConnectionState::Idle,
                },
            ],
        };
        (template, state)
    }
}
