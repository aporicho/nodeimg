//! Light theme implementation -- refined light editorial palette.

use super::tokens;
use super::Theme;
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Shadow, Stroke};

pub struct LightTheme;

impl Theme for LightTheme {
    // -- Canvas & Panel --

    fn canvas_bg(&self) -> Color32 {
        Color32::from_rgb(0xf0, 0xf1, 0xf5)
    }

    fn panel_bg(&self) -> Color32 {
        Color32::from_rgb(0xfa, 0xfb, 0xfd)
    }

    fn panel_separator(&self) -> Color32 {
        Color32::from_rgb(0xd8, 0xda, 0xe2)
    }

    // -- Node --

    fn node_body_fill(&self) -> Color32 {
        Color32::WHITE
    }

    fn node_body_stroke(&self) -> Stroke {
        Stroke::new(1.0, Color32::from_rgb(0xd8, 0xda, 0xe2))
    }

    fn node_shadow(&self) -> Shadow {
        Shadow {
            offset: [0, 2],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(18),
        }
    }

    fn node_header_text(&self) -> Color32 {
        Color32::WHITE
    }

    // -- Categories --

    fn category_color(&self, cat_id: &str) -> Color32 {
        match cat_id {
            "data" => Color32::from_rgb(0x42, 0x85, 0xc8),
            "generate" => Color32::from_rgb(0x7d, 0x55, 0xb9),
            "color" => Color32::from_rgb(0xcd, 0x73, 0x32),
            "transform" => Color32::from_rgb(0x28, 0xa0, 0x78),
            "filter" => Color32::from_rgb(0xbe, 0x50, 0x87),
            "composite" => Color32::from_rgb(0xb4, 0x96, 0x23),
            "tool" => Color32::from_rgb(0x6e, 0x76, 0x8a),
            "ai" => Color32::from_rgb(0x9b, 0x59, 0xb6),
            _ => Color32::from_rgb(0xa0, 0xa5, 0xaf),
        }
    }

    // -- Pins --

    fn pin_color(&self, data_type: &str) -> Color32 {
        match data_type {
            "image" => Color32::from_rgb(0x38, 0x99, 0xe0),
            "mask" => Color32::from_rgb(0x21, 0xb3, 0x82),
            "float" => Color32::from_rgb(0xe8, 0x78, 0x2e),
            "int" => Color32::from_rgb(0x00, 0xb3, 0xc7),
            "color" => Color32::from_rgb(0x8c, 0x5c, 0xd1),
            "boolean" => Color32::from_rgb(0xd6, 0x57, 0x61),
            "string" => Color32::from_rgb(0x78, 0x82, 0x96),
            "model" => Color32::from_rgb(0xb5, 0x4a, 0xd6),
            "clip" => Color32::from_rgb(0xe8, 0xa8, 0x2e),
            "vae" => Color32::from_rgb(0xe0, 0x4a, 0x4a),
            "conditioning" => Color32::from_rgb(0xd1, 0x8c, 0xe0),
            "latent" => Color32::from_rgb(0xff, 0x66, 0xa3),
            _ => Color32::GRAY,
        }
    }

    // -- Text --

    fn text_primary(&self) -> Color32 {
        Color32::from_rgb(0x1a, 0x1c, 0x24)
    }

    fn text_secondary(&self) -> Color32 {
        Color32::from_rgb(0x6b, 0x70, 0x80)
    }

    // -- Widgets --

    fn widget_bg(&self) -> Color32 {
        Color32::from_rgb(0xeb, 0xed, 0xf2)
    }

    fn widget_hover_bg(&self) -> Color32 {
        Color32::from_rgb(0xde, 0xe2, 0xec)
    }

    fn widget_active_bg(&self) -> Color32 {
        Color32::from_rgb(0x42, 0x63, 0xeb)
    }

    fn accent(&self) -> Color32 {
        Color32::from_rgb(0x42, 0x63, 0xeb)
    }

    fn accent_hover(&self) -> Color32 {
        Color32::from_rgb(0x55, 0x78, 0xf5)
    }

    // -- Apply --

    fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::light());

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
                Stroke::new(0.5, Color32::from_rgb(0xd2, 0xd4, 0xdc));
            style.visuals.widgets.inactive.bg_stroke =
                Stroke::new(0.5, Color32::from_rgb(0xc8, 0xcc, 0xd7));
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_hover());
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.5, self.accent());

            // Text (fg_stroke)
            style.visuals.widgets.noninteractive.fg_stroke =
                Stroke::new(1.0, self.text_secondary());
            style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

            // Selection
            style.visuals.selection.bg_fill = self.accent().gamma_multiply(0.15);
            style.visuals.selection.stroke = Stroke::new(1.0, self.accent());

            // Panel & window
            style.visuals.panel_fill = self.panel_bg();
            style.visuals.window_fill = Color32::WHITE;
            style.visuals.window_stroke = Stroke::new(1.0, self.node_body_stroke().color);
            style.visuals.window_shadow = Shadow {
                offset: [0, 2],
                blur: 10,
                spread: 0,
                color: Color32::from_black_alpha(18),
            };
            style.visuals.window_corner_radius = CornerRadius::same(tokens::NODE_CORNER_RADIUS);

            style.visuals.override_text_color = Some(self.text_primary());
            style.visuals.extreme_bg_color = Color32::WHITE;
            style.visuals.faint_bg_color = Color32::from_rgb(0xf8, 0xf9, 0xfc);
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
