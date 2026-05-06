use super::model::TextBoxValueKind;
use super::number::parse_number_text;
use super::output::{
    changed_color_output, changed_file_path_output, changed_number_output, changed_text_output,
};
use super::retained_spec::retained_control_text_box_spec;
use crate::control::{ControlSpec, ControlValue, TextBoxFont, TextBoxMode};
use crate::output::{ControlEvent, GuiEvent};
use crate::theme::dark_theme;

#[test]
fn parse_number_text_rejects_incomplete_numbers() {
    assert_eq!(parse_number_text(""), None);
    assert_eq!(parse_number_text("-"), None);
    assert_eq!(parse_number_text("+."), None);
    assert_eq!(parse_number_text(" 42.5 "), Some(42.5));
}

#[test]
fn text_output_uses_unified_value_changed_event() {
    let output = changed_text_output("control::text", "hello");

    assert!(output.events.iter().any(|event| matches!(
        event,
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::Text(value)
        }) if id == "control::text" && value == "hello"
    )));
}

#[test]
fn number_output_uses_unified_value_changed_event() {
    let output = changed_number_output("control::number", 2.5, 1.0);

    assert!(output.events.iter().any(|event| matches!(
        event,
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::Number(value)
        }) if id == "control::number" && (*value - 2.5).abs() < f32::EPSILON
    )));
}

#[test]
fn color_output_uses_unified_value_changed_event_for_valid_hex() {
    let output = changed_color_output("control::color", "#336699", [1.0, 0.0, 0.0, 0.8]);

    assert!(output.events.iter().any(|event| matches!(
        event,
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::Color(value)
        }) if id == "control::color"
            && (value[0] - 51.0 / 255.0).abs() < 0.0001
            && (value[1] - 102.0 / 255.0).abs() < 0.0001
            && (value[2] - 153.0 / 255.0).abs() < 0.0001
            && (value[3] - 0.8).abs() < 0.0001
    )));
}

#[test]
fn color_output_ignores_invalid_hex() {
    let output = changed_color_output("control::color", "#GG0000", [1.0, 0.0, 0.0, 1.0]);

    assert!(output.events.is_empty());
}

#[test]
fn file_path_output_uses_unified_value_changed_event() {
    let output = changed_file_path_output("control::file", "/tmp/out.png", "");

    assert!(output.events.iter().any(|event| matches!(
        event,
        GuiEvent::Control(ControlEvent::ValueChanged {
            id,
            value: ControlValue::FilePath(value)
        }) if id == "control::file" && value == "/tmp/out.png"
    )));
}

#[test]
fn retained_spec_builds_text_area_as_multiline_text_box() {
    let theme = dark_theme();
    let spec = retained_control_text_box_spec(
        &ControlSpec::TextArea {
            value: "notes".to_string(),
            min_rows: 4,
        },
        &theme,
    )
    .expect("text area should map to text box spec");

    assert_eq!(spec.external_text, "notes");
    assert_eq!(spec.mode, TextBoxMode::MultiLine { min_rows: 4 });
    assert_eq!(spec.value_kind, TextBoxValueKind::Text);
    assert_eq!(spec.font, TextBoxFont::Body);
}

#[test]
fn retained_spec_builds_number_as_mono_number_text_box() {
    let theme = dark_theme();
    let spec = retained_control_text_box_spec(
        &ControlSpec::Number {
            value: 1.25,
            min: 0.0,
            max: 2.0,
            step: 0.25,
            precision: 2,
        },
        &theme,
    )
    .expect("number should map to text box spec");

    assert_eq!(spec.external_text, "1.25");
    assert_eq!(spec.mode, TextBoxMode::SingleLine);
    assert_eq!(spec.font, TextBoxFont::Mono);
    assert_eq!(
        spec.value_kind,
        TextBoxValueKind::Number {
            value: 1.25,
            min: 0.0,
            max: 2.0,
            step: 0.25,
            precision: 2,
        }
    );
}

#[test]
fn retained_spec_builds_color_as_mono_text_box() {
    let theme = dark_theme();
    let spec = retained_control_text_box_spec(
        &ControlSpec::Color {
            rgba: [1.0, 0.5, 0.0, 0.75],
        },
        &theme,
    )
    .expect("color should map to text box spec");

    assert_eq!(spec.external_text, "#FF8000");
    assert_eq!(spec.mode, TextBoxMode::SingleLine);
    assert_eq!(spec.font, TextBoxFont::Mono);
    assert_eq!(
        spec.value_kind,
        TextBoxValueKind::Color {
            rgba: [1.0, 0.5, 0.0, 0.75],
        }
    );
}

#[test]
fn retained_spec_builds_file_path_as_text_box() {
    let theme = dark_theme();
    let spec = retained_control_text_box_spec(
        &ControlSpec::FilePath {
            path: "/tmp/out.png".to_string(),
            extensions: vec!["png".to_string()],
        },
        &theme,
    )
    .expect("file path should map to text box spec");

    assert_eq!(spec.external_text, "/tmp/out.png");
    assert_eq!(spec.mode, TextBoxMode::SingleLine);
    assert_eq!(spec.value_kind, TextBoxValueKind::FilePath);
    assert_eq!(spec.font, TextBoxFont::Body);
}
