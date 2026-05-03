use super::model::{TextBoxSpec, TextBoxValueKind};
use crate::control::{format_number, ControlSpec, TextBoxFont, TextBoxMode};
use crate::theme::{ControlSize, Density, Theme};

pub(super) fn retained_control_text_box_spec(
    control: &ControlSpec,
    theme: &Theme,
) -> Option<TextBoxSpec> {
    let tokens = theme.text_field_metrics(ControlSize::Small, Density::Compact);
    match control {
        ControlSpec::Text { value } => Some(TextBoxSpec {
            external_text: value.clone(),
            mode: TextBoxMode::SingleLine,
            value_kind: TextBoxValueKind::Text,
            tokens,
            font: TextBoxFont::Body,
            disabled: false,
        }),
        ControlSpec::TextArea { value, min_rows } => Some(TextBoxSpec {
            external_text: value.clone(),
            mode: TextBoxMode::MultiLine {
                min_rows: *min_rows,
            },
            value_kind: TextBoxValueKind::Text,
            tokens,
            font: TextBoxFont::Body,
            disabled: false,
        }),
        ControlSpec::Number {
            value,
            min,
            max,
            step,
            precision,
        } => Some(TextBoxSpec {
            external_text: format_number(*value, *precision),
            mode: TextBoxMode::SingleLine,
            value_kind: TextBoxValueKind::Number {
                value: *value,
                min: *min,
                max: *max,
                step: *step,
                precision: *precision,
            },
            tokens,
            font: TextBoxFont::Mono,
            disabled: false,
        }),
        _ => None,
    }
}
