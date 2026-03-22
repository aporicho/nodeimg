use crate::node::constraint::Constraint;
use crate::node::types::Value;
use eframe::egui;

pub fn render_number_input(
    ui: &mut egui::Ui,
    value: &mut Value,
    _constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let changed = match value {
        Value::Float(ref mut v) => ui
            .add_enabled(!disabled, egui::DragValue::new(v).speed(0.01))
            .changed(),
        Value::Int(ref mut v) => ui
            .add_enabled(!disabled, egui::DragValue::new(v).speed(1.0))
            .changed(),
        _ => false,
    };
    changed
}
