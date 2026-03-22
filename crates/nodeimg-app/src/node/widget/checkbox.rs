use nodeimg_types::constraint::Constraint;
use nodeimg_types::value::Value;
use eframe::egui;

/// Renders a checkbox. Returns true if value changed.
pub fn render_checkbox(
    ui: &mut egui::Ui,
    value: &mut Value,
    _constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Value::Boolean(ref mut v) = value else {
        return false;
    };
    ui.add_enabled(!disabled, egui::Checkbox::new(v, ""))
        .changed()
}
