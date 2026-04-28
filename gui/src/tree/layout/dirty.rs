use super::LayoutDirtyQueues;
use crate::tree::{DirtyQueues, NodeId};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutDirtySet {
    pub nodes: BTreeSet<NodeId>,
}

impl LayoutDirtySet {
    pub fn from_queues(queues: &DirtyQueues) -> Self {
        Self {
            nodes: queues.layout.clone(),
        }
    }
}

impl From<&DirtyQueues> for LayoutDirtyQueues {
    fn from(value: &DirtyQueues) -> Self {
        Self {
            boundaries: value.layout.clone(),
            text_nodes: value.text_layout.clone(),
        }
    }
}
