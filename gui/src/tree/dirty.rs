use super::NodeId;
use std::collections::BTreeSet;
use std::ops::{BitOr, BitOrAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyFlags(u16);

impl DirtyFlags {
    pub const NONE: Self = Self(0);
    pub const STRUCTURE: Self = Self(1 << 0);
    pub const STYLE: Self = Self(1 << 1);
    pub const LAYOUT: Self = Self(1 << 2);
    pub const TEXT_LAYOUT: Self = Self(1 << 3);
    pub const PAINT: Self = Self(1 << 4);
    pub const HIT: Self = Self(1 << 5);
    pub const PAINT_ORDER: Self = Self(1 << 6);
    pub const COMPOSITE: Self = Self(1 << 7);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for DirtyFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for DirtyFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirtyQueues {
    pub structure: BTreeSet<NodeId>,
    pub layout: BTreeSet<NodeId>,
    pub text_layout: BTreeSet<NodeId>,
    pub paint: BTreeSet<NodeId>,
    pub hit: BTreeSet<NodeId>,
    pub paint_order: BTreeSet<NodeId>,
    pub composite: BTreeSet<NodeId>,
}

impl DirtyQueues {
    pub fn mark(&mut self, node: NodeId, flags: DirtyFlags) {
        if flags.contains(DirtyFlags::STRUCTURE) {
            self.structure.insert(node);
        }
        if flags.contains(DirtyFlags::LAYOUT) {
            self.layout.insert(node);
        }
        if flags.contains(DirtyFlags::TEXT_LAYOUT) {
            self.text_layout.insert(node);
        }
        if flags.contains(DirtyFlags::PAINT) {
            self.paint.insert(node);
        }
        if flags.contains(DirtyFlags::HIT) {
            self.hit.insert(node);
        }
        if flags.contains(DirtyFlags::PAINT_ORDER) {
            self.paint_order.insert(node);
        }
        if flags.contains(DirtyFlags::COMPOSITE) {
            self.composite.insert(node);
        }
    }
}
