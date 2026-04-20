use crate::node_manager::{NodeDef, NodeDefEntry};

pub fn collect_inventory_defs() -> Vec<NodeDef> {
    inventory::iter::<NodeDefEntry>
        .into_iter()
        .map(|entry| (entry.0)())
        .collect()
}
