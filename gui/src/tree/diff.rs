use super::desc::Desc;
use super::layout::LeafKind;
use super::node::{NodeId, NodeKind, NodeLocalRuntime, TreeNode};
use super::repaint::{NodePaintMeta, RepaintBoundaryReason};
use super::tree::Tree;
use super::NodeProps;
use crate::renderer::Rect;
use crate::widget::props::WidgetBuildCx;
use std::collections::{HashMap, HashSet};

pub fn reconcile(tree: &mut Tree, desc: Desc, cx: WidgetBuildCx<'_>) {
    if let Some(root_id) = tree.root() {
        reconcile_node(tree, root_id, desc, cx);
    } else {
        let root_id = create_from_desc(tree, desc, cx);
        tree.set_root(root_id);
    }
}

fn reconcile_node(tree: &mut Tree, node_id: NodeId, desc: Desc, cx: WidgetBuildCx<'_>) {
    match desc {
        Desc::Container {
            id: _,
            style,
            decoration,
            children,
        } => {
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &decoration, &NodeKind::Container) {
                    node.style = style;
                    node.decoration = decoration;
                    node.kind = NodeKind::Container;
                }
            }
            reconcile_children(tree, node_id, children, cx);
        }
        Desc::Leaf { id: _, style, kind } => {
            let new_kind = NodeKind::Leaf(kind);
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &None, &new_kind) {
                    node.style = style;
                    node.decoration = None;
                    node.kind = new_kind;
                }
            }
        }
        Desc::Widget(widget) => {
            let (id, props, children) = widget.into_parts();
            tree.record_widget_build_call();
            let mut wb = props.build(&id, &cx);
            wb.children.extend(children);
            let props_changed = tree
                .get(node_id)
                .map(|n| match &n.kind {
                    NodeKind::Widget(old) => !old.props_eq(props.as_ref()),
                    _ => true,
                })
                .unwrap_or(true)
                || cx.force_rebuild;

            if props_changed {
                if let Some(node) = tree.get_mut(node_id) {
                    node.style = wb.style;
                    node.decoration = wb.decoration;
                    node.kind = NodeKind::Widget(props);
                }
            }
            reconcile_children(tree, node_id, wb.children, cx);
        }
    }
}

fn reconcile_children(
    tree: &mut Tree,
    node_id: NodeId,
    desc_children: Vec<Desc>,
    cx: WidgetBuildCx<'_>,
) {
    let old_children: Vec<NodeId> = tree
        .get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();

    let mut new_children = Vec::with_capacity(desc_children.len());
    let mut old_map: HashMap<String, NodeId> = HashMap::with_capacity(old_children.len());
    for (old_id, stable_id) in old_children
        .iter()
        .filter_map(|&id| tree.get(id).map(|n| (id, n.id.as_ref().to_owned())))
    {
        debug_assert!(
            old_map.insert(stable_id.clone(), old_id).is_none(),
            "duplicate sibling stable id during legacy reconcile: {stable_id}"
        );
    }

    let mut next_ids = HashSet::with_capacity(desc_children.len());
    for child_desc in desc_children {
        let child_id_str = child_desc.id();
        debug_assert!(
            next_ids.insert(child_id_str.to_string()),
            "duplicate new sibling stable id during legacy reconcile: {child_id_str}"
        );
        if let Some(existing_id) = old_map.remove(child_id_str) {
            tree.record_reconcile_child_match();
            reconcile_node(tree, existing_id, child_desc, cx);
            new_children.push(existing_id);
        } else {
            let new_id = create_from_desc(tree, child_desc, cx);
            new_children.push(new_id);
        }
    }

    for (_, old_id) in old_map {
        tree.remove(old_id);
    }

    tree.set_children(node_id, new_children);
}

