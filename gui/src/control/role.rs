#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlRole {
    #[default]
    Generic,
    Button,
    Checkbox,
    Collapsible,
    Dropdown,
    NumberInput,
    Panel,
    Radio,
    Slider,
    TextArea,
    TextInput,
    Toggle,
}

impl ControlRole {
    pub fn is_focusable(self) -> bool {
        matches!(
            self,
            Self::Button
                | Self::Checkbox
                | Self::Collapsible
                | Self::Dropdown
                | Self::NumberInput
                | Self::Radio
                | Self::Slider
                | Self::TextArea
                | Self::TextInput
                | Self::Toggle
        )
    }

    pub fn is_panel(self) -> bool {
        matches!(self, Self::Panel)
    }

    pub fn is_text_input(self) -> bool {
        matches!(self, Self::TextInput)
    }

    pub fn is_text_area(self) -> bool {
        matches!(self, Self::TextArea)
    }
}
