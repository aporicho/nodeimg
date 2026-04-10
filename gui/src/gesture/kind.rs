/// 节点可声明的手势类型。
///
/// 命中测试后，hit chain 上每个节点的 gestures 列表告诉手势竞技场
/// 应该注册哪些识别器。本枚举与 gesture/ 下已有的识别器一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gesture {
    Tap,
    DoubleTap,
    Drag,
    LongPress,
}
