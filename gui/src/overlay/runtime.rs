use super::OverlayRequest;

pub(crate) struct OverlayState {
    pub(crate) request: OverlayRequest,
    #[cfg(test)]
    pub(crate) last_x: f32,
    #[cfg(test)]
    pub(crate) last_y: f32,
    #[cfg(test)]
    pub(crate) last_width: Option<f32>,
}
