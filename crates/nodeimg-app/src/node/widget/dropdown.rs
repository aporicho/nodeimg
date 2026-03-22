use nodeimg_types::constraint::Constraint;
use nodeimg_types::value::Value;
use eframe::egui;

pub fn render_dropdown(
    ui: &mut egui::Ui,
    value: &mut Value,
    constraint: &Constraint,
    param_name: &str,
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
        egui::ComboBox::from_id_salt(format!("{}-{}", ui.id().value(), param_name))
            .selected_text(
                options
                    .iter()
                    .find(|(_, v)| v == selected)
                    .map(|(l, _)| l.as_str())
                    .unwrap_or(selected.as_str()),
            )
            .show_ui(ui, |ui: &mut egui::Ui| {
                for (label, val) in options {
                    if ui.selectable_label(selected == val, label).clicked() {
                        *selected = val.clone();
                        changed = true;
                    }
                }
            });
    });
    changed
}
