/// 手势识别后产出的动作。
#[derive(Debug, Clone)]
pub enum Action {
    Click(String),
    DoubleClick(String),
    DragStart { id: String, x: f32, y: f32 },
    DragMove { id: String, x: f32, y: f32 },
    DragEnd { id: String, x: f32, y: f32 },
    LongPress(String),
}
