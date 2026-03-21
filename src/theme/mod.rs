pub mod dark;
pub mod light;
pub mod tokens;

use eframe::egui::{self, Color32, Frame, Shadow, Stroke};

/// Defines all visual properties for a theme. Each theme implements this independently.
pub trait Theme: Send + Sync {
    fn canvas_bg(&self) -> Color32;
    fn panel_bg(&self) -> Color32;
    fn panel_separator(&self) -> Color32;

    fn node_body_fill(&self) -> Color32;
    fn node_body_stroke(&self) -> Stroke;
    fn node_shadow(&self) -> Shadow;
    fn node_header_text(&self) -> Color32;

    fn category_color(&self, cat_id: &str) -> Color32;
    fn pin_color(&self, data_type: &str) -> Color32;

    fn text_primary(&self) -> Color32;
    fn text_secondary(&self) -> Color32;

    fn widget_bg(&self) -> Color32;
    fn widget_hover_bg(&self) -> Color32;
    fn widget_active_bg(&self) -> Color32;
    fn accent(&self) -> Color32;
    fn accent_hover(&self) -> Color32;

    fn apply(&self, ctx: &egui::Context);

    fn node_header_frame(&self, cat_id: &str) -> Frame;
    fn node_body_frame(&self) -> Frame;
}
