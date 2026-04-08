/// 鼠标按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// 键盘按键（后续按需扩展）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Escape,
    Tab,
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Space,
    /// 字母 A-Z（大写存储，不区分大小写）
    Char(char),
    /// 其他未映射的按键
    Other,
}

/// 修饰键状态。
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on macOS, Win on Windows
}

/// 应用层事件。由 EventTranslator 从 winit 原始事件翻译而来。
pub enum AppEvent {
    // ── 鼠标 ──
    MouseMove { x: f32, y: f32 },
    MousePress { x: f32, y: f32, button: MouseButton },
    MouseRelease { x: f32, y: f32, button: MouseButton },
    Scroll { x: f32, y: f32, delta_x: f32, delta_y: f32 },

    // ── 键盘 ──
    KeyPress { key: Key, modifiers: Modifiers },
    KeyRelease { key: Key, modifiers: Modifiers },
    TextInput { text: String },

    // ── 窗口 ──
    Resized { width: u32, height: u32 },
    ScaleFactorChanged { scale_factor: f64 },
    CloseRequested,
    Focused,
    Unfocused,

    // ── 触控板手势 ──
    PinchZoom { delta: f32 },
}
