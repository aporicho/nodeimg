#[derive(Debug, Clone, Copy)]
pub enum OverlayPlacement {
    AtPoint { x: f32, y: f32 },
    BelowStart,
}
