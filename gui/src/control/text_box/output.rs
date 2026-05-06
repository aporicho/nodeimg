use super::model::TextBoxValueKind;
use super::number::live_number_output;
use super::system::TextBoxSystem;
use crate::control::{parse_color_hex, ControlValue, SystemCx};
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
        TextBoxValueKind::Color { rgba } => {
            changed_color_output(control_id, runtime.editor().text(), rgba)
        }
        TextBoxValueKind::FilePath => {
            changed_file_path_output(control_id, runtime.editor().text(), runtime.external_text())
        }
    }
}

pub(super) fn changed_text_output(control_id: &str, value: &str) -> FrameworkOutput {
    OutputBuilder::new()
        .control(ControlEvent::ValueChanged {
            id: control_id.to_string(),
            value: ControlValue::Text(value.to_string()),
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
            .control(ControlEvent::ValueChanged {
                id: control_id.to_string(),
                value: ControlValue::Number(value),
            })
            .finish()
    }
}

pub(super) fn changed_color_output(
    control_id: &str,
    text: &str,
    external_rgba: [f32; 4],
) -> FrameworkOutput {
    let Some(next) = parse_color_hex(text, external_rgba[3]) else {
        return FrameworkOutput::default();
    };
    if colors_match(next, external_rgba) {
        return FrameworkOutput::default();
    }
    OutputBuilder::new()
        .control(ControlEvent::ValueChanged {
            id: control_id.to_string(),
            value: ControlValue::Color(next),
        })
        .finish()
}

pub(super) fn changed_file_path_output(
    control_id: &str,
    text: &str,
    external_text: &str,
) -> FrameworkOutput {
    if text == external_text {
        return FrameworkOutput::default();
    }
    OutputBuilder::new()
        .control(ControlEvent::ValueChanged {
            id: control_id.to_string(),
            value: ControlValue::FilePath(text.to_string()),
        })
        .finish()
}

fn colors_match(left: [f32; 4], right: [f32; 4]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| (left - right).abs() <= 0.000_001)
}
