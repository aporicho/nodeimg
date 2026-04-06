/// 节点图中节点的唯一标识符，自增分配。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

/// 引用某个节点的某个引脚。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PinRef {
    pub node: NodeId,
    pub pin: String,
}
