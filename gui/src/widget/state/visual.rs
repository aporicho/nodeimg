/// 控件可见交互态。由框架运行时状态与 disabled 配置共同决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetVisualState {
    Normal,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}
