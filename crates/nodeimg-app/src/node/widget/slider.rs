use crate::node::constraint::Constraint;
use crate::node::types::Value;
use eframe::egui::{self, Sense, Vec2};

const TRACK_HEIGHT: f32 = 2.0;
const HANDLE_RADIUS: f32 = 5.0;
const SLIDER_WIDTH: f32 = 120.0;

/// Renders a custom float slider with thin track and small handle. Returns true if value changed.
pub fn render_slider(
    ui: &mut egui::Ui,
    value: &mut Value,
    constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Constraint::Range { min, max } = constraint else {
        return false;
    };
    let Value::Float(ref mut v) = value else {
        return false;
    };

    let fmin = *min as f32;
    let fmax = *max as f32;
    let mut changed = false;

    ui.add_enabled_ui(!disabled, |ui| {
        ui.horizontal(|ui| {
            // Allocate slider area
            let desired_size = Vec2::new(SLIDER_WIDTH, HANDLE_RADIUS * 2.0 + 4.0);
            let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

            // Handle interaction
            if response.dragged() || response.clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let t = ((pointer_pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                    *v = fmin + t * (fmax - fmin);
                    changed = true;
                }
            }

            // Compute normalized position
            let range = fmax - fmin;
            let t = if range.abs() < f32::EPSILON {
                0.0
            } else {
                ((*v - fmin) / range).clamp(0.0, 1.0)
            };

            // Draw
            let painter = ui.painter();
            let center_y = rect.center().y;

            // Inactive track (full width)
            let track_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left(), center_y - TRACK_HEIGHT * 0.5),
                egui::pos2(rect.right(), center_y + TRACK_HEIGHT * 0.5),
            );
            let inactive_color = ui.visuals().widgets.inactive.bg_fill;
            painter.rect_filled(track_rect, 1.0, inactive_color);

            // Active track (left of handle)
            let handle_x = rect.left() + t * rect.width();
            if handle_x > rect.left() {
                let active_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), center_y - TRACK_HEIGHT * 0.5),
                    egui::pos2(handle_x, center_y + TRACK_HEIGHT * 0.5),
                );
                let accent_color = ui.visuals().selection.stroke.color;
                painter.rect_filled(active_rect, 1.0, accent_color);
            }

            // Handle circle
            let handle_center = egui::pos2(handle_x, center_y);
            let handle_color = if response.hovered() || response.dragged() {
                ui.visuals().widgets.active.fg_stroke.color
            } else {
                ui.visuals().widgets.inactive.fg_stroke.color
            };
            painter.circle_filled(handle_center, HANDLE_RADIUS, handle_color);

            // DragValue for precise input
            let dv = ui.add(
                egui::DragValue::new(v)
                    .range(fmin..=fmax)
                    .speed((fmax - fmin) * 0.005),
            );
            if dv.changed() {
                changed = true;
            }
        });
    });

    changed
}

/// Renders a custom integer slider with thin track and small handle. Returns true if value changed.
pub fn render_int_slider(
    ui: &mut egui::Ui,
    value: &mut Value,
    constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Constraint::Range { min, max } = constraint else {
        return false;
    };
    let Value::Int(ref mut v) = value else {
        return false;
    };

    let imin = *min as i32;
    let imax = *max as i32;
    let mut changed = false;

    ui.add_enabled_ui(!disabled, |ui| {
        ui.horizontal(|ui| {
            // Allocate slider area
            let desired_size = Vec2::new(SLIDER_WIDTH, HANDLE_RADIUS * 2.0 + 4.0);
            let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

            // Handle interaction
            if response.dragged() || response.clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let t = ((pointer_pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                    let float_val = imin as f32 + t * (imax - imin) as f32;
                    *v = float_val.round() as i32;
                    changed = true;
                }
            }

            // Compute normalized position
            let range = (imax - imin) as f32;
            let t = if range.abs() < f32::EPSILON {
                0.0
            } else {
                ((*v - imin) as f32 / range).clamp(0.0, 1.0)
            };

            // Draw
            let painter = ui.painter();
            let center_y = rect.center().y;

            // Inactive track (full width)
            let track_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left(), center_y - TRACK_HEIGHT * 0.5),
                egui::pos2(rect.right(), center_y + TRACK_HEIGHT * 0.5),
            );
            let inactive_color = ui.visuals().widgets.inactive.bg_fill;
            painter.rect_filled(track_rect, 1.0, inactive_color);

            // Active track (left of handle)
            let handle_x = rect.left() + t * rect.width();
            if handle_x > rect.left() {
                let active_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), center_y - TRACK_HEIGHT * 0.5),
                    egui::pos2(handle_x, center_y + TRACK_HEIGHT * 0.5),
                );
                let accent_color = ui.visuals().selection.stroke.color;
                painter.rect_filled(active_rect, 1.0, accent_color);
            }

            // Handle circle
            let handle_center = egui::pos2(handle_x, center_y);
            let handle_color = if response.hovered() || response.dragged() {
                ui.visuals().widgets.active.fg_stroke.color
            } else {
                ui.visuals().widgets.inactive.fg_stroke.color
            };
            painter.circle_filled(handle_center, HANDLE_RADIUS, handle_color);

            // DragValue for precise input
            let dv = ui.add(egui::DragValue::new(v).range(imin..=imax).speed(1.0));
            if dv.changed() {
                changed = true;
            }
        });
    });

    changed
}
