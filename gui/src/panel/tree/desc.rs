use std::hash::{Hash, Hasher};
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
use super::node::NodeKind;

/// 视图描述：view() 函数返回的轻量描述树。
pub enum Desc {
    Container {
        id: &'static str,
        style: BoxStyle,
        decoration: Option<Decoration>,
        children: Vec<Desc>,
    },
    Leaf {
        id: &'static str,
        style: BoxStyle,
        kind: LeafKind,
    },
}

impl Desc {
    pub fn id(&self) -> &'static str {
        match self {
            Desc::Container { id, .. } => id,
            Desc::Leaf { id, .. } => id,
        }
    }

    /// 消费 Desc，返回 (子描述列表, NodeKind)。
    pub(crate) fn into_parts(self) -> (Vec<Desc>, NodeKind) {
        match self {
            Desc::Container { style, decoration, children, .. } => {
                (children, NodeKind::Container { style, decoration })
            }
            Desc::Leaf { style, kind, .. } => {
                (Vec::new(), NodeKind::Leaf { style, kind })
            }
        }
    }

    pub fn props_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Desc::Container { id, children, decoration, .. } => {
                0u8.hash(&mut hasher);
                id.hash(&mut hasher);
                children.len().hash(&mut hasher);
                if let Some(dec) = decoration {
                    1u8.hash(&mut hasher);
                    if let Some(bg) = &dec.background {
                        bg.r.to_bits().hash(&mut hasher);
                        bg.g.to_bits().hash(&mut hasher);
                        bg.b.to_bits().hash(&mut hasher);
                        bg.a.to_bits().hash(&mut hasher);
                    }
                }
            }
            Desc::Leaf { id, kind, .. } => {
                1u8.hash(&mut hasher);
                id.hash(&mut hasher);
                match kind {
                    LeafKind::Text { content, font_size, color } => {
                        content.hash(&mut hasher);
                        font_size.to_bits().hash(&mut hasher);
                        color.r.to_bits().hash(&mut hasher);
                        color.g.to_bits().hash(&mut hasher);
                        color.b.to_bits().hash(&mut hasher);
                        color.a.to_bits().hash(&mut hasher);
                    }
                }
            }
        }
        hasher.finish()
    }
}
