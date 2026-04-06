use iced::widget::canvas::{Frame, Path, Stroke, Text};
use iced::{Color, Point, Size};

use crate::render::{BORDER, TEXT_PRIMARY, TEXT_SECONDARY};

pub const SLIDER_HEIGHT: f32 = 28.0;
pub const TRACK_HEIGHT: f32 = 4.0;
pub const THUMB_RADIUS: f32 = 7.0;

/// 单个 Slider 实例的布局信息。
///
/// 坐标已换算为屏幕像素（乘过 zoom）。
pub struct SliderLayout {
    /// 轨道区域左上角 X（屏幕像素）
    pub x: f32,
    /// 轨道区域左上角 Y（屏幕像素）
    pub y: f32,
    /// 轨道宽度（屏幕像素）
    pub width: f32,
    /// 当前缩放比例（用于字体大小等比例缩放）
    pub zoom: f32,
}

impl SliderLayout {
    /// 绘制 Slider（轨道 + 填充 + 拇指 + 标签 + 数值）。
    ///
    /// - `label` — 参数名（左侧显示）
    /// - `value` — 当前值
    /// - `min / max` — 取值范围
    pub fn draw(
        &self,
        frame: &mut Frame,
        label: &str,
        value: f32,
        min: f32,
        max: f32,
    ) {
        let zoom = self.zoom;
        let x = self.x;
        let y = self.y;
        let w = self.width;

        // --- 轨道垂直居中位置 ---
        let track_top = y + (SLIDER_HEIGHT - TRACK_HEIGHT) / 2.0 * zoom;
        let track_h = TRACK_HEIGHT * zoom;

        // --- 背景轨道 ---
        let bg_path = Path::rectangle(
            Point::new(x, track_top),
            Size::new(w, track_h),
        );
        frame.fill(&bg_path, BORDER);

        // --- 填充比例 ---
        let ratio = if (max - min).abs() < f32::EPSILON {
            0.0
        } else {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        };
        let fill_w = w * ratio;

        if fill_w > 0.0 {
            let fill_path = Path::rectangle(
                Point::new(x, track_top),
                Size::new(fill_w, track_h),
            );
            frame.fill(&fill_path, TEXT_PRIMARY);
        }

        // --- 拇指圆 ---
        let thumb_cx = x + fill_w;
        let thumb_cy = track_top + track_h / 2.0;
        let thumb_r = THUMB_RADIUS * zoom;

        let thumb_path = Path::circle(Point::new(thumb_cx, thumb_cy), thumb_r);
        frame.fill(&thumb_path, Color::WHITE);

        let thumb_stroke = Stroke::default()
            .with_color(TEXT_PRIMARY)
            .with_width(1.5 * zoom);
        frame.stroke(&thumb_path, thumb_stroke);

        // --- 字体尺寸 ---
        let font_size = 11.0 * zoom;
        let text_y = y + (SLIDER_HEIGHT / 2.0 - font_size / 2.0) * zoom;

        // --- 左侧标签 ---
        frame.fill_text(Text {
            content: label.to_string(),
            position: Point::new(x, text_y),
            size: iced::Pixels(font_size),
            color: TEXT_SECONDARY,
            ..Text::default()
        });

        // --- 右侧数值（截断为 2 位小数） ---
        let value_str = format!("{:.2}", value);
        // 每个字符约 0.6 倍字体宽度（粗略估算），右对齐
        let value_w = value_str.len() as f32 * font_size * 0.6;
        let value_x = x + w - value_w;
        frame.fill_text(Text {
            content: value_str,
            position: Point::new(value_x, text_y),
            size: iced::Pixels(font_size),
            color: TEXT_PRIMARY,
            ..Text::default()
        });
    }

    /// 命中测试：判断屏幕坐标 `(cx, cy)` 是否落在此 Slider 区域内。
    pub fn hit_test(&self, cx: f32, cy: f32) -> bool {
        let zoom = self.zoom;
        cx >= self.x
            && cx <= self.x + self.width
            && cy >= self.y
            && cy <= self.y + SLIDER_HEIGHT * zoom
    }

    /// 根据屏幕 X 坐标换算出参数值（夹在 [min, max] 范围内）。
    pub fn value_from_x(&self, cx: f32, min: f32, max: f32) -> f32 {
        let ratio = ((cx - self.x) / self.width).clamp(0.0, 1.0);
        min + ratio * (max - min)
    }
}
