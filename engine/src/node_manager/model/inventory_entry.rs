use super::NodeDef;

/// inventory 收集用的包装类型。
/// node! 宏展开时调用 inventory::submit!(NodeDefEntry(|| { ... }))
pub struct NodeDefEntry(pub fn() -> NodeDef);

inventory::collect!(NodeDefEntry);

include!(concat!(env!("OUT_DIR"), "/python_nodes_inventory.rs"));
