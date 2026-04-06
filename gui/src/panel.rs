//! 通用浮动面板组件。
//! 所有面板浮在画布上方，可拖拽。
//! 存储相对于默认位置的偏移量，不存绝对位置。

use iced::{Point, Vector};

#[derive(Debug)]
pub struct FloatingPanel {
    /// 相对于默认布局位置的偏移
    pub offset: Vector,
    /// 是否可见
    pub visible: bool,
    /// 拖拽中：上一帧光标位置
    last_cursor: Option<Point>,
}

#[derive(Debug, Clone)]
pub enum Event {
    DragStart,
    DragMove(Point),
    DragEnd,
}

impl FloatingPanel {
    pub fn new() -> Self {
        Self {
            offset: Vector::ZERO,
            visible: true,
            last_cursor: None,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::DragStart => {
                // 标记开始，等第一次 move 记录起点
                self.last_cursor = None;
            }
            Event::DragMove(cursor) => {
                if let Some(last) = self.last_cursor {
                    // 增量更新偏移
                    self.offset = self.offset + Vector::new(cursor.x - last.x, cursor.y - last.y);
                }
                self.last_cursor = Some(cursor);
            }
            Event::DragEnd => {
                self.last_cursor = None;
            }
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.last_cursor.is_some()
    }
}
