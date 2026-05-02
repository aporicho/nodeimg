use super::number::parse_number_text;
use super::retained_spec::retained_param_text_box_spec;
use crate::control::state::TextBoxValueKind;
use crate::control::{ParamControlSpec, TextBoxFont, TextBoxMode};
use crate::theme::dark_theme;

#[test]
fn parse_number_text_rejects_incomplete_numbers() {
    assert_eq!(parse_number_text(""), None);
    assert_eq!(parse_number_text("-"), None);
    assert_eq!(parse_number_text("+."), None);
    assert_eq!(parse_number_text(" 42.5 "), Some(42.5));
}

#[test]
fn retained_spec_maps_text_area_to_multiline_text_box() {
    let theme = dark_theme();
    let spec = retained_param_text_box_spec(
        &ParamControlSpec::TextArea {
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
fn retained_spec_maps_number_to_mono_number_text_box() {
    let theme = dark_theme();
    let spec = retained_param_text_box_spec(
        &ParamControlSpec::Number {
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
