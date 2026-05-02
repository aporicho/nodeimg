use super::PanelConfig;
use crate::control::ResizeEdge;
use crate::renderer::Rect;
use crate::tree::RuntimeSlot;

#[derive(Clone, Debug)]
pub struct PanelRuntime {
    pub rect: Rect,
    pub min_size: [f32; 2],
    pub visible: bool,
    pub z_index: i32,
    pub collapsed: bool,
}

impl PanelRuntime {
    pub(crate) fn from_config(config: &PanelConfig, z_index: i32) -> Self {
        Self {
            rect: config.default_rect,
            min_size: config.min_size,
            visible: config.initially_visible,
            z_index,
            collapsed: false,
        }
    }
}

impl Default for PanelRuntime {
    fn default() -> Self {
        Self {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            min_size: [0.0, 0.0],
            visible: false,
            z_index: 0,
            collapsed: false,
        }
    }
}

impl RuntimeSlot for PanelRuntime {}

#[derive(Clone, Debug)]
pub(crate) struct PanelPointerSession {
    pub(crate) id: String,
    pub(crate) last_x: f32,
    pub(crate) last_y: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct PanelResizeSession {
    pub(crate) id: String,
    pub(crate) edge: ResizeEdge,
    pub(crate) last_x: f32,
    pub(crate) last_y: f32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PanelRootRuntime {
    pub(crate) focused: Option<String>,
    pub(crate) next_z: i32,
    pub(crate) active_drag: Option<PanelPointerSession>,
    pub(crate) active_resize: Option<PanelResizeSession>,
}

impl RuntimeSlot for PanelRootRuntime {}
