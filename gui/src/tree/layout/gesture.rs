/// Gesture declarations that a tree node can advertise for input resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gesture {
    Tap,
    DoubleTap,
    Drag,
    LongPress,
    Resize,
}
