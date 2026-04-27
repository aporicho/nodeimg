use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::props::WidgetProps;

pub(super) struct WidgetOwner<'tree, 'node> {
    pub(super) root_id: &'node str,
    pub(super) root_node_id: NodeId,
    pub(super) props: &'tree dyn WidgetProps,
}

pub(super) fn widget_owner_for_part<'tree, 'node>(
    tree: &'tree Tree,
    node_id: &'node str,
    matches_props: impl Fn(&dyn WidgetProps) -> bool,
) -> Option<WidgetOwner<'tree, 'node>> {
    for candidate in part_candidates(node_id) {
        let Some((root_node_id, root_node)) =
            tree.iter().find(|(_, node)| node.id.as_ref() == candidate)
        else {
            continue;
        };
        let NodeKind::Widget(props) = &root_node.kind else {
            continue;
        };
        if matches_props(props.as_ref()) {
            return Some(WidgetOwner {
                root_id: candidate,
                root_node_id,
                props: props.as_ref(),
            });
        }
    }
    None
}

fn part_candidates(node_id: &str) -> impl Iterator<Item = &str> {
    [
        Some(node_id),
        node_id.strip_suffix("::field"),
        node_id.strip_suffix("::value"),
    ]
    .into_iter()
    .flatten()
}
