use winit::window::CursorIcon;

/// 光标样式。各模块设置，shell 每帧应用到窗口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Default,
    Move,
    ResizeN,
    ResizeS,
    ResizeW,
    ResizeE,
    ResizeNW,
    ResizeNE,
    ResizeSW,
    ResizeSE,
    Pointer,  // 手形，用于可点击元素
}

impl CursorStyle {
    pub fn to_winit(self) -> CursorIcon {
        match self {
            Self::Default => CursorIcon::Default,
            Self::Move => CursorIcon::Move,
            Self::ResizeN | Self::ResizeS => CursorIcon::NsResize,
            Self::ResizeW | Self::ResizeE => CursorIcon::EwResize,
            Self::ResizeNW | Self::ResizeSE => CursorIcon::NwseResize,
            Self::ResizeNE | Self::ResizeSW => CursorIcon::NeswResize,
            Self::Pointer => CursorIcon::Pointer,
        }
    }
}

/// 全局光标状态。每帧重置为 Default，各模块按需设置。
pub struct CursorState {
    current: CursorStyle,
    applied: CursorStyle,
}

impl CursorState {
    pub fn new() -> Self {
        Self {
            current: CursorStyle::Default,
            applied: CursorStyle::Default,
        }
    }

    /// 每帧开始时重置为 Default。
    pub fn reset(&mut self) {
        self.current = CursorStyle::Default;
    }

    /// 设置本帧的光标样式。后设置的覆盖先设置的。
    pub fn set(&mut self, style: CursorStyle) {
        self.current = style;
    }

    /// 如果光标样式变了，应用到窗口。
    pub fn apply(&mut self, window: &winit::window::Window) {
        if self.current != self.applied {
            window.set_cursor(self.current.to_winit());
            self.applied = self.current;
        }
    }
}
