use crate::registry::NodeDef;

pub fn search_nodes<'a>(defs: impl Iterator<Item = &'a NodeDef>, query: &str) -> Vec<&'a NodeDef> {
    let q = query.to_lowercase();
    defs.filter(|def| {
        def.name.to_lowercase().contains(&q) || def.type_id.to_lowercase().contains(&q)
    })
    .collect()
}
