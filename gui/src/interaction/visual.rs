#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetVisualState {
    Normal,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}
