use super::{
    canvas_port_stable_id, CanvasNodeLayout, CanvasPortConnectionState, CanvasPortSide,
    CanvasPortView,
};
use crate::canvas::node_card::CanvasNodeView;
use crate::canvas::param_control::CanvasNodeParamControl;

#[derive(Clone, Debug)]
pub struct CanvasNodeRenderView {
    pub template: CanvasNodeTemplate,
    pub state: CanvasNodeInstanceState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasNodeTemplate {
    pub type_id: String,
    pub title: String,
    pub subtitle: String,
    pub category: String,
    pub inputs: Vec<CanvasNodePortTemplate>,
    pub outputs: Vec<CanvasNodePortTemplate>,
    pub params: Vec<CanvasNodeParamTemplate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasNodePortTemplate {
    pub key: String,
    pub name: String,
    pub side: CanvasPortSide,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasNodeParamTemplate {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub default_value: String,
    pub control: CanvasNodeParamControl,
}

#[derive(Clone, Debug)]
pub struct CanvasNodeInstanceState {
    pub owner_id: String,
    pub layout: CanvasNodeLayout,
    pub selected: bool,
    pub port_states: Vec<CanvasNodePortState>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasNodePortState {
    pub key: String,
    pub side: CanvasPortSide,
    pub connection_state: CanvasPortConnectionState,
}

impl CanvasNodeTemplate {
    pub fn port_templates(&self, side: CanvasPortSide) -> &[CanvasNodePortTemplate] {
        match side {
            CanvasPortSide::Input => &self.inputs,
            CanvasPortSide::Output => &self.outputs,
        }
    }

    pub fn from_legacy_view(view: &CanvasNodeView) -> Self {
        Self {
            type_id: view.subtitle.clone(),
            title: view.title.clone(),
            subtitle: view.subtitle.clone(),
            category: view.category.clone(),
            inputs: view
                .inputs
                .iter()
                .map(CanvasNodePortTemplate::from_legacy_port)
                .collect(),
            outputs: view
                .outputs
                .iter()
                .map(CanvasNodePortTemplate::from_legacy_port)
                .collect(),
            params: view
                .params
                .iter()
                .map(|param| CanvasNodeParamTemplate {
                    key: param.name.clone(),
                    name: param.name.clone(),
                    kind: param.kind.clone(),
                    default_value: if param.value.is_empty() {
                        param.kind.clone()
                    } else {
                        param.value.clone()
                    },
                    control: param.control.clone(),
                })
                .collect(),
        }
    }
}

impl CanvasNodePortTemplate {
    pub fn new(key: impl Into<String>, name: impl Into<String>, side: CanvasPortSide) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            side,
        }
    }

    pub fn from_legacy_port(port: &CanvasPortView) -> Self {
        Self {
            key: port.name.clone(),
            name: port.name.clone(),
            side: port.side,
        }
    }
}

impl CanvasNodeParamTemplate {
    pub fn new(
        key: impl Into<String>,
        name: impl Into<String>,
        kind: impl Into<String>,
        default_value: impl Into<String>,
        control: CanvasNodeParamControl,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            kind: kind.into(),
            default_value: default_value.into(),
            control,
        }
    }
}

impl CanvasNodeInstanceState {
    pub fn from_legacy_view(view: &CanvasNodeView) -> Self {
        let mut port_states = view
            .inputs
            .iter()
            .chain(view.outputs.iter())
            .map(|port| CanvasNodePortState {
                key: port.name.clone(),
                side: port.side,
                connection_state: port.connection_state,
            })
            .collect::<Vec<_>>();
        port_states.sort_by(|a, b| {
            a.side
                .as_str()
                .cmp(b.side.as_str())
                .then_with(|| a.key.cmp(&b.key))
        });

        Self {
            owner_id: view.owner_id.clone(),
            layout: view.layout.clone(),
            selected: view.selected,
            port_states,
        }
    }

    pub fn connection_state(&self, side: CanvasPortSide, key: &str) -> CanvasPortConnectionState {
        self.port_states
            .iter()
            .find(|state| state.side == side && state.key == key)
            .map(|state| state.connection_state)
            .unwrap_or(CanvasPortConnectionState::Idle)
    }

    pub fn port_id(&self, side: CanvasPortSide, key: &str) -> String {
        canvas_port_stable_id(&self.owner_id, side, key)
    }
}
