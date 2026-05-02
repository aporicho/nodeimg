use super::output::changed_number_output;
use super::system::TextBoxSystem;
use crate::control::format_number;
use crate::control::state::TextBoxValueKind;
use crate::output::FrameworkOutput;

impl TextBoxSystem {
    pub(super) fn step_number_input(
        &mut self,
        control_id: &str,
        direction: f32,
    ) -> FrameworkOutput {
        let Some(runtime) = self.store.text_box_mut(control_id) else {
            return FrameworkOutput::default();
        };
        let TextBoxValueKind::Number {
            value,
            min,
            max,
            step,
            precision,
        } = runtime.value_kind()
        else {
            return FrameworkOutput::default();
        };

        let current = parse_number_text(runtime.editor().text()).unwrap_or(value);
        let next = (current + step * direction).clamp(min, max);
        runtime.clear_preedit();
        runtime
            .editor_mut()
            .set_text(&format_number(next, precision));
        changed_number_output(control_id, next, value).with_consumed(true)
    }

    pub(super) fn finalize_number_input(&mut self, control_id: &str) -> FrameworkOutput {
        let Some(runtime) = self.store.text_box_mut(control_id) else {
            return FrameworkOutput::default();
        };
        let TextBoxValueKind::Number {
            value, min, max, ..
        } = runtime.value_kind()
        else {
            return FrameworkOutput::default();
        };

        match parse_number_text(runtime.editor().text()) {
            Some(parsed) if parsed >= min && parsed <= max => {
                let output = changed_number_output(control_id, parsed, value);
                if output.events.is_empty() {
                    runtime.revert_to_external();
                }
                output.with_consumed(true)
            }
            _ => {
                runtime.revert_to_external();
                FrameworkOutput::consumed()
            }
        }
    }
}

pub(super) fn live_number_output(
    control_id: &str,
    text: &str,
    external_value: f32,
    min: f32,
    max: f32,
) -> FrameworkOutput {
    parse_number_text(text)
        .filter(|value| *value >= min && *value <= max)
        .map(|value| changed_number_output(control_id, value, external_value))
        .unwrap_or_default()
}

pub(super) fn parse_number_text(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if trimmed.is_empty() || matches!(trimmed, "+" | "-" | "." | "+." | "-.") {
        return None;
    }
    trimmed.parse::<f32>().ok()
}
