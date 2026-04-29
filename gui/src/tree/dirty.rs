use super::NodeId;
use std::collections::BTreeSet;
use std::fmt;
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
    pub const PAINT_PLACEMENT: Self = Self(1 << 8);

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

impl fmt::Display for DirtyFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return f.write_str("NONE");
        }

        let mut separator = "";
        for (flag, name) in [
            (Self::STRUCTURE, "STRUCTURE"),
            (Self::STYLE, "STYLE"),
            (Self::LAYOUT, "LAYOUT"),
            (Self::TEXT_LAYOUT, "TEXT_LAYOUT"),
            (Self::PAINT, "PAINT"),
            (Self::HIT, "HIT"),
            (Self::PAINT_ORDER, "PAINT_ORDER"),
            (Self::COMPOSITE, "COMPOSITE"),
            (Self::PAINT_PLACEMENT, "PAINT_PLACEMENT"),
        ] {
            if self.contains(flag) {
                f.write_str(separator)?;
                f.write_str(name)?;
                separator = "|";
            }
        }
        Ok(())
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
    pub paint_placement: BTreeSet<NodeId>,
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
        if flags.contains(DirtyFlags::PAINT_PLACEMENT) {
            self.paint_placement.insert(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_flags_display_uses_names() {
        let flags = DirtyFlags::LAYOUT | DirtyFlags::PAINT | DirtyFlags::HIT;

        assert_eq!(flags.to_string(), "LAYOUT|PAINT|HIT");
        assert_eq!(DirtyFlags::NONE.to_string(), "NONE");
    }
}
