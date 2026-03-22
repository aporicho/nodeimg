use nodeimg_types::constraint::Constraint;
use nodeimg_types::value::Value;
use eframe::egui;

pub fn render_text_input(
    ui: &mut egui::Ui,
    value: &mut Value,
    _constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Value::String(ref mut text) = value else {
        return false;
    };
    let mut changed = false;
    ui.add_enabled_ui(!disabled, |ui| {
        let response = ui.add(
            egui::TextEdit::multiline(text)
                .desired_width(ui.available_width())
                .desired_rows(3),
        );
        if response.changed() {
            changed = true;
        }
    });
    changed
}
