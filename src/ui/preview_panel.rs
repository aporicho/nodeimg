use crate::theme::Theme;
use eframe::egui;

pub struct PreviewPanel {
    pub show: bool,
}

impl Default for PreviewPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewPanel {
    pub fn new() -> Self {
        Self { show: true }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &dyn Theme,
        preview_texture: Option<&egui::TextureHandle>,
    ) {
        if !self.show {
            return;
        }

        let panel_width = ctx.content_rect().width() / 2.0;
        egui::SidePanel::left("preview_panel")
            .default_width(panel_width)
            .min_width(200.0)
            .resizable(true)
            .frame(egui::Frame {
                inner_margin: egui::Margin::same(0),
                outer_margin: egui::Margin::ZERO,
                corner_radius: egui::CornerRadius::ZERO,
                fill: theme.panel_bg(),
                stroke: egui::Stroke::NONE,
                shadow: egui::Shadow::NONE,
            })
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.add(egui::Button::new("\u{2715}").frame(false)).clicked() {
                        self.show = false;
                    }
                });

                if let Some(tex) = preview_texture {
                    let size = tex.size_vec2();
                    let available = ui.available_size();
                    let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
                    ui.centered_and_justified(|ui| {
                        ui.image(egui::load::SizedTexture::new(tex.id(), size * scale));
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.colored_label(theme.text_secondary(), "No preview");
                    });
                }
            });
    }
}
