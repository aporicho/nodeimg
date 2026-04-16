use crate::widget::resize_edge::ResizeEdge;

/// 手势识别后产出的内部信号。
#[derive(Debug, Clone)]
pub(crate) enum GestureSignal {
    Click(String),
    DoubleClick(String),
    DragStart {
        id: String,
        x: f32,
        y: f32,
    },
    DragMove {
        id: String,
        x: f32,
        y: f32,
    },
    DragEnd {
        id: String,
        x: f32,
        y: f32,
    },
    LongPress(String),
    ResizeStart {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
    ResizeMove {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
    ResizeEnd {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
}
