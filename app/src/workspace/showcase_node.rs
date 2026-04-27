use gui::canvas::node_template::{
    CanvasNodeInstanceState, CanvasNodeParamTemplate, CanvasNodePortState, CanvasNodePortTemplate,
    CanvasNodeRenderView, CanvasNodeTemplate,
};
use gui::canvas::{
    CanvasNodeIdentity, CanvasNodeLayout, CanvasPortConnectionState, CanvasPortSide,
};
use gui::renderer::Rect;
use gui::widget::mapping::ParamControlSpec;

pub(crate) const SHOWCASE_OWNER_ID: &str = "showcase_node::all_controls";
pub(crate) const SOLO_OWNER_ID: &str = "showcase_node::solo_control";
pub(crate) const TEXT_AREA_OWNER_ID: &str = "showcase_node::text_area_control";

pub(crate) fn showcase_node_identity() -> CanvasNodeIdentity {
    CanvasNodeIdentity {
        owner_id: SHOWCASE_OWNER_ID.to_string(),
        default_rect: Rect {
            x: -520.0,
            y: 0.0,
            w: 304.0,
            h: 420.0,
        },
    }
}

pub(crate) fn solo_node_identity() -> CanvasNodeIdentity {
    CanvasNodeIdentity {
        owner_id: SOLO_OWNER_ID.to_string(),
        default_rect: Rect {
            x: -1040.0,
            y: 0.0,
            w: 304.0,
            h: 132.0,
        },
    }
}

pub(crate) fn text_area_node_identity() -> CanvasNodeIdentity {
    CanvasNodeIdentity {
        owner_id: TEXT_AREA_OWNER_ID.to_string(),
        default_rect: Rect {
            x: -1040.0,
            y: 180.0,
            w: 304.0,
            h: 180.0,
        },
    }
}

pub(crate) fn is_showcase_node(owner_id: &str) -> bool {
    owner_id == SHOWCASE_OWNER_ID || owner_id == SOLO_OWNER_ID || owner_id == TEXT_AREA_OWNER_ID
}

pub(crate) fn showcase_render_view_for_layout_with_text(
    layout: CanvasNodeLayout,
    text_area_value: &str,
) -> Option<CanvasNodeRenderView> {
    let template = match layout.owner_id.as_str() {
        SHOWCASE_OWNER_ID => showcase_node_template(),
        SOLO_OWNER_ID => solo_node_template(),
        TEXT_AREA_OWNER_ID => text_area_node_template(text_area_value),
        _ => return None,
    };
    let state = showcase_instance_state(&template, layout);
    Some(CanvasNodeRenderView { template, state })
}

pub(crate) fn showcase_node_template() -> CanvasNodeTemplate {
    CanvasNodeTemplate {
        type_id: "showcase_node::all_controls".to_string(),
        title: "Node Control Showcase".to_string(),
        subtitle: "all canvas param controls".to_string(),
        category: "ui/showcase".to_string(),
        params: vec![
            CanvasNodeParamTemplate::new(
                "Readonly",
                "Readonly",
                "image",
                "image",
                ParamControlSpec::ReadOnly {
                    value: "image".to_string(),
                },
            ),
            CanvasNodeParamTemplate::new(
                "Prompt",
                "Prompt",
                "string",
                "A long prompt value",
                ParamControlSpec::Text {
                    value: "A long prompt value".to_string(),
                },
            ),
            CanvasNodeParamTemplate::new(
                "Seed",
                "Seed",
                "int",
                "42",
                ParamControlSpec::Number {
                    value: 42.0,
                    min: 0.0,
                    max: 9999.0,
                    step: 1.0,
                    precision: 0,
                },
            ),
            CanvasNodeParamTemplate::new(
                "Strength",
                "Strength",
                "float",
                "0.65",
                ParamControlSpec::Slider {
                    value: 0.65,
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                },
            ),
            CanvasNodeParamTemplate::new(
                "Enabled",
                "Enabled",
                "bool",
                "true",
                ParamControlSpec::Toggle { checked: true },
            ),
            CanvasNodeParamTemplate::new(
                "Sampler",
                "Sampler",
                "enum",
                "Euler",
                ParamControlSpec::Select {
                    options: vec![
                        "Euler".to_string(),
                        "DPM++ 2M".to_string(),
                        "UniPC".to_string(),
                    ],
                    selected: 0,
                },
            ),
            CanvasNodeParamTemplate::new(
                "Tint",
                "Tint",
                "color",
                "#FF8040",
                ParamControlSpec::Color {
                    rgba: [1.0, 0.5, 0.25, 1.0],
                },
            ),
            CanvasNodeParamTemplate::new(
                "Output",
                "Output",
                "file_path",
                "*.png",
                ParamControlSpec::FilePath {
                    path: String::new(),
                    extensions: vec!["png".to_string(), "jpg".to_string()],
                },
            ),
        ],
        inputs: showcase_ports(
            CanvasPortSide::Input,
            &["image", "mask", "prompt", "strength", "seed", "enabled"],
        ),
        outputs: showcase_ports(CanvasPortSide::Output, &["image", "preview", "metadata"]),
    }
}

