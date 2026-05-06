use gui::canvas::node_template::{
    CanvasNodeInstanceState, CanvasNodeParamTemplate, CanvasNodePortState, CanvasNodePortTemplate,
    CanvasNodeRenderView, CanvasNodeTemplate,
};
use gui::canvas::{
    CanvasNodeIdentity, CanvasNodeLayout, CanvasPortConnectionState, CanvasPortSide,
};
use gui::control::{ControlNode, ControlSpec};
use gui::layout::TextureHandle;
use gui::renderer::{ImageFit, ImageStyle, Rect};

use super::showcase_state::{ShowcaseState, SAMPLER_OPTIONS};

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

pub(crate) fn showcase_render_view_for_layout(
    layout: CanvasNodeLayout,
    state: &ShowcaseState,
) -> Option<CanvasNodeRenderView> {
    let template = match layout.owner_id.as_str() {
        SHOWCASE_OWNER_ID => showcase_node_template(state),
        SOLO_OWNER_ID => solo_node_template(state),
        TEXT_AREA_OWNER_ID => text_area_node_template(state),
        _ => return None,
    };
    let state = showcase_instance_state(&template, layout);
    Some(CanvasNodeRenderView { template, state })
}

pub(crate) fn showcase_node_template(state: &ShowcaseState) -> CanvasNodeTemplate {
    CanvasNodeTemplate {
        type_id: "showcase_node::all_controls".to_string(),
        title: "Node Control Showcase".to_string(),
        subtitle: "all canvas param controls".to_string(),
        category: "ui/showcase".to_string(),
        params: vec![
            CanvasNodeParamTemplate::new(
                "Action",
                "Action",
                "button",
                "Run",
                ControlSpec::Button {
                    label: "Run".to_string(),
                },
            ),
            CanvasNodeParamTemplate::new(
                "Status",
                "Status",
                "label",
                "Ready",
                ControlSpec::Label {
                    text: "Ready".to_string(),
                    muted: false,
                },
            ),
            CanvasNodeParamTemplate::new(
                "Readonly",
                "Readonly",
                "image",
                "image",
                ControlSpec::ReadOnly {
                    value: "image".to_string(),
                },
            ),
            CanvasNodeParamTemplate::new(
                "Prompt",
                "Prompt",
                "string",
                "A long prompt value",
                ControlSpec::Text {
                    value: state.prompt().to_string(),
                },
            ),
            CanvasNodeParamTemplate::new(
                "Seed",
                "Seed",
                "int",
                "42",
                ControlSpec::Number {
                    value: state.seed(),
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
                ControlSpec::Slider {
                    value: state.strength(),
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
                ControlSpec::Toggle {
                    checked: state.enabled(),
                },
            ),
            CanvasNodeParamTemplate::new(
                "Sampler",
                "Sampler",
                "enum",
                "Euler",
                ControlSpec::Select {
                    options: SAMPLER_OPTIONS
                        .iter()
                        .map(|option| option.to_string())
                        .collect(),
                    selected: state.sampler(),
                },
            ),
            CanvasNodeParamTemplate::new(
                "Tint",
                "Tint",
                "color",
                "#FF8040",
                ControlSpec::Color { rgba: state.tint() },
            ),
            CanvasNodeParamTemplate::new(
                "Output",
                "Output",
                "file_path",
                "*.png",
                ControlSpec::FilePath {
                    path: state.output_path().to_string(),
                    extensions: vec!["png".to_string(), "jpg".to_string()],
                },
            ),
            CanvasNodeParamTemplate::new(
                "Preview",
                "Preview",
                "image",
                "sample",
                ControlSpec::Image {
                    texture: TextureHandle(1),
                    image_style: ImageStyle::default().with_fit(ImageFit::Contain),
                    min_height: 96.0,
                },
            ),
            CanvasNodeParamTemplate::new(
                "Advanced",
                "Advanced",
                "group",
                "nested",
                ControlSpec::Group {
                    title: "Advanced".to_string(),
                    children: vec![
                        ControlNode::muted_label("hint", "Nested group"),
                        ControlNode::toggle("flag", state.enabled()),
                    ],
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

pub(crate) fn solo_node_template(state: &ShowcaseState) -> CanvasNodeTemplate {
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
            ControlSpec::Slider {
                value: state.solo_strength(),
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
        )],
        inputs: showcase_ports(CanvasPortSide::Input, &["in"]),
        outputs: showcase_ports(CanvasPortSide::Output, &["out"]),
    }
}

pub(crate) fn text_area_node_template(state: &ShowcaseState) -> CanvasNodeTemplate {
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
            ControlSpec::TextArea {
                value: state.text_area_prompt().to_string(),
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
    fn showcase_node_contains_all_current_control_shapes() {
        let state = ShowcaseState::default();
        let template = showcase_node_template(&state);

        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Button { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Label { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::ReadOnly { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Text { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Number { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Slider { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Toggle { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Select { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Color { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::FilePath { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Image { .. })));
        assert!(template
            .params
            .iter()
            .any(|param| matches!(param.control, ControlSpec::Group { .. })));
    }

    #[test]
    fn solo_node_contains_one_tunable_control() {
        let identity = solo_node_identity();
        let state = ShowcaseState::default();
        let view = showcase_render_view_for_layout(
            CanvasNodeLayout {
                owner_id: identity.owner_id,
                rect: identity.default_rect,
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            &state,
        )
        .expect("solo render view");

        assert_eq!(view.state.owner_id, SOLO_OWNER_ID);
        assert_eq!(view.template.params.len(), 1);
        assert!(matches!(
            view.template.params[0].control,
            ControlSpec::Slider { .. }
        ));
        assert_eq!(view.template.inputs.len(), 1);
        assert_eq!(view.template.outputs.len(), 1);
    }

    #[test]
    fn text_area_node_contains_only_one_text_area_control() {
        let identity = text_area_node_identity();
        let state = ShowcaseState::default();
        let view = showcase_render_view_for_layout(
            CanvasNodeLayout {
                owner_id: identity.owner_id,
                rect: identity.default_rect,
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            &state,
        )
        .expect("text area render view");

        assert_eq!(view.state.owner_id, TEXT_AREA_OWNER_ID);
        assert_eq!(view.template.params.len(), 1);
        assert!(matches!(
            view.template.params[0].control,
            ControlSpec::TextArea { min_rows: 5, .. }
        ));
        assert!(view.template.params[0].name.is_empty());
        assert!(view.template.params[0].default_value.is_empty());
        assert!(view.template.inputs.is_empty());
        assert!(view.template.outputs.is_empty());
    }
}
