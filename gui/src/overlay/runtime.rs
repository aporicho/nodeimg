use super::OverlayRequest;

pub(crate) struct OverlayState {
    pub(crate) request: OverlayRequest,
    pub(crate) last_x: f32,
    pub(crate) last_y: f32,
    pub(crate) last_width: Option<f32>,
}
