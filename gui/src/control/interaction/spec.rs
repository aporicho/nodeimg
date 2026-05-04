use crate::tree::RuntimeSlot;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum ControlInteractionSpec {
    #[default]
    None,
    Toggle {
        checked: bool,
    },
    Slider {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
}

impl ControlInteractionSpec {
    pub(crate) fn toggle(checked: bool) -> Self {
        Self::Toggle { checked }
    }

    pub(crate) fn slider(value: f32, min: f32, max: f32, step: f32) -> Self {
        Self::Slider {
            value,
            min,
            max,
            step,
        }
    }

    pub(crate) fn is_interactive(self) -> bool {
        !matches!(self, Self::None)
    }
}

impl RuntimeSlot for ControlInteractionSpec {}