fn create_from_desc(tree: &mut Tree, desc: Desc, cx: WidgetBuildCx<'_>) -> NodeId {
    let (id, style, decoration, kind, child_descs) = match desc {
        Desc::Container {
            id,
            style,
            decoration,
            children,
        } => (id, style, decoration, NodeKind::Container, children),
        Desc::Leaf { id, style, kind } => {
            (id, style, None, NodeKind::Leaf(kind), Vec::<Desc>::new())
        }
        Desc::Widget(widget) => {
            let (id, props, children) = widget.into_parts();
            tree.record_widget_build_call();
            let mut wb = props.build(&id, &cx);
            wb.children.extend(children);
            (
                id,
                wb.style,
                wb.decoration,
                NodeKind::Widget(props),
                wb.children,
            )
        }
    };

    let id_string = id.as_ref().to_string();
    let runtime_slots = tree
        .take_retained_runtime_slots(&id_string)
        .unwrap_or_default();
    let paint_meta = legacy_paint_meta(&id_string, &kind);

    let node_id = tree.insert(TreeNode {
        id: id.into(),
        props: NodeProps::default(),
        style,
        decoration,
        kind,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: Default::default(),
        paint_meta,
        runtime_slots,
    });

    let child_ids: Vec<NodeId> = child_descs
        .into_iter()
        .map(|d| create_from_desc(tree, d, cx))
        .collect();

    tree.set_children(node_id, child_ids);

    node_id
}

fn legacy_paint_meta(id: &str, kind: &NodeKind) -> NodePaintMeta {
    if id == "canvas_connections" {
        return NodePaintMeta::boundary(RepaintBoundaryReason::CanvasConnectionLayer);
    }
    if id.contains("canvas_node::") && id.ends_with("::card") {
        return NodePaintMeta::boundary(RepaintBoundaryReason::CanvasNodeCard);
    }
    if matches!(kind, NodeKind::Leaf(LeafKind::Grid { .. })) {
        return NodePaintMeta::boundary(RepaintBoundaryReason::CanvasGridLayer);
    }
    NodePaintMeta::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Color;
    use crate::theme::light_theme;
    use crate::tree::layout::{BoxStyle, LeafKind, Size};
    use crate::widget::atoms::dot::DotProps;
    use crate::widget::WidgetDesc;
    use std::borrow::Cow;

    #[test]
    fn widget_desc_reconciles_external_children() {
        let theme = light_theme();
        let mut tree = Tree::new();
        let desc = Desc::Widget(WidgetDesc::with_children(
            "wrapper",
            DotProps {
                diameter: 10.0,
                fill: Color::WHITE,
                border: None,
                hittable: false,
                gestures: Vec::new(),
            },
            vec![Desc::Leaf {
                id: Cow::Borrowed("wrapper::child"),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: "child".to_string(),
                    style: theme.text_style_label_sm(),
                    layout: Default::default(),
                },
            }],
        ));

        reconcile(
            &mut tree,
            desc,
            WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        let root = tree.root().expect("root");
        let root = tree.get(root).expect("root node");
        assert_eq!(root.children.len(), 1);
        let child = tree.get(root.children[0]).expect("child node");
        assert_eq!(child.id.as_ref(), "wrapper::child");
    }

    #[test]
    fn reconcile_children_matches_by_key_in_linear_time() {
        let theme = light_theme();
        let mut tree = Tree::new();
        let first = Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle::default(),
            decoration: None,
            children: vec![
                Desc::Leaf {
                    id: Cow::Borrowed("a"),
                    style: BoxStyle::default(),
                    kind: LeafKind::Text {
                        content: "a".to_string(),
                        style: theme.text_style_label_sm(),
                        layout: Default::default(),
                    },
                },
                Desc::Leaf {
                    id: Cow::Borrowed("b"),
                    style: BoxStyle::default(),
                    kind: LeafKind::Text {
                        content: "b".to_string(),
                        style: theme.text_style_label_sm(),
                        layout: Default::default(),
                    },
                },
            ],
        };
        reconcile(
            &mut tree,
            first,
            WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );
        let a = tree.node_by_str("a").expect("a");
        let b = tree.node_by_str("b").expect("b");
        tree.clear_frame_stats();

        let second = Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle::default(),
            decoration: None,
            children: vec![
                Desc::Leaf {
                    id: Cow::Borrowed("b"),
                    style: BoxStyle::default(),
                    kind: LeafKind::Text {
                        content: "b2".to_string(),
                        style: theme.text_style_label_sm(),
                        layout: Default::default(),
                    },
                },
                Desc::Leaf {
                    id: Cow::Borrowed("a"),
                    style: BoxStyle::default(),
                    kind: LeafKind::Text {
                        content: "a2".to_string(),
                        style: theme.text_style_label_sm(),
                        layout: Default::default(),
                    },
                },
            ],
        };
        reconcile(
            &mut tree,
            second,
            WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );

        assert_eq!(tree.node_by_str("a"), Some(a));
        assert_eq!(tree.node_by_str("b"), Some(b));
        assert_eq!(tree.frame_stats_snapshot().reconcile_child_matches, 2);
    }
}
