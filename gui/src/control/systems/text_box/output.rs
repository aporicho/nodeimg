use super::number::live_number_output;
use super::system::TextBoxSystem;
use crate::control::state::TextBoxValueKind;
use crate::control::systems::SystemCx;
use crate::output::{ControlEvent, FrameworkOutput, OutputBuilder};
use crate::tree::Tree;

impl TextBoxSystem {
    pub(super) fn output_for_editor_for(
        &self,
        cx: &SystemCx<'_>,
        control_id: &str,
    ) -> FrameworkOutput {
        output_for_editor(self, cx.tree(), control_id)
    }
}

pub(super) fn output_for_editor(
    system: &TextBoxSystem,
    _tree: &Tree,
    control_id: &str,
) -> FrameworkOutput {
    let Some(runtime) = system.store.text_box(control_id) else {
        return FrameworkOutput::default();
    };

    match runtime.value_kind() {
        TextBoxValueKind::Text => changed_text_output(control_id, runtime.editor().text()),
        TextBoxValueKind::Number {
            value, min, max, ..
        } => live_number_output(control_id, runtime.editor().text(), value, min, max),
    }
}

pub(super) fn changed_text_output(control_id: &str, value: &str) -> FrameworkOutput {
    OutputBuilder::new()
        .control(ControlEvent::TextChanged {
            id: control_id.to_string(),
            value: value.to_string(),
        })
        .finish()
}

pub(super) fn changed_number_output(
    control_id: &str,
    value: f32,
    external_value: f32,
) -> FrameworkOutput {
    if (value - external_value).abs() < f32::EPSILON {
        FrameworkOutput::default()
    } else {
        OutputBuilder::new()
            .control(ControlEvent::NumberChanged {
                id: control_id.to_string(),
                value,
            })
            .finish()
    }
}
