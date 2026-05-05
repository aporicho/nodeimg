use crate::control::ControlKind;
use crate::tree::layout::Align;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlHeight {
    Fixed(f32),
    Fill { min_height: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlLayoutPolicy {
    pub kind: ControlKind,
    pub height: ControlHeight,
    pub row_align: Align,
    pub wrapper_align: Align,
    pub affects_parent_height: bool,
}

impl ControlLayoutPolicy {
    pub fn min_height(self) -> f32 {
        match self.height {
            ControlHeight::Fixed(height) => height,
            ControlHeight::Fill { min_height } => min_height,
        }
    }

    pub fn fills_parent_height(self) -> bool {
        matches!(self.height, ControlHeight::Fill { .. })
    }
}
