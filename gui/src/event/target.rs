use crate::tree::{NodeId, NodeKind, Tree};

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

    #[allow(dead_code)]
    pub(crate) fn owner_node(&self, node_id: NodeId) -> Option<NodeId> {
        let node = self.tree.get(node_id)?;
        if matches!(&node.kind, NodeKind::Widget(_)) {
            return Some(node_id);
        }

        let owner_id = self.owner_id(node.id.as_ref());
        self.tree.iter().find_map(|(candidate_id, candidate)| {
            (candidate.id.as_ref() == owner_id && matches!(&candidate.kind, NodeKind::Widget(_)))
                .then_some(candidate_id)
        })
    }

    #[allow(dead_code)]
    pub(crate) fn widget_type_for_node(&self, node_id: NodeId) -> Option<&'static str> {
        let owner_id = self.owner_node(node_id)?;
        let node = self.tree.get(owner_id)?;
        let NodeKind::Widget(props) = &node.kind else {
            return None;
        };
        Some(props.widget_type())
    }

    pub(crate) fn widget_type(&self, id: &str) -> Option<&'static str> {
        self.widget_type_for_name(id)
    }

    pub(crate) fn widget_type_for_name(&self, id: &str) -> Option<&'static str> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::theme::dark_theme;
    use crate::tree::layout::{BoxStyle, Size};
    use crate::tree::{layout, reconcile, Desc};
    use crate::widget::atoms::button::ButtonProps;
    use crate::widget::atoms::slider::SliderProps;
    use crate::widget::atoms::text_input::TextInputProps;
    use crate::widget::frameworks::panel::PanelProps;
    use crate::widget::props::WidgetBuildCx;
    use std::borrow::Cow;

    fn build_tree(children: Vec<Desc>) -> Tree {
        let theme = dark_theme();
        let mut tree = Tree::new();
        reconcile(
            &mut tree,
            Desc::Container {
                id: Cow::Borrowed("root"),
                style: BoxStyle {
                    width: Size::Fixed(640.0),
                    height: Size::Fixed(480.0),
                    ..BoxStyle::default()
                },
                decoration: None,
                children,
            },
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
        let tree = build_tree(vec![Desc::Widget {
            id: Cow::Borrowed("button"),
            props: Box::new(ButtonProps {
                label: Cow::Borrowed("Run"),
                icon: None,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            }),
        }]);
        let resolver = TargetResolver::new(&tree);
        let label = node_by_name(&tree, "button::label");

        assert_eq!(resolver.owner_id("button::label"), "button");
        assert_eq!(resolver.widget_type_for_node(label), Some("Button"));
    }

    #[test]
    fn panel_titlebar_resolves_to_panel() {
        let tree = build_tree(vec![Desc::Widget {
            id: Cow::Borrowed("panel"),
            props: Box::new(PanelProps {
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
            }),
        }]);
        let resolver = TargetResolver::new(&tree);
        let titlebar = node_by_name(&tree, "panel::titlebar");

        assert_eq!(resolver.owner_id("panel::titlebar"), "panel");
        assert_eq!(resolver.widget_type_for_node(titlebar), Some("Panel"));
    }

    #[test]
    fn text_input_field_resolves_to_text_input() {
        let tree = build_tree(vec![Desc::Widget {
            id: Cow::Borrowed("input"),
            props: Box::new(TextInputProps {
                label: Cow::Borrowed("Prompt"),
                value: Cow::Borrowed("hello"),
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            }),
        }]);
        let resolver = TargetResolver::new(&tree);
        let field = node_by_name(&tree, "input::field");

        assert_eq!(resolver.owner_id("input::field"), "input");
        assert_eq!(resolver.widget_type_for_node(field), Some("TextInput"));
    }

    #[test]
    fn slider_thumb_resolves_to_slider() {
        let tree = build_tree(vec![Desc::Widget {
            id: Cow::Borrowed("slider"),
            props: Box::new(SliderProps {
                label: Cow::Borrowed("Radius"),
                min: 0.0,
                max: 10.0,
                step: 1.0,
                value: 5.0,
                disabled: false,
            }),
        }]);
        let resolver = TargetResolver::new(&tree);
        let thumb = node_by_name(&tree, "slider::thumb");

        assert_eq!(resolver.owner_id("slider::thumb"), "slider");
        assert_eq!(resolver.widget_type_for_node(thumb), Some("Slider"));
    }

    #[test]
    fn longest_widget_prefix_supports_existing_ids_with_separator() {
        let tree = build_tree(vec![Desc::Widget {
            id: Cow::Borrowed("gallery_panel::basic"),
            props: Box::new(PanelProps {
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
            }),
        }]);
        let resolver = TargetResolver::new(&tree);

        assert_eq!(
            resolver.owner_id("gallery_panel::basic::titlebar"),
            "gallery_panel::basic"
        );
    }
}
