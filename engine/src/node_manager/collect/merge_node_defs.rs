use crate::node_manager::NodeDef;

pub fn merge_node_defs(rust_defs: Vec<NodeDef>, python_defs: Vec<NodeDef>) -> Vec<NodeDef> {
    let mut merged = Vec::new();

    for def in rust_defs.into_iter().chain(python_defs) {
        if let Some(existing) = merged
            .iter()
            .position(|item: &NodeDef| item.type_id == def.type_id)
        {
            eprintln!("[warn] 节点类型 '{}' 重复注册，覆盖", def.type_id);
            merged[existing] = def;
        } else {
            merged.push(def);
        }
    }

    merged
}
