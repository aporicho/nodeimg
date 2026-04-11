use crate::registry::NodeDef;
use std::collections::BTreeSet;

pub fn list_categories<'a>(defs: impl Iterator<Item = &'a NodeDef>) -> Vec<String> {
    defs.map(|def| def.category.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn list_nodes_by_category<'a>(
    defs: impl Iterator<Item = &'a NodeDef>,
    category: &str,
) -> Vec<&'a NodeDef> {
    defs.filter(|def| def.category == category).collect()
}
