use crate::node::constraint::Constraint;
use crate::node::types::Value;
use eframe::egui;

pub fn render_file_picker(
    ui: &mut egui::Ui,
    value: &mut Value,
    constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Constraint::FilePath { filters } = constraint else {
        return false;
    };
    let Value::String(ref mut path) = value else {
        return false;
    };
    let mut changed = false;
    ui.add_enabled_ui(!disabled, |ui: &mut egui::Ui| {
        ui.horizontal(|ui: &mut egui::Ui| {
            let display = if path.is_empty() {
                "Select file..."
            } else {
                std::path::Path::new(path.as_str())
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path.as_str())
            };
            let label = ui.label(display);
            if !path.is_empty() {
                label.on_hover_text(path.as_str());
            }
            if ui.button("Browse").clicked() {
                let mut dialog = rfd::FileDialog::new();
                for ext in filters {
                    dialog = dialog.add_filter(ext, &[ext.as_str()]);
                }
                if let Some(p) = dialog.pick_file() {
                    *path = p.to_string_lossy().to_string();
                    changed = true;
                }
            }
        });
    });
    changed
}
