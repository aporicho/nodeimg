use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::widget::layout::{BoxStyle, LayoutBox, Size};
use crate::widget::r#trait::Widget;
use crate::renderer::{Border, Color, Point, Rect, RectStyle, Renderer, TextStyle};

// shadcn zinc 视觉常量
const FONT_SIZE: f32 = 12.0;
const PADDING_V: f32 = 8.0;
const RADIUS: f32 = 4.0;
const BORDER_COLOR: Color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 }; // zinc-200
const TEXT_COLOR: Color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };   // zinc-900

pub struct Button {
    pub id: &'static str,
    pub label: &'static str,
    pub color: Color,
}

impl Widget for Button {
    fn id(&self) -> &'static str {
        self.id
    }

    fn layout(&self, renderer: &mut Renderer) -> LayoutBox {
        let (_text_w, text_h) = renderer.measure_text(self.label, FONT_SIZE);
        LayoutBox {
            id: Some(self.id),
            style: BoxStyle {
                height: Size::Fixed(text_h + PADDING_V * 2.0),
                ..BoxStyle::default()
            },
            children: vec![],
        }
    }

    fn paint(&self, rect: Rect, renderer: &mut Renderer) {
        renderer.draw_rect(rect, &RectStyle {
            color: self.color,
            border: Some(Border { width: 1.0, color: BORDER_COLOR }),
            radius: [RADIUS; 4],
            shadow: None,
        });

        let (text_w, text_h) = renderer.measure_text(self.label, FONT_SIZE);
        let text_x = rect.x + (rect.w - text_w) / 2.0;
        let text_y = rect.y + (rect.h - text_h) / 2.0;
        renderer.draw_text(
            Point { x: text_x, y: text_y },
            self.label,
            &TextStyle { color: TEXT_COLOR, size: FONT_SIZE },
        );
    }

    fn props_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.label.hash(&mut hasher);
        self.color.r.to_bits().hash(&mut hasher);
        self.color.g.to_bits().hash(&mut hasher);
        self.color.b.to_bits().hash(&mut hasher);
        self.color.a.to_bits().hash(&mut hasher);
        hasher.finish()
    }
}
