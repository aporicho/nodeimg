#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlVisualState {
    Normal,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}
