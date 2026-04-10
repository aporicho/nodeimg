//! Dark theme implementation -- modern neutral palette.

use super::tokens;
use super::Theme;
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Shadow, Stroke};

pub struct DarkTheme;

impl Theme for DarkTheme {
    // -- Canvas & Panel --

    fn canvas_bg(&self) -> Color32 {
        Color32::from_rgb(0x2b, 0x2d, 0x30)
    }

    fn panel_bg(&self) -> Color32 {
        Color32::from_rgb(0x1e, 0x1f, 0x22)
    }

    fn panel_separator(&self) -> Color32 {
        Color32::from_rgb(0x4e, 0x51, 0x57)
    }

    // -- Node --

    fn node_body_fill(&self) -> Color32 {
        Color32::from_rgb(0x39, 0x3b, 0x40)
    }

    fn node_body_stroke(&self) -> Stroke {
        Stroke::new(1.0, Color32::from_rgb(0x4e, 0x51, 0x57))
    }

    fn node_shadow(&self) -> Shadow {
        Shadow {
            offset: [0, 2],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(40),
        }
    }

    fn node_header_text(&self) -> Color32 {
        Color32::from_rgb(0xe8, 0xe9, 0xed)
    }

    // -- Categories --

    fn category_color(&self, cat_id: &str) -> Color32 {
        match cat_id {
            "data" => Color32::from_rgb(0x4a, 0x8c, 0xc7),
            "generate" => Color32::from_rgb(0x8c, 0x69, 0xbe),
            "color" => Color32::from_rgb(0xc7, 0x7d, 0x4a),
            "transform" => Color32::from_rgb(0x37, 0x9b, 0x78),
            "filter" => Color32::from_rgb(0xb4, 0x5a, 0x82),
            "composite" => Color32::from_rgb(0xaa, 0x91, 0x2d),
            "tool" => Color32::from_rgb(0x64, 0x6c, 0x7d),
            "ai" => Color32::from_rgb(0xa8, 0x6b, 0xc4),
            _ => Color32::from_rgb(0x5a, 0x5e, 0x66),
        }
    }

    // -- Pins --

    fn pin_color(&self, data_type: &str) -> Color32 {
        match data_type {
            "image" => Color32::from_rgb(0x50, 0xaa, 0xeb),
            "mask" => Color32::from_rgb(0x32, 0xc3, 0x96),
            "float" => Color32::from_rgb(0xf0, 0x8c, 0x41),
            "int" => Color32::from_rgb(0x1e, 0xc3, 0xd2),
            "color" => Color32::from_rgb(0xa0, 0x6e, 0xe1),
            "boolean" => Color32::from_rgb(0xe1, 0x64, 0x6e),
            "string" => Color32::from_rgb(0x8c, 0x96, 0xa8),
            "model" => Color32::from_rgb(0xc0, 0x5e, 0xe0),
            "clip" => Color32::from_rgb(0xf0, 0xb8, 0x3c),
            "vae" => Color32::from_rgb(0xe8, 0x5c, 0x5c),
            "conditioning" => Color32::from_rgb(0xdc, 0x9c, 0xea),
            "latent" => Color32::from_rgb(0xff, 0x7a, 0xb0),
            _ => Color32::GRAY,
        }
    }

    // -- Text --

    fn text_primary(&self) -> Color32 {
        Color32::from_rgb(0xb0, 0xb3, 0xb8)
    }

    fn text_secondary(&self) -> Color32 {
        Color32::from_rgb(0x6a, 0x6d, 0x73)
    }

    // -- Widgets --

    fn widget_bg(&self) -> Color32 {
        Color32::from_rgb(0x32, 0x34, 0x38)
    }

    fn widget_hover_bg(&self) -> Color32 {
        Color32::from_rgb(0x3c, 0x3f, 0x44)
    }

    fn widget_active_bg(&self) -> Color32 {
        Color32::from_rgb(0x4b, 0x6e, 0xc8)
    }

    fn accent(&self) -> Color32 {
        Color32::from_rgb(0x4b, 0x6e, 0xc8)
    }

    fn accent_hover(&self) -> Color32 {
        Color32::from_rgb(0x5d, 0x82, 0xd8)
    }

    // -- Apply --

    fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::dark());

        ctx.style_mut(|style| {
            style.interaction.selectable_labels = false;
            style.spacing.item_spacing = tokens::ITEM_SPACING;
            style.spacing.button_padding = tokens::BUTTON_PADDING;

            // Rounded corners
            let cr = CornerRadius::same(tokens::CORNER_RADIUS);
            style.visuals.widgets.noninteractive.corner_radius = cr;
            style.visuals.widgets.inactive.corner_radius = cr;
            style.visuals.widgets.hovered.corner_radius = cr;
            style.visuals.widgets.active.corner_radius = cr;

            // Widget fills
            style.visuals.widgets.noninteractive.bg_fill = self.widget_bg();
            style.visuals.widgets.inactive.bg_fill = self.widget_bg();
            style.visuals.widgets.hovered.bg_fill = self.widget_hover_bg();
            style.visuals.widgets.active.bg_fill = self.widget_active_bg();

            // Strokes
            style.visuals.widgets.noninteractive.bg_stroke =
                Stroke::new(0.5, Color32::from_rgb(0x4e, 0x51, 0x57));
            style.visuals.widgets.inactive.bg_stroke =
                Stroke::new(0.5, Color32::from_rgb(0x55, 0x58, 0x5e));
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_hover());
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.5, self.accent());

            // Text (fg_stroke)
            style.visuals.widgets.noninteractive.fg_stroke =
                Stroke::new(1.0, self.text_secondary());
            style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

            // Selection
            style.visuals.selection.bg_fill = self.accent().gamma_multiply(0.20);
            style.visuals.selection.stroke = Stroke::new(1.0, self.accent());

            // Panel & window
            style.visuals.panel_fill = self.panel_bg();
            style.visuals.window_fill = self.node_body_fill();
            style.visuals.window_stroke = self.node_body_stroke();
            style.visuals.window_shadow = Shadow {
                offset: [0, 2],
                blur: 12,
                spread: 0,
                color: Color32::from_black_alpha(40),
            };
            style.visuals.window_corner_radius = CornerRadius::same(tokens::NODE_CORNER_RADIUS);

            style.visuals.override_text_color = Some(self.text_primary());
            style.visuals.extreme_bg_color = Color32::from_rgb(0x1a, 0x1b, 0x1e);
            style.visuals.faint_bg_color = Color32::from_rgb(0x2e, 0x30, 0x34);
        });
    }

    // -- Frames --

    fn node_header_frame(&self, cat_id: &str) -> Frame {
        Frame {
            inner_margin: tokens::NODE_HEADER_PADDING,
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius {
                nw: tokens::NODE_CORNER_RADIUS,
                ne: tokens::NODE_CORNER_RADIUS,
                sw: 0,
                se: 0,
            },
            fill: self.category_color(cat_id),
            stroke: Stroke::NONE,
            shadow: Shadow::NONE,
        }
    }

    fn node_body_frame(&self) -> Frame {
        Frame {
            inner_margin: Margin {
                top: 0,
                bottom: 10,
                left: 10,
                right: 10,
            },
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius::same(tokens::NODE_CORNER_RADIUS),
            fill: self.node_body_fill(),
            stroke: self.node_body_stroke(),
            shadow: self.node_shadow(),
        }
    }
}
