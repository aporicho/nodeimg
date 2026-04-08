use crate::controls::infra::layout::BoxStyle;
use crate::controls::infra::widget::Widget;
use super::node::NodeKind;

/// 视图描述：view() 函数返回的轻量描述树。
pub enum Desc {
    /// 容器（布局概念：Column/Row/Scrollable 等）
    Container {
        id: &'static str,
        style: BoxStyle,
        children: Vec<Desc>,
    },
    /// 控件叶子
    Widget(Box<dyn Widget>),
}

impl Desc {
    /// 消费 Desc，返回 (子描述列表, NodeKind)。
    pub(crate) fn into_parts(self) -> (Vec<Desc>, NodeKind) {
        match self {
            Desc::Container { style, children, .. } => {
                (children, NodeKind::Container { style })
            }
            Desc::Widget(w) => {
                (Vec::new(), NodeKind::Widget(w))
            }
        }
    }
}

impl Desc {
    pub fn id(&self) -> &'static str {
        match self {
            Desc::Container { id, .. } => id,
            Desc::Widget(w) => w.id(),
        }
    }

    pub fn props_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        match self {
            Desc::Container { id, children, .. } => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                0u8.hash(&mut hasher);
                id.hash(&mut hasher);
                children.len().hash(&mut hasher);
                hasher.finish()
            }
            Desc::Widget(w) => w.props_hash(),
        }
    }
}
