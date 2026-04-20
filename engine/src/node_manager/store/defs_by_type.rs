use crate::node_manager::NodeDef;
use std::collections::HashMap;

pub fn defs_by_type(defs: Vec<NodeDef>) -> HashMap<String, NodeDef> {
    defs.into_iter()
        .map(|def| (def.type_id.clone(), def))
        .collect()
}
