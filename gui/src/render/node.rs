use engine::registry::def::NodeDef;
use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Size};

use super::{
    BORDER, BORDER_SELECTED, NODE_BG, TEXT_MUTED, TEXT_SECONDARY,
    category_color, pin_color,
};

// --- 布局常量 ---
pub const NODE_WIDTH: f32 = 192.0;
pub const NODE_RADIUS: f32 = 8.0;
pub const HEADER_HEIGHT: f32 = 32.0;
pub const PIN_RADIUS: f32 = 5.0;
pub const PADDING: f32 = 10.0;
pub const PIN_ROW_HEIGHT: f32 = 20.0;
pub const PARAM_HEIGHT: f32 = 28.0;

/// 计算节点的屏幕矩形（左上角坐标 + 尺寸）。
pub fn node_bounds(
    canvas_x: f32,
    canvas_y: f32,
    def: &NodeDef,
    offset_x: f32,
    offset_y: f32,
    zoom: f32,
) -> Rectangle {
    let pin_rows = def.inputs.len().max(def.outputs.len());
    let param_rows = def.params.len();
    let body_height = HEADER_HEIGHT
        + pin_rows as f32 * PIN_ROW_HEIGHT
        + param_rows as f32 * PARAM_HEIGHT
        + PADDING * 2.0;

    let screen_x = (canvas_x + offset_x) * zoom;
    let screen_y = (canvas_y + offset_y) * zoom;

    Rectangle {
        x: screen_x,
        y: screen_y,
        width: NODE_WIDTH * zoom,
        height: body_height * zoom,
    }
}

/// 计算输入引脚的屏幕坐标（圆心）。
pub fn input_pin_position(
    canvas_x: f32,
    canvas_y: f32,
    pin_index: usize,
    offset_x: f32,
    offset_y: f32,
    zoom: f32,
) -> Point {
    let screen_x = (canvas_x + offset_x) * zoom;
    let screen_y = (canvas_y + offset_y) * zoom;

    Point::new(
        screen_x,
        screen_y + (HEADER_HEIGHT + PADDING + pin_index as f32 * PIN_ROW_HEIGHT + PIN_ROW_HEIGHT / 2.0) * zoom,
    )
}

/// 计算输出引脚的屏幕坐标（圆心）。
/// `_num_inputs` 保留供 Task 4 中布局对齐使用。
pub fn output_pin_position(
    canvas_x: f32,
    canvas_y: f32,
    pin_index: usize,
    _num_inputs: usize,
    offset_x: f32,
    offset_y: f32,
    zoom: f32,
) -> Point {
    let screen_x = (canvas_x + offset_x) * zoom;
    let screen_y = (canvas_y + offset_y) * zoom;

    Point::new(
        screen_x + NODE_WIDTH * zoom,
        screen_y + (HEADER_HEIGHT + PADDING + pin_index as f32 * PIN_ROW_HEIGHT + PIN_ROW_HEIGHT / 2.0) * zoom,
    )
}

