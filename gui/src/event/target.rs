use crate::tree::{NodeId, NodeKind, Tree};

pub(crate) struct TargetResolver<'a> {
    tree: &'a Tree,
}

impl<'a> TargetResolver<'a> {
    pub(crate) fn new(tree: &'a Tree) -> Self {
        Self { tree }
    }

    pub(crate) fn owner_id(&self, id: &str) -> String {
        let exact = self
            .tree
            .node_by_str(id)
            .and_then(|node_id| self.tree.get(node_id));
        if let Some(node_id) = self.tree.node_by_str(id) {
            if let Some(node) = self.tree.get(node_id) {
                if let Some(owner_id) = node.props.owner_id.as_ref() {
                    return owner_id.to_string();
                }
                if matches!(&node.kind, NodeKind::Widget(_)) || node.props.semantic_role.is_some() {
                    return node.id.to_string();
                }
            }
        }

        for prefix in stable_id_prefixes(id) {
            if let Some(owner) = self
                .tree
                .node_by_str(prefix)
                .and_then(|node_id| self.tree.get(node_id))
            {
                if matches!(&owner.kind, NodeKind::Widget(_)) || owner.props.semantic_role.is_some()
                {
                    return owner.id.to_string();
                }
            }
        }

        if exact.is_some_and(is_retained_interaction_target) {
            return id.to_string();
        }

        id.to_string()
    }

    #[allow(dead_code)]
    pub(crate) fn owner_node(&self, node_id: NodeId) -> Option<NodeId> {
        let node = self.tree.get(node_id)?;
        if matches!(&node.kind, NodeKind::Widget(_)) {
            return Some(node_id);
        }

        let owner_id = self.owner_id(node.id.as_ref());
        self.tree.node_by_str(&owner_id)
    }

    #[allow(dead_code)]
    pub(crate) fn widget_type_for_node(&self, node_id: NodeId) -> Option<&str> {
        let owner_id = self.owner_node(node_id)?;
        let node = self.tree.get(owner_id)?;
        match &node.kind {
            NodeKind::Widget(props) => Some(props.widget_type()),
            _ => node.props.semantic_role.as_deref(),
        }
    }

    pub(crate) fn widget_type(&self, id: &str) -> Option<&str> {
        self.widget_type_for_name(id)
    }

    pub(crate) fn widget_type_for_name(&self, id: &str) -> Option<&str> {
        let node = self.tree.get(self.tree.node_by_str(id)?)?;
        match &node.kind {
            NodeKind::Widget(props) => Some(props.widget_type()),
            _ => node.props.semantic_role.as_deref(),
        }
    }
}

fn is_retained_interaction_target(node: &crate::tree::TreeNode) -> bool {
    node.style.hittable == Some(true)
        || node.style.draggable
        || node.style.resizable
        || !node.style.gestures.is_empty()
        || node.props.action_id.is_some()
}

fn stable_id_prefixes(id: &str) -> impl Iterator<Item = &str> {
    id.match_indices("::")
        .map(|(index, _)| &id[..index])
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::theme::dark_theme;
    use crate::tree::{layout, reconcile, Desc};
    use crate::ui::{self, StyleBuilder};
    use crate::widget::atoms::button::ButtonProps;
    use crate::widget::atoms::slider::SliderProps;
    use crate::widget::atoms::text_input::TextInputProps;
    use crate::widget::frameworks::panel::PanelProps;
    use crate::widget::props::{WidgetBuildCx, WidgetProps};
    use std::borrow::Cow;

    fn widget(id: impl Into<Cow<'static, str>>, props: impl WidgetProps) -> Desc {
        ui::widget(id, props).build()
    }

    fn build_tree(children: Vec<Desc>) -> Tree {
        let theme = dark_theme();
        let mut tree = Tree::new();
        reconcile(
            &mut tree,
            ui::container("root")
                .fixed_width(640.0)
                .fixed_height(480.0)
                .children(children)
                .build(),
            WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );
        let root = tree.root().expect("tree root");
        layout(
            &mut tree,
            root,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 640.0,
                h: 480.0,
            },
            &mut |_text, _style| (20.0, 14.0),
        );
        tree
    }

    fn node_by_name(tree: &Tree, name: &str) -> NodeId {
        tree.iter()
            .find_map(|(id, node)| (node.id.as_ref() == name).then_some(id))
            .expect("node by name")
    }

    #[test]
    fn button_label_resolves_to_button() {
        let tree = build_tree(vec![widget(
            "button",
            ButtonProps {
                label: Cow::Borrowed("Run"),
                icon: None,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        )]);
        let resolver = TargetResolver::new(&tree);
        let label = node_by_name(&tree, "button::label");

        assert_eq!(resolver.owner_id("button::label"), "button");
        assert_eq!(resolver.widget_type_for_node(label), Some("Button"));
    }

    #[test]
    fn panel_titlebar_resolves_to_panel() {
        let tree = build_tree(vec![widget(
            "panel",
            PanelProps {
                title: Cow::Borrowed("Panel"),
                rect: Rect {
                    x: 20.0,
                    y: 20.0,
                    w: 240.0,
                    h: 160.0,
                },
                z_index: 0,
                min_size: [120.0, 80.0],
                titlebar_visible: true,
                draggable: true,
                resizable: true,
                closable: false,
                content: vec![],
            },
        )]);
        let resolver = TargetResolver::new(&tree);
        let titlebar = node_by_name(&tree, "panel::titlebar");

        assert_eq!(resolver.owner_id("panel::titlebar"), "panel");
        assert_eq!(resolver.widget_type_for_node(titlebar), Some("Panel"));
    }

    #[test]
    fn text_input_field_resolves_to_text_input() {
        let tree = build_tree(vec![widget(
            "input",
            TextInputProps {
                label: Some(Cow::Borrowed("Prompt")),
                value: Cow::Borrowed("hello"),
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        )]);
        let resolver = TargetResolver::new(&tree);
        let field = node_by_name(&tree, "input::field");

        assert_eq!(resolver.owner_id("input::field"), "input");
        assert_eq!(resolver.widget_type_for_node(field), Some("TextInput"));
    }

    #[test]
    fn slider_thumb_resolves_to_slider() {
        let tree = build_tree(vec![widget(
            "slider",
            SliderProps {
                label: Some(Cow::Borrowed("Radius")),
                min: 0.0,
                max: 10.0,
                step: 1.0,
                value: 5.0,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        )]);
        let resolver = TargetResolver::new(&tree);
        let thumb = node_by_name(&tree, "slider::thumb");

        assert_eq!(resolver.owner_id("slider::thumb"), "slider");
        assert_eq!(resolver.widget_type_for_node(thumb), Some("Slider"));
    }

    #[test]
    fn longest_widget_prefix_supports_existing_ids_with_separator() {
        let tree = build_tree(vec![widget(
            "gallery_panel::basic",
            PanelProps {
                title: Cow::Borrowed("Panel"),
                rect: Rect {
                    x: 20.0,
                    y: 20.0,
                    w: 240.0,
                    h: 160.0,
                },
                z_index: 0,
                min_size: [120.0, 80.0],
                titlebar_visible: true,
                draggable: true,
                resizable: true,
                closable: false,
                content: vec![],
            },
        )]);
        let resolver = TargetResolver::new(&tree);

        assert_eq!(
            resolver.owner_id("gallery_panel::basic::titlebar"),
            "gallery_panel::basic"
        );
    }
}
