use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CategoryId(pub String);

impl CategoryId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

pub struct CategoryInfo {
    pub id: CategoryId,
    pub name: String,
    pub weight: i32, // lower = appears first
}

pub struct CategoryRegistry {
    categories: HashMap<CategoryId, CategoryInfo>,
}

impl Default for CategoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoryRegistry {
    pub fn new() -> Self {
        Self {
            categories: HashMap::new(),
        }
    }

    pub fn register(&mut self, info: CategoryInfo) {
        self.categories.insert(info.id.clone(), info);
    }

    pub fn get(&self, id: &CategoryId) -> Option<&CategoryInfo> {
        self.categories.get(id)
    }

    /// Returns all categories sorted by weight.
    pub fn sorted(&self) -> Vec<&CategoryInfo> {
        let mut cats: Vec<_> = self.categories.values().collect();
        cats.sort_by_key(|c| c.weight);
        cats
    }

    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        let cats = [
            ("data", "\u{6570}\u{636e}\u{578b}", 0),
            ("generate", "\u{751f}\u{6210}\u{578b}", 1),
            ("color", "\u{989c}\u{8272}\u{5904}\u{7406}\u{578b}", 2),
            ("transform", "\u{7a7a}\u{95f4}\u{53d8}\u{6362}\u{578b}", 3),
            ("filter", "\u{6ee4}\u{955c}\u{578b}", 4),
            ("composite", "\u{5408}\u{6210}\u{578b}", 5),
            ("tool", "\u{5de5}\u{5177}\u{578b}", 6),
            ("ai", "AI \u{751f}\u{6210}", 7),
        ];
        for (id, name, weight) in cats {
            reg.register(CategoryInfo {
                id: CategoryId::new(id),
                name: name.to_string(),
                weight,
            });
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_query() {
        let reg = CategoryRegistry::with_builtins();
        let info = reg.get(&CategoryId::new("filter"));
        assert!(info.is_some());
        assert_eq!(info.unwrap().name, "\u{6ee4}\u{955c}\u{578b}");
    }

    #[test]
    fn test_sorted_by_weight() {
        let reg = CategoryRegistry::with_builtins();
        let sorted = reg.sorted();
        assert_eq!(sorted[0].id, CategoryId::new("data"));
        assert_eq!(sorted[6].id, CategoryId::new("tool"));
    }
}
