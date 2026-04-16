use crate::tree::{NodeKind, Tree};

pub(crate) struct TargetResolver<'a> {
    tree: &'a Tree,
}

impl<'a> TargetResolver<'a> {
    pub(crate) fn new(tree: &'a Tree) -> Self {
        Self { tree }
    }

    pub(crate) fn owner_id(&self, id: &str) -> String {
        let exact_widget = self
            .tree
            .iter()
            .find(|(_, node)| node.id.as_ref() == id && matches!(&node.kind, NodeKind::Widget(_)))
            .map(|(_, node)| node.id.to_string());
        if let Some(widget_id) = exact_widget {
            return widget_id;
        }

        self.tree
            .iter()
            .filter_map(|(_, node)| {
                if !matches!(&node.kind, NodeKind::Widget(_)) {
                    return None;
                }
                let candidate = node.id.as_ref();
                id.strip_prefix(candidate)
                    .filter(|suffix| suffix.starts_with("::"))
                    .map(|_| candidate)
            })
            .max_by_key(|candidate| candidate.len())
            .map(str::to_string)
            .unwrap_or_else(|| id.to_string())
    }

    pub(crate) fn widget_type(&self, id: &str) -> Option<&'static str> {
        self.tree.iter().find_map(|(_, node)| {
            if node.id.as_ref() != id {
                return None;
            }
            let NodeKind::Widget(props) = &node.kind else {
                return None;
            };
            Some(props.widget_type())
        })
    }
}