/// 绘制单个节点。
///
/// - `canvas_x / canvas_y` — 节点在画布坐标系中的位置
/// - `offset_x / offset_y / zoom` — 视口参数（与 Viewport 解耦，Task 4 中集成）
/// - `selected` — 是否选中（影响边框颜色）
pub fn draw_node(
    frame: &mut Frame,
    canvas_x: f32,
    canvas_y: f32,
    def: &NodeDef,
    selected: bool,
    offset_x: f32,
    offset_y: f32,
    zoom: f32,
) {
    let pin_rows = def.inputs.len().max(def.outputs.len());
    let param_rows = def.params.len();
    let body_height = HEADER_HEIGHT
        + pin_rows as f32 * PIN_ROW_HEIGHT
        + param_rows as f32 * PARAM_HEIGHT
        + PADDING * 2.0;

    let sx = (canvas_x + offset_x) * zoom;
    let sy = (canvas_y + offset_y) * zoom;
    let w = NODE_WIDTH * zoom;
    let h = body_height * zoom;

    // --- 背景圆角矩形 ---
    let bg_path = Path::rectangle(Point::new(sx, sy), Size::new(w, h));
    frame.fill(&bg_path, NODE_BG);

    // 边框
    let border_color = if selected { BORDER_SELECTED } else { BORDER };
    let border_stroke = Stroke::default().with_color(border_color).with_width(1.0 * zoom);
    frame.stroke(&bg_path, border_stroke);

    // --- 头部色条 ---
    let cat_color = category_color(&def.category);
    let header_path = Path::rectangle(
        Point::new(sx, sy),
        Size::new(w, HEADER_HEIGHT * zoom),
    );
    frame.fill(&header_path, cat_color);

    // 节点名称
    let name_size = 13.0 * zoom;
    frame.fill_text(Text {
        content: def.name.clone(),
        position: Point::new(sx + PADDING * zoom, sy + (HEADER_HEIGHT / 2.0 - name_size / 2.0) * zoom),
        size: iced::Pixels(name_size),
        color: Color::WHITE,
        ..Text::default()
    });

    // --- 输入引脚 ---
    for (i, pin) in def.inputs.iter().enumerate() {
        let py = sy + (HEADER_HEIGHT + PADDING + i as f32 * PIN_ROW_HEIGHT + PIN_ROW_HEIGHT / 2.0) * zoom;
        let px = sx;
        let color = pin_color(&pin.data_type.0);

        // 引脚圆点
        let circle = Path::circle(Point::new(px, py), PIN_RADIUS * zoom);
        frame.fill(&circle, color);

        // 引脚标签
        let label_size = 11.0 * zoom;
        frame.fill_text(Text {
            content: pin.name.clone(),
            position: Point::new(px + (PIN_RADIUS + 5.0) * zoom, py - label_size / 2.0),
            size: iced::Pixels(label_size),
            color: TEXT_SECONDARY,
            ..Text::default()
        });
    }

    // --- 输出引脚 ---
    for (i, pin) in def.outputs.iter().enumerate() {
        let py = sy + (HEADER_HEIGHT + PADDING + i as f32 * PIN_ROW_HEIGHT + PIN_ROW_HEIGHT / 2.0) * zoom;
        let px = sx + w;
        let color = pin_color(&pin.data_type.0);

        // 引脚圆点
        let circle = Path::circle(Point::new(px, py), PIN_RADIUS * zoom);
        frame.fill(&circle, color);

        // 引脚标签（右侧，文字靠右对齐）
        let label_size = 11.0 * zoom;
        frame.fill_text(Text {
            content: pin.name.clone(),
            position: Point::new(px - (PIN_RADIUS + 5.0) * zoom, py - label_size / 2.0),
            size: iced::Pixels(label_size),
            color: TEXT_SECONDARY,
            ..Text::default()
        });
    }

    // --- 参数区域（占位，Task 9 实现 slider 渲染）---
    let params_start_y = sy + (HEADER_HEIGHT + PADDING + pin_rows as f32 * PIN_ROW_HEIGHT) * zoom;
    for (i, param) in def.params.iter().enumerate() {
        let row_y = params_start_y + (i as f32 * PARAM_HEIGHT) * zoom;
        let label_size = 11.0 * zoom;

        // 参数名称
        frame.fill_text(Text {
            content: param.name.clone(),
            position: Point::new(sx + PADDING * zoom, row_y + (PARAM_HEIGHT / 2.0 - label_size / 2.0) * zoom),
            size: iced::Pixels(label_size),
            color: TEXT_MUTED,
            ..Text::default()
        });

        // 参数占位背景条
        let slot_path = Path::rectangle(
            Point::new(sx + PADDING * zoom, row_y + PADDING * zoom * 0.5),
            Size::new(w - PADDING * 2.0 * zoom, (PARAM_HEIGHT - PADDING) * zoom),
        );
        frame.stroke(&slot_path, Stroke::default().with_color(BORDER).with_width(1.0));
    }
}
