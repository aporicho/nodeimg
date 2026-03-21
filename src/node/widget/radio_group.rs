use crate::node::constraint::Constraint;
use crate::node::types::Value;
use eframe::egui;

pub fn render_radio_group(
    ui: &mut egui::Ui,
    value: &mut Value,
    constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Constraint::Enum { options } = constraint else {
        return false;
    };
    let Value::String(ref mut selected) = value else {
        return false;
    };
    let mut changed = false;
    ui.add_enabled_ui(!disabled, |ui: &mut egui::Ui| {
        for (label, val) in options {
            if ui.radio_value(selected, val.clone(), label).changed() {
                changed = true;
            }
        }
    });
    changed
}
