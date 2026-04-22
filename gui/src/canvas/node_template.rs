use super::{canvas_port_stable_id, CanvasNodeLayout, CanvasPortConnectionState, CanvasPortSide};
use crate::widget::mapping::ParamControlSpec;

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
    pub control: ParamControlSpec,
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
}

impl CanvasNodePortTemplate {
    pub fn new(key: impl Into<String>, name: impl Into<String>, side: CanvasPortSide) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            side,
        }
    }
}

impl CanvasNodeParamTemplate {
    pub fn new(
        key: impl Into<String>,
        name: impl Into<String>,
        kind: impl Into<String>,
        default_value: impl Into<String>,
        control: ParamControlSpec,
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
