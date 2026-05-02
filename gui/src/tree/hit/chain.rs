use crate::tree::NodeId;

/// 从命中的叶子到根的节点链。
/// 链上 [0] 是最深的命中节点（叶子），[len-1] 是 root。
#[derive(Debug, Clone)]
pub struct HitChain {
    nodes: Vec<NodeId>,
}

impl HitChain {
    pub fn new(nodes: Vec<NodeId>) -> Self {
        Self { nodes }
    }

    pub fn empty() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 最深的命中节点（叶子）
    pub fn leaf(&self) -> Option<NodeId> {
        self.nodes.first().copied()
    }

    /// 最外层的命中节点（root）
    pub fn root(&self) -> Option<NodeId> {
        self.nodes.last().copied()
    }

    /// 从叶子到根顺序遍历
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().copied()
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains(&id)
    }
}
