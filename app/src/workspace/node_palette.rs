use gui::action::NODE_LIBRARY_ADD_PREFIX;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodePaletteState {
    pub(crate) items: Vec<NodePaletteItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodePaletteItem {
    pub(crate) type_id: String,
    pub(crate) name: String,
    pub(crate) category: String,
    pub(crate) source: String,
}

pub(crate) fn node_palette_item_id(type_id: &str) -> String {
    format!("{NODE_LIBRARY_ADD_PREFIX}{type_id}")
}
