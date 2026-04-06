pub mod def;
pub mod inventory_collect;

pub use def::*;
pub use inventory_collect::NodeDefEntry;

use types::{DataType, Value};
use std::collections::HashMap;

/// 节点管理器，存储和查询所有已注册的节点类型定义。
pub struct NodeManager {
    nodes: HashMap<String, NodeDef>,
}

impl NodeManager {
    /// 从 inventory 收集所有注册的 NodeDef。
    pub fn from_inventory() -> Self {
        let mut nodes = HashMap::new();
        for entry in inventory::iter::<NodeDefEntry> {
            let def = (entry.0)();
            if nodes.contains_key(&def.type_id) {
                eprintln!("[warn] 节点类型 '{}' 重复注册，覆盖", def.type_id);
            }
            nodes.insert(def.type_id.clone(), def);
        }
        Self { nodes }
    }

    /// 创建空的 NodeManager（测试用）。
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// 手动注册（测试用）。
    pub fn register(&mut self, def: NodeDef) {
        self.nodes.insert(def.type_id.clone(), def);
    }

    /// 按 type_id 获取节点定义。
    pub fn get(&self, type_id: &str) -> Option<&NodeDef> {
        self.nodes.get(type_id)
    }

    /// 按分类列出节点。
    pub fn list_by_category(&self, category: &str) -> Vec<&NodeDef> {
        self.nodes
            .values()
            .filter(|n| n.category == category)
            .collect()
    }

    /// 搜索节点（按名称或 type_id 模糊匹配）。
    pub fn search(&self, query: &str) -> Vec<&NodeDef> {
        let q = query.to_lowercase();
        self.nodes
            .values()
            .filter(|n| n.name.to_lowercase().contains(&q) || n.type_id.contains(&q))
            .collect()
    }

    /// 获取节点的默认参数值。
    pub fn default_params(&self, type_id: &str) -> Option<HashMap<String, Value>> {
        self.get(type_id).map(|def| {
            def.params
                .iter()
                .map(|p| (p.name.clone(), p.default_value.clone()))
                .collect()
        })
    }

    /// 查询某节点某引脚的数据类型。
    pub fn pin_type(&self, type_id: &str, pin_name: &str) -> Option<&DataType> {
        let def = self.get(type_id)?;
        def.inputs
            .iter()
            .chain(def.outputs.iter())
            .find(|p| p.name == pin_name)
            .map(|p| &p.data_type)
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_def(type_id: &str, category: &str) -> NodeDef {
        NodeDef {
            type_id: type_id.into(),
            name: type_id.into(),
            category: category.into(),
            inputs: vec![PinDef {
                name: "in".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "amount".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(0.0),
            }],
            execute: Box::new(|inputs| Box::pin(async move { Ok(inputs) })),
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        assert!(nm.get("brightness").is_some());
        assert!(nm.get("nonexistent").is_none());
    }

    #[test]
    fn test_pin_type() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        assert_eq!(nm.pin_type("brightness", "in"), Some(&DataType::image()));
        assert_eq!(nm.pin_type("brightness", "out"), Some(&DataType::image()));
        assert_eq!(nm.pin_type("brightness", "nope"), None);
    }

    #[test]
    fn test_default_params() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        let params = nm.default_params("brightness").unwrap();
        assert!(matches!(params.get("amount"), Some(Value::Float(f)) if *f == 0.0));
    }

    #[test]
    fn test_list_by_category() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        nm.register(make_test_def("blur", "filter"));
        assert_eq!(nm.list_by_category("color").len(), 1);
        assert_eq!(nm.list_by_category("filter").len(), 1);
    }

    #[test]
    fn test_search() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        nm.register(make_test_def("blur", "filter"));
        assert_eq!(nm.search("bright").len(), 1);
        assert_eq!(nm.search("b").len(), 2);
    }
}
