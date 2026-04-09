/// 控件的交互状态。不存在节点上，从竞技场和 hit test 结果推导。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionState {
    Normal,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}
