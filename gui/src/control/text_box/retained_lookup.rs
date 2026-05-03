use crate::control::ControlRole;
use crate::renderer::Rect;
use crate::tree::Tree;

pub(super) fn text_box_owner_from_retained_node(tree: &Tree, id: &str) -> Option<String> {
    if retained_text_box_role(tree, id).is_some() {
        return Some(id.to_string());
    }
    for prefix in retained_prefixes(id) {
        if retained_text_box_role(tree, prefix).is_some() {
            return Some(prefix.to_string());
        }
    }
    None
}

fn retained_text_box_role(tree: &Tree, id: &str) -> Option<ControlRole> {
    let node = tree.get(tree.node_by_str(id)?)?;
    match node.props.semantic_role? {
        role @ (ControlRole::TextInput | ControlRole::TextArea | ControlRole::NumberInput) => {
            Some(role)
        }
        _ => None,
    }
}

pub(super) fn retained_prefixes(id: &str) -> impl Iterator<Item = &str> {
    id.match_indices("::")
        .map(|(index, _)| &id[..index])
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
}

pub(super) fn find_rect(tree: &Tree, node_id: &str) -> Option<Rect> {
    tree.node_by_str(node_id)
        .and_then(|node_id| tree.get(node_id))
        .map(|node| node.rect)
}
