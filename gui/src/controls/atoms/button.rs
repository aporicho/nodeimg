use crate::controls::infra::layout::{BoxStyle, LayoutBox, Size};
use crate::renderer::{Border, Color, Point, Rect, RectStyle, Renderer, TextStyle};

// shadcn zinc 视觉常量
const FONT_SIZE: f32 = 12.0;
const PADDING_V: f32 = 8.0;
#[allow(dead_code)]
const PADDING_H: f32 = 16.0;
const RADIUS: f32 = 4.0;
const BORDER_COLOR: Color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 }; // zinc-200 #e4e4e7
const TEXT_COLOR: Color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };   // zinc-900 #18181b

pub struct ButtonProps {
    pub id: &'static str,
    pub label: &'static str,
    pub color: Color,
}

/// 计算 Button 的 LayoutBox（需要 renderer 做文字度量）
pub fn button_layout(props: &ButtonProps, renderer: &mut Renderer) -> LayoutBox {
    let (_text_w, text_h) = renderer.measure_text(props.label, FONT_SIZE);
    LayoutBox {
        id: Some(props.id),
        style: BoxStyle {
            height: Size::Fixed(text_h + PADDING_V * 2.0),
            ..BoxStyle::default()
        },
        children: vec![],
    }
}

/// 渲染 Button
pub fn button_paint(props: &ButtonProps, rect: Rect, renderer: &mut Renderer) {
    renderer.draw_rect(rect, &RectStyle {
        color: props.color,
        border: Some(Border { width: 1.0, color: BORDER_COLOR }),
        radius: [RADIUS; 4],
        shadow: None,
    });

    let (text_w, text_h) = renderer.measure_text(props.label, FONT_SIZE);
    let text_x = rect.x + (rect.w - text_w) / 2.0;
    let text_y = rect.y + (rect.h - text_h) / 2.0;
    renderer.draw_text(
        Point { x: text_x, y: text_y },
        props.label,
        &TextStyle { color: TEXT_COLOR, size: FONT_SIZE },
    );
}
