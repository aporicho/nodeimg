use super::def::NodeDef;

/// inventory 收集用的包装类型。
/// node! 宏展开时调用 inventory::submit!(NodeDefEntry(|| { ... }))
pub struct NodeDefEntry(pub fn() -> NodeDef);

inventory::collect!(NodeDefEntry);
