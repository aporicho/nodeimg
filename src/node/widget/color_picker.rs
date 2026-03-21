use crate::node::constraint::Constraint;
use crate::node::types::Value;
use eframe::egui;

pub fn render_color_picker(
    ui: &mut egui::Ui,
    value: &mut Value,
    _constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Value::Color(ref mut c) = value else {
        return false;
    };
    let mut rgba = egui::Rgba::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
    let mut changed = false;
    ui.add_enabled_ui(!disabled, |ui: &mut egui::Ui| {
        if egui::color_picker::color_edit_button_rgba(
            ui,
            &mut rgba,
            egui::color_picker::Alpha::OnlyBlend,
        )
        .changed()
        {
            *c = [rgba.r(), rgba.g(), rgba.b(), rgba.a()];
            changed = true;
        }
    });
    changed
}
