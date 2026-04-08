use crate::shell::CursorStyle;

use super::frame::PanelFrame;
use super::layer::PanelLayer;

const EDGE_THRESHOLD: f32 = 6.0;

#[derive(Debug, Clone, Copy)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    pub fn cursor_style(self) -> CursorStyle {
        match self {
            Self::Top => CursorStyle::ResizeN,
            Self::Bottom => CursorStyle::ResizeS,
            Self::Left => CursorStyle::ResizeW,
            Self::Right => CursorStyle::ResizeE,
            Self::TopLeft => CursorStyle::ResizeNW,
            Self::TopRight => CursorStyle::ResizeNE,
            Self::BottomLeft => CursorStyle::ResizeSW,
            Self::BottomRight => CursorStyle::ResizeSE,
        }
    }
}

/// Resize 状态。
pub struct ResizeState {
    active_panel: Option<&'static str>,
    edge: ResizeEdge,
    last_x: f32,
    last_y: f32,
}

impl ResizeState {
    pub fn new() -> Self {
        Self {
            active_panel: None,
            edge: ResizeEdge::Right,
            last_x: 0.0,
            last_y: 0.0,
        }
    }

    /// 检测鼠标在面板的哪个边缘。返回 None 表示不在边缘。
    pub fn detect_edge(frame: &PanelFrame, x: f32, y: f32) -> Option<ResizeEdge> {
        let r = frame.rect();
        let near_left = (x - r.x).abs() < EDGE_THRESHOLD;
        let near_right = (x - (r.x + r.w)).abs() < EDGE_THRESHOLD;
        let near_top = (y - r.y).abs() < EDGE_THRESHOLD;
        let near_bottom = (y - (r.y + r.h)).abs() < EDGE_THRESHOLD;

        match (near_left, near_right, near_top, near_bottom) {
            (true, _, true, _) => Some(ResizeEdge::TopLeft),
            (true, _, _, true) => Some(ResizeEdge::BottomLeft),
            (_, true, true, _) => Some(ResizeEdge::TopRight),
            (_, true, _, true) => Some(ResizeEdge::BottomRight),
            (true, _, _, _) => Some(ResizeEdge::Left),
            (_, true, _, _) => Some(ResizeEdge::Right),
            (_, _, true, _) => Some(ResizeEdge::Top),
            (_, _, _, true) => Some(ResizeEdge::Bottom),
            _ => None,
        }
    }

    pub fn start(&mut self, panel_id: &'static str, edge: ResizeEdge, x: f32, y: f32) {
        self.active_panel = Some(panel_id);
        self.edge = edge;
        self.last_x = x;
        self.last_y = y;
    }

    pub fn update(&mut self, x: f32, y: f32, layer: &mut PanelLayer) {
        let Some(id) = self.active_panel else { return };
        let dx = x - self.last_x;
        let dy = y - self.last_y;

        if let Some(frame) = layer.get_mut(id) {
            match self.edge {
                ResizeEdge::Right => {
                    frame.w = (frame.w + dx).max(frame.min_w);
                }
                ResizeEdge::Bottom => {
                    frame.h = (frame.h + dy).max(frame.min_h);
                }
                ResizeEdge::Left => {
                    let new_w = (frame.w - dx).max(frame.min_w);
                    let actual_dx = frame.w - new_w;
                    frame.x += actual_dx;
                    frame.w = new_w;
                }
                ResizeEdge::Top => {
                    let new_h = (frame.h - dy).max(frame.min_h);
                    let actual_dy = frame.h - new_h;
                    frame.y += actual_dy;
                    frame.h = new_h;
                }
                ResizeEdge::TopLeft => {
                    let new_w = (frame.w - dx).max(frame.min_w);
                    let new_h = (frame.h - dy).max(frame.min_h);
                    frame.x += frame.w - new_w;
                    frame.y += frame.h - new_h;
                    frame.w = new_w;
                    frame.h = new_h;
                }
                ResizeEdge::TopRight => {
                    frame.w = (frame.w + dx).max(frame.min_w);
                    let new_h = (frame.h - dy).max(frame.min_h);
                    frame.y += frame.h - new_h;
                    frame.h = new_h;
                }
                ResizeEdge::BottomLeft => {
                    let new_w = (frame.w - dx).max(frame.min_w);
                    frame.x += frame.w - new_w;
                    frame.w = new_w;
                    frame.h = (frame.h + dy).max(frame.min_h);
                }
                ResizeEdge::BottomRight => {
                    frame.w = (frame.w + dx).max(frame.min_w);
                    frame.h = (frame.h + dy).max(frame.min_h);
                }
            }
        }

        self.last_x = x;
        self.last_y = y;
    }

    pub fn end(&mut self) {
        self.active_panel = None;
    }

    pub fn is_active(&self) -> bool {
        self.active_panel.is_some()
    }

    pub fn current_edge(&self) -> Option<ResizeEdge> {
        if self.active_panel.is_some() { Some(self.edge) } else { None }
    }
}
