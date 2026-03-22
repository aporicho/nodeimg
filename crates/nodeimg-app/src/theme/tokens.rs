//! Design tokens shared by all themes. Layout constants only -- no colors.

use eframe::egui::{vec2, Margin, Vec2};

// Spacing
pub const ITEM_SPACING: Vec2 = vec2(6.0, 4.0);
pub const BUTTON_PADDING: Vec2 = vec2(8.0, 4.0);
pub const NODE_HEADER_PADDING: Margin = Margin::symmetric(12, 6);
pub const NODE_BODY_PADDING: Margin = Margin::same(10);
pub const PARAM_LABEL_WIDTH: f32 = 80.0;

// Corner radius (u8 because CornerRadius::same() takes u8)
pub const CORNER_RADIUS: u8 = 6;
pub const NODE_CORNER_RADIUS: u8 = 10;

// Font sizes
pub const FONT_SIZE_SMALL: f32 = 11.0;
pub const FONT_SIZE_NORMAL: f32 = 13.0;

// Node constraints
pub const NODE_MAX_WIDTH: f32 = 280.0;
pub const NODE_PREVIEW_MAX_SIZE: f32 = 200.0;