pub(crate) fn solo_node_template() -> CanvasNodeTemplate {
    CanvasNodeTemplate {
        type_id: "showcase_node::solo_control".to_string(),
        title: "Solo Control".to_string(),
        subtitle: "single control tuning".to_string(),
        category: "ui/showcase".to_string(),
        params: vec![CanvasNodeParamTemplate::new(
            "Strength",
            "Strength",
            "float",
            "0.65",
            ParamControlSpec::Slider {
                value: 0.65,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
        )],
        inputs: showcase_ports(CanvasPortSide::Input, &["in"]),
        outputs: showcase_ports(CanvasPortSide::Output, &["out"]),
    }
}

pub(crate) fn text_area_node_template(value: &str) -> CanvasNodeTemplate {
    CanvasNodeTemplate {
        type_id: "showcase_node::text_area_control".to_string(),
        title: "Text Area".to_string(),
        subtitle: "single text area control".to_string(),
        category: "ui/showcase".to_string(),
        params: vec![CanvasNodeParamTemplate::new(
            "Prompt",
            "",
            "",
            "",
            ParamControlSpec::TextArea {
                value: value.to_string(),
                min_rows: 5,
            },
        )],
        inputs: Vec::new(),
        outputs: Vec::new(),
    }
}

fn showcase_ports(side: CanvasPortSide, names: &[&str]) -> Vec<CanvasNodePortTemplate> {
    names
        .iter()
        .map(|name| CanvasNodePortTemplate::new(*name, *name, side))
        .collect()
}

fn showcase_instance_state(
    template: &CanvasNodeTemplate,
    layout: CanvasNodeLayout,
) -> CanvasNodeInstanceState {
    CanvasNodeInstanceState {
        owner_id: layout.owner_id.clone(),
        layout,
        selected: false,
        input_group: Default::default(),
        output_group: Default::default(),
        port_states: template
            .inputs
            .iter()
            .chain(template.outputs.iter())
            .map(|port| CanvasNodePortState {
                key: port.key.clone(),
                side: port.side,
                connection_state: CanvasPortConnectionState::Idle,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showcase_node_contains_all_current_param_control_shapes() {
        let template = showcase_node_template();

        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::ReadOnly { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::Text { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::Number { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::Slider { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::Toggle { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::Select { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::Color { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ParamControlSpec::FilePath { .. })));
    }

    #[test]
    fn solo_node_contains_one_tunable_control() {
        let identity = solo_node_identity();
        let view = showcase_render_view_for_layout_with_text(
            CanvasNodeLayout {
                owner_id: identity.owner_id,
                rect: identity.default_rect,
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            "A compact text field",
        )
        .expect("solo render view");

        assert_eq!(view.state.owner_id, SOLO_OWNER_ID);
        assert_eq!(view.template.params.len(), 1);
        assert!(matches!(
            view.template.params[0].control,
            ParamControlSpec::Slider { .. }
        ));
        assert_eq!(view.template.inputs.len(), 1);
        assert_eq!(view.template.outputs.len(), 1);
    }

    #[test]
    fn text_area_node_contains_only_one_text_area_control() {
        let identity = text_area_node_identity();
        let view = showcase_render_view_for_layout_with_text(
            CanvasNodeLayout {
                owner_id: identity.owner_id,
                rect: identity.default_rect,
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            "A compact text field",
        )
        .expect("text area render view");

        assert_eq!(view.state.owner_id, TEXT_AREA_OWNER_ID);
        assert_eq!(view.template.params.len(), 1);
        assert!(matches!(
            view.template.params[0].control,
            ParamControlSpec::TextArea { min_rows: 5, .. }
        ));
        assert!(view.template.params[0].name.is_empty());
        assert!(view.template.params[0].default_value.is_empty());
        assert!(view.template.inputs.is_empty());
        assert!(view.template.outputs.is_empty());
    }
}
