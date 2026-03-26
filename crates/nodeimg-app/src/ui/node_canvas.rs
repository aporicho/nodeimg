use eframe::egui;
use egui::Id;
use egui_snarl::ui::{NodeLayout, SnarlStyle, SnarlWidget};
use egui_snarl::Snarl;

use crate::node::viewer::NodeViewer;
use nodeimg_types::node_instance::NodeInstance;
use crate::theme::Theme;
use crate::ui::preview_panel::PreviewPanel;

pub struct NodeCanvas {
    pub style: SnarlStyle,
}

impl Default for NodeCanvas {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeCanvas {
    pub fn new() -> Self {
        let style = SnarlStyle {
            node_layout: Some(NodeLayout::sandwich()),
            collapsible: Some(false),
            ..SnarlStyle::new()
        };
        Self { style }
    }

    pub fn show(
        &self,
        ctx: &egui::Context,
        theme: &dyn Theme,
        snarl: &mut Snarl<NodeInstance>,
        viewer: &mut NodeViewer,
        preview_panel: &mut PreviewPanel,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.canvas_bg()))
            .show(ctx, |ui| {
                if !preview_panel.show {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        if ui
                            .add(egui::Button::new("\u{25a1}").frame(false))
                            .on_hover_text("Open Preview")
                            .clicked()
                        {
                            preview_panel.show = true;
                        }
                    });
                }
                SnarlWidget::new()
                    .id(Id::new("nodeimg"))
                    .style(self.style)
                    .show(snarl, viewer, ui);
            });
    }
}
