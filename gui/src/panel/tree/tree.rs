use crate::widget::node::{NodeId, PanelNode};

/// 面板树存储。用 Vec<Option<>> 做 arena，索引访问。
pub struct PanelTree {
    nodes: Vec<Option<PanelNode>>,
    root: Option<NodeId>,
    free: Vec<NodeId>,
}

impl PanelTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            free: Vec::new(),
        }
    }

    pub fn insert(&mut self, node: PanelNode) -> NodeId {
        if let Some(id) = self.free.pop() {
            self.nodes[id] = Some(node);
            id
        } else {
            let id = self.nodes.len();
            self.nodes.push(Some(node));
            id
        }
    }

    pub fn remove(&mut self, id: NodeId) {
        if id < self.nodes.len() {
            // 递归删除子节点
            if let Some(node) = self.nodes[id].take() {
                let children = node.children;
                for child_id in children {
                    self.remove(child_id);
                }
            }
            self.free.push(id);
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&PanelNode> {
        self.nodes.get(id).and_then(|n| n.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut PanelNode> {
        self.nodes.get_mut(id).and_then(|n| n.as_mut())
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PanelNode> {
        self.nodes.iter_mut().filter_map(|n| n.as_mut())
    }
}
