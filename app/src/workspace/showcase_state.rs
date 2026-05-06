use gui::control::ControlValue;

pub(crate) const SAMPLER_OPTIONS: [&str; 3] = ["Euler", "DPM++ 2M", "UniPC"];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ShowcaseState {
    prompt: String,
    seed: f32,
    strength: f32,
    enabled: bool,
    sampler: usize,
    tint: [f32; 4],
    output_path: String,
    solo_strength: f32,
    text_area_prompt: String,
}

impl ShowcaseState {
    pub(crate) fn update_control_value(&mut self, id: &str, value: ControlValue) -> bool {
        let Some((owner_id, key)) = showcase_control_target(id) else {
            return false;
        };
        match (owner_id, key, value) {
            (super::showcase_node::SHOWCASE_OWNER_ID, "Prompt", ControlValue::Text(value)) => {
                update_if_changed(&mut self.prompt, value)
            }
            (super::showcase_node::SHOWCASE_OWNER_ID, "Seed", ControlValue::Number(value)) => {
                update_f32_if_changed(&mut self.seed, value)
            }
            (super::showcase_node::SHOWCASE_OWNER_ID, "Strength", ControlValue::Number(value)) => {
                update_f32_if_changed(&mut self.strength, value)
            }
            (super::showcase_node::SHOWCASE_OWNER_ID, "Enabled", ControlValue::Bool(value)) => {
                update_if_changed(&mut self.enabled, value)
            }
            (
                super::showcase_node::SHOWCASE_OWNER_ID,
                "Sampler",
                ControlValue::Selection(value),
            ) => {
                let value = value.min(SAMPLER_OPTIONS.len().saturating_sub(1));
                update_if_changed(&mut self.sampler, value)
            }
            (super::showcase_node::SHOWCASE_OWNER_ID, "Tint", ControlValue::Color(value)) => {
                update_if_changed(&mut self.tint, value)
            }
            (super::showcase_node::SHOWCASE_OWNER_ID, "Output", ControlValue::FilePath(value)) => {
                update_if_changed(&mut self.output_path, value)
            }
            (super::showcase_node::SOLO_OWNER_ID, "Strength", ControlValue::Number(value)) => {
                update_f32_if_changed(&mut self.solo_strength, value)
            }
            (super::showcase_node::TEXT_AREA_OWNER_ID, "Prompt", ControlValue::Text(value)) => {
                update_if_changed(&mut self.text_area_prompt, value)
            }
            _ => false,
        }
    }

    pub(crate) fn prompt(&self) -> &str {
        &self.prompt
    }

    pub(crate) fn seed(&self) -> f32 {
        self.seed
    }

    pub(crate) fn strength(&self) -> f32 {
        self.strength
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn sampler(&self) -> usize {
        self.sampler
    }

    pub(crate) fn tint(&self) -> [f32; 4] {
        self.tint
    }

    pub(crate) fn output_path(&self) -> &str {
        &self.output_path
    }

    pub(crate) fn solo_strength(&self) -> f32 {
        self.solo_strength
    }

    pub(crate) fn text_area_prompt(&self) -> &str {
        &self.text_area_prompt
    }
}

impl Default for ShowcaseState {
    fn default() -> Self {
        Self {
            prompt: "A long prompt value".to_string(),
            seed: 42.0,
            strength: 0.65,
            enabled: true,
            sampler: 0,
            tint: [1.0, 0.5, 0.25, 1.0],
            output_path: String::new(),
            solo_strength: 0.65,
            text_area_prompt: "A compact text field".to_string(),
        }
    }
}

fn showcase_control_target(id: &str) -> Option<(&str, &str)> {
    let owner_path = id.strip_prefix("canvas_node::")?;
    let (owner_id, rest) = owner_path.split_once("::body::param::")?;
    let (key, _) = rest.split_once("::control::content")?;
    Some((owner_id, key))
}

fn update_if_changed<T: PartialEq>(slot: &mut T, value: T) -> bool {
    if *slot == value {
        return false;
    }
    *slot = value;
    true
}

fn update_f32_if_changed(slot: &mut f32, value: f32) -> bool {
    if (*slot - value).abs() <= 0.000_001 {
        return false;
    }
    *slot = value;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_showcase_selection_from_control_id() {
        let mut state = ShowcaseState::default();
        assert!(state.update_control_value(
            "canvas_node::showcase_node::all_controls::body::param::Sampler::control::content",
            ControlValue::Selection(2),
        ));
        assert_eq!(state.sampler(), 2);
    }

    #[test]
    fn ignores_unknown_control_ids() {
        let mut state = ShowcaseState::default();
        assert!(!state.update_control_value("other", ControlValue::Bool(false)));
    }
}
