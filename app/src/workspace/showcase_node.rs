use gui::canvas::node_card::{CanvasNodeParamView, CanvasNodeView};
use gui::canvas::param_control::CanvasNodeParamControl;
use gui::canvas::{
    canvas_port_stable_id, CanvasNodeIdentity, CanvasNodeLayout, CanvasPortConnectionState,
    CanvasPortGroupView, CanvasPortSide, CanvasPortView,
};
use gui::renderer::Rect;

pub(crate) const SHOWCASE_OWNER_ID: &str = "showcase_node::all_controls";
pub(crate) const SOLO_OWNER_ID: &str = "showcase_node::solo_control";

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

pub(crate) fn is_showcase_node(owner_id: &str) -> bool {
    owner_id == SHOWCASE_OWNER_ID || owner_id == SOLO_OWNER_ID
}

pub(crate) fn showcase_view_for_layout(layout: CanvasNodeLayout) -> Option<CanvasNodeView> {
    match layout.owner_id.as_str() {
        SHOWCASE_OWNER_ID => Some(showcase_node_view(layout)),
        SOLO_OWNER_ID => Some(solo_node_view(layout)),
        _ => None,
    }
}

pub(crate) fn showcase_node_view(layout: CanvasNodeLayout) -> CanvasNodeView {
    CanvasNodeView {
        owner_id: SHOWCASE_OWNER_ID.to_string(),
        title: "Node Control Showcase".to_string(),
        subtitle: "all canvas param controls".to_string(),
        category: "ui/showcase".to_string(),
        params: vec![
            CanvasNodeParamView {
                name: "Readonly".to_string(),
                kind: "image".to_string(),
                value: "image".to_string(),
                control: CanvasNodeParamControl::ReadOnly {
                    value: "image".to_string(),
                },
            },
            CanvasNodeParamView {
                name: "Prompt".to_string(),
                kind: "string".to_string(),
                value: "A long prompt value".to_string(),
                control: CanvasNodeParamControl::Text {
                    value: "A long prompt value".to_string(),
                },
            },
            CanvasNodeParamView {
                name: "Seed".to_string(),
                kind: "int".to_string(),
                value: "42".to_string(),
                control: CanvasNodeParamControl::Number {
                    value: 42.0,
                    min: 0.0,
                    max: 9999.0,
                    step: 1.0,
                    precision: 0,
                },
            },
            CanvasNodeParamView {
                name: "Strength".to_string(),
                kind: "float".to_string(),
                value: "0.65".to_string(),
                control: CanvasNodeParamControl::Slider {
                    value: 0.65,
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                },
            },
            CanvasNodeParamView {
                name: "Enabled".to_string(),
                kind: "bool".to_string(),
                value: "true".to_string(),
                control: CanvasNodeParamControl::Toggle { checked: true },
            },
            CanvasNodeParamView {
                name: "Sampler".to_string(),
                kind: "enum".to_string(),
                value: "Euler".to_string(),
                control: CanvasNodeParamControl::Select {
                    options: vec![
                        "Euler".to_string(),
                        "DPM++ 2M".to_string(),
                        "UniPC".to_string(),
                    ],
                    selected: 0,
                },
            },
            CanvasNodeParamView {
                name: "Tint".to_string(),
                kind: "color".to_string(),
                value: "#FF8040".to_string(),
                control: CanvasNodeParamControl::Color {
                    rgba: [1.0, 0.5, 0.25, 1.0],
                },
            },
            CanvasNodeParamView {
                name: "Output".to_string(),
                kind: "file_path".to_string(),
                value: "*.png".to_string(),
                control: CanvasNodeParamControl::FilePath {
                    path: String::new(),
                    extensions: vec!["png".to_string(), "jpg".to_string()],
                },
            },
        ],
        input_group: CanvasPortGroupView { open: true },
        output_group: CanvasPortGroupView { open: true },
        inputs: showcase_ports(
            SHOWCASE_OWNER_ID,
            CanvasPortSide::Input,
            &["image", "mask", "prompt", "strength", "seed", "enabled"],
        ),
        outputs: showcase_ports(
            SHOWCASE_OWNER_ID,
            CanvasPortSide::Output,
            &["image", "preview", "metadata"],
        ),
        selected: false,
        layout,
    }
}

pub(crate) fn solo_node_view(layout: CanvasNodeLayout) -> CanvasNodeView {
    CanvasNodeView {
        owner_id: SOLO_OWNER_ID.to_string(),
        title: "Solo Control".to_string(),
        subtitle: "single control tuning".to_string(),
        category: "ui/showcase".to_string(),
        params: vec![CanvasNodeParamView {
            name: "Strength".to_string(),
            kind: "float".to_string(),
            value: "0.65".to_string(),
            control: CanvasNodeParamControl::Slider {
                value: 0.65,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
        }],
        input_group: CanvasPortGroupView { open: true },
        output_group: CanvasPortGroupView { open: true },
        inputs: showcase_ports(SOLO_OWNER_ID, CanvasPortSide::Input, &["in"]),
        outputs: showcase_ports(SOLO_OWNER_ID, CanvasPortSide::Output, &["out"]),
        selected: false,
        layout,
    }
}

fn showcase_ports(owner_id: &str, side: CanvasPortSide, names: &[&str]) -> Vec<CanvasPortView> {
    let count = names.len();
    names
        .iter()
        .enumerate()
        .map(|(index, name)| CanvasPortView {
            name: (*name).to_string(),
            stable_id: canvas_port_stable_id(owner_id, side, name),
            side,
            index,
            count,
            connection_state: CanvasPortConnectionState::Idle,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showcase_node_contains_all_current_param_control_shapes() {
        let identity = showcase_node_identity();
        let view = showcase_node_view(CanvasNodeLayout {
            owner_id: identity.owner_id,
            rect: identity.default_rect,
            z_index: 0,
            collapsed: false,
        });

        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::ReadOnly { .. })));
        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::Text { .. })));
        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::Number { .. })));
        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::Slider { .. })));
        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::Toggle { .. })));
        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::Select { .. })));
        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::Color { .. })));
        assert!(view
            .params
            .iter()
            .any(|param| matches!(param.control, CanvasNodeParamControl::FilePath { .. })));
    }

    #[test]
    fn solo_node_contains_one_tunable_control() {
        let identity = solo_node_identity();
        let view = solo_node_view(CanvasNodeLayout {
            owner_id: identity.owner_id,
            rect: identity.default_rect,
            z_index: 0,
            collapsed: false,
        });

        assert_eq!(view.owner_id, SOLO_OWNER_ID);
        assert_eq!(view.params.len(), 1);
        assert!(matches!(
            view.params[0].control,
            CanvasNodeParamControl::Slider { .. }
        ));
        assert_eq!(view.inputs.len(), 1);
        assert_eq!(view.outputs.len(), 1);
    }
}
