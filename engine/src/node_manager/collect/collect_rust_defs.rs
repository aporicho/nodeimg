use crate::registry::{NodeDef, NodeDefEntry};

pub fn collect_rust_defs() -> Vec<NodeDef> {
    inventory::iter::<NodeDefEntry>
        .into_iter()
        .map(|entry| (entry.0)())
        .collect()
}
