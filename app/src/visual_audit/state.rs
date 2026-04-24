pub(crate) const POPUP_TRIGGER_ID: &str = "popup_trigger";
pub(crate) const POPUP_CLOSE_ID: &str = "popup_close";
pub(crate) const TEXT_PROMPT_ID: &str = "text_prompt";
pub(crate) const SLIDER_RADIUS_ID: &str = "slider_radius";
pub(crate) const NUMBER_RADIUS_ID: &str = "number_radius";
pub(crate) const TOGGLE_GRID_ID: &str = "toggle_grid";
pub(crate) const ADVANCED_TOGGLE_ID: &str = "advanced_toggle";
pub(crate) const CHECKBOX_SNAP_ID: &str = "checkbox_snap";
pub(crate) const RADIO_FAST_ID: &str = "radio_quality_fast";
pub(crate) const RADIO_BALANCED_ID: &str = "radio_quality_balanced";
pub(crate) const DROPDOWN_BLEND_ID: &str = "dropdown_blend";
pub(crate) const ADVANCED_SECTION_ID: &str = "advanced_section";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QualityMode {
    Fast,
    Balanced,
}

#[derive(Debug, Clone)]
pub(crate) struct VisualAuditState {
    pub text_value: String,
    pub toggle_value: bool,
    pub snap_to_grid: bool,
    pub quality_mode: QualityMode,
    pub advanced_open: bool,
    pub blend_mode: usize,
    pub slider_value: f32,
}

impl Default for VisualAuditState {
    fn default() -> Self {
        Self {
            text_value: "Hello nodeimg".to_string(),
            toggle_value: true,
            snap_to_grid: true,
            quality_mode: QualityMode::Balanced,
            advanced_open: true,
            blend_mode: 0,
            slider_value: 5.0,
        }
    }
}

impl VisualAuditState {
    pub(crate) fn apply_click(&mut self, id: &str) -> bool {
        match id {
            TOGGLE_GRID_ID | ADVANCED_TOGGLE_ID => {
                self.toggle_value = !self.toggle_value;
                true
            }
            CHECKBOX_SNAP_ID => {
                self.snap_to_grid = !self.snap_to_grid;
                true
            }
            RADIO_FAST_ID => {
                self.quality_mode = QualityMode::Fast;
                true
            }
            RADIO_BALANCED_ID => {
                self.quality_mode = QualityMode::Balanced;
                true
            }
            ADVANCED_SECTION_ID => {
                self.advanced_open = !self.advanced_open;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn apply_text_change(&mut self, id: &str, value: String) -> bool {
        match id {
            TEXT_PROMPT_ID => {
                self.text_value = value;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn apply_number_change(&mut self, id: &str, value: f32) -> bool {
        match id {
            NUMBER_RADIUS_ID => {
                self.slider_value = value;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn apply_select_change(&mut self, id: &str, selected: usize) -> bool {
        match id {
            DROPDOWN_BLEND_ID => {
                self.blend_mode = selected;
                true
            }
            _ => false,
        }
    }
}
