use crate::theme::{ControlSize, Density, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlMetrics {
    pub control_height: f32,
    pub control_width: f32,
    pub size: ControlSize,
    pub density: Density,
}

impl ControlMetrics {
    pub fn from_theme(_theme: &Theme) -> Self {
        Self {
            control_height: 24.0,
            control_width: 128.0,
            size: ControlSize::Small,
            density: Density::Compact,
        }
    }
}
