use super::LayoutDirtyQueues;
use crate::tree::DirtyQueues;

impl From<&DirtyQueues> for LayoutDirtyQueues {
    fn from(value: &DirtyQueues) -> Self {
        Self {
            boundaries: value.layout.clone(),
            text_nodes: value.text_layout.clone(),
        }
    }
}
