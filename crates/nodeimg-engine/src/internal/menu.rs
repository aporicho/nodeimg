use nodeimg_types::category::{CategoryId, CategoryRegistry};
use crate::internal::registry::NodeRegistry;

pub struct MenuItem {
    pub type_id: String,
    pub title: String,
}

pub struct MenuCategory {
    pub id: CategoryId,
    pub name: String,
    pub items: Vec<MenuItem>,
}

pub struct Menu;

impl Menu {
    /// Generate categorized menu from registries.
    pub fn generate(node_reg: &NodeRegistry, cat_reg: &CategoryRegistry) -> Vec<MenuCategory> {
        let mut result = Vec::new();
        for cat in cat_reg.sorted() {
            let nodes = node_reg.list(Some(&cat.id));
            if nodes.is_empty() {
                continue;
            }
            result.push(MenuCategory {
                id: cat.id.clone(),
                name: cat.name.clone(),
                items: nodes
                    .iter()
                    .map(|n| MenuItem {
                        type_id: n.type_id.clone(),
                        title: n.title.clone(),
                    })
                    .collect(),
            });
        }
        result
    }

    /// Filter nodes by search keyword (matches title or category name).
    #[allow(dead_code)] // Used in tests; will be exposed via transport when search UI is added
    pub fn search(
        keyword: &str,
        node_reg: &NodeRegistry,
        cat_reg: &CategoryRegistry,
    ) -> Vec<MenuItem> {
        let kw = keyword.to_lowercase();
        node_reg
            .list(None)
            .iter()
            .filter(|n| {
                n.title.to_lowercase().contains(&kw)
                    || cat_reg
                        .get(&n.category)
                        .is_some_and(|c| c.name.to_lowercase().contains(&kw))
            })
            .map(|n| MenuItem {
                type_id: n.type_id.clone(),
                title: n.title.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::registry::{NodeDef, PinDef};
    use nodeimg_types::data_type::DataTypeId;

    fn test_registries() -> (NodeRegistry, CategoryRegistry) {
        let cat_reg = CategoryRegistry::with_builtins();
        let mut node_reg = NodeRegistry::new();
        node_reg.register(NodeDef {
            type_id: "invert".into(),
            title: "Invert".into(),
            category: CategoryId::new("color"),
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            }],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: Some(Box::new(|_, _| std::collections::HashMap::new())),
            gpu_process: None,
        });
        (node_reg, cat_reg)
    }

    #[test]
    fn test_menu_generation() {
        let (node_reg, cat_reg) = test_registries();
        let menu = Menu::generate(&node_reg, &cat_reg);
        assert_eq!(menu.len(), 1);
        assert_eq!(menu[0].items.len(), 1);
    }

    #[test]
    fn test_menu_search() {
        let (node_reg, cat_reg) = test_registries();
        let results = Menu::search("invert", &node_reg, &cat_reg);
        assert_eq!(results.len(), 1);
        let results = Menu::search("nonexistent", &node_reg, &cat_reg);
        assert_eq!(results.len(), 0);
    }
}
