use super::OverlayRequest;
use crate::tree::NodeId;

pub(crate) struct OverlayState {
    pub(crate) request: OverlayRequest,
    pub(crate) last_x: f32,
    pub(crate) last_y: f32,
    pub(crate) last_width: Option<f32>,
    pub(crate) root: Option<NodeId>,
    pub(crate) dirty: bool,
}
