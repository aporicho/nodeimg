use super::Tree;
use crate::tree::dirty::DirtyFlags;
use crate::tree::layout::LeafKind;
use crate::tree::node::NodeId;
use crate::tree::repaint::RepaintBoundaryId;
use crate::tree::runtime_slots::RuntimeSlots;
use crate::tree::snapshot::{
    index_error_message, repaint_boundary_list, DirtyQueueSummary, LayoutDirtyQueueSummary,
    PaintDirtyQueueSummary, TreeCacheSummary, TreeDumpLevel, TreeSnapshot, TreeSnapshotIssue,
    TreeSnapshotMaxNodes, TreeSnapshotNode, TreeSnapshotOptions, TreeSnapshotSummary,
};
use crate::tree::NodeKind;
use std::collections::HashSet;

impl Tree {
    pub fn debug_snapshot(&self, options: TreeSnapshotOptions) -> TreeSnapshot {
        let max_nodes = match options.level {
            TreeDumpLevel::Normal => options.max_nodes,
            TreeDumpLevel::Full => TreeSnapshotMaxNodes::All,
        };
        let mut issues = self.debug_snapshot_issues();
        let mut visited = HashSet::new();
        let mut ordered = Vec::<(usize, NodeId)>::new();

        if let Some(root) = self.root {
            collect_snapshot_order(self, root, 0, &mut visited, &mut ordered);
        }

        for (node_id, _) in self.iter() {
            if !visited.contains(&node_id) {
                issues.push(TreeSnapshotIssue {
                    message: format!("orphan or unreachable node node={node_id}"),
                });
                ordered.push((0, node_id));
            }
        }

        let live_nodes = self.iter().count();
        let limit = match max_nodes {
            TreeSnapshotMaxNodes::Limit(limit) => Some(limit),
            TreeSnapshotMaxNodes::All => None,
        };
        let included = limit.map_or(ordered.len(), |limit| ordered.len().min(limit));
        let truncated = included < ordered.len();
        let omitted_nodes = ordered.len().saturating_sub(included);
        let nodes = ordered
            .into_iter()
            .take(included)
            .filter_map(|(depth, node_id)| {
                let node = self.get(node_id)?;
                Some(TreeSnapshotNode {
                    node: node_id,
                    depth,
                    stable_id: node.id.as_ref().to_string(),
                    line: self.format_snapshot_node(node_id, depth, options.level),
                })
            })
            .collect::<Vec<_>>();

        let summary = TreeSnapshotSummary {
            level: options.level,
            root: self.root,
            live_nodes,
            indexed_nodes: self.index.len(),
            dirty: DirtyQueueSummary::from(&self.dirty),
            layout_dirty: LayoutDirtyQueueSummary {
                boundaries: self.layout_dirty.boundaries.len(),
                text_nodes: self.layout_dirty.text_nodes.len(),
            },
            paint_dirty: PaintDirtyQueueSummary::from(&self.paint_dirty),
            caches: TreeCacheSummary {
                layout_entries: self.layout_cache.len(),
                hit_order_entries: self.hit_order_cache.borrow().len(),
                paint_order_entries: self.paint_order_cache.borrow().len(),
                paint_fragments: self.paint_cache.borrow().len(),
            },
            issues: issues.len(),
            truncated,
            omitted_nodes,
        };

        TreeSnapshot {
            summary,
            nodes,
            issues,
        }
    }

    fn debug_snapshot_issues(&self) -> Vec<TreeSnapshotIssue> {
        let mut issues = Vec::new();
        match self.root {
            Some(root) if self.get(root).is_none() => issues.push(TreeSnapshotIssue {
                message: format!("root points to missing node root={root}"),
            }),
            None => issues.push(TreeSnapshotIssue {
                message: "missing root".to_string(),
            }),
            _ => {}
        }

        if let Err(error) = self.validate_index() {
            issues.push(TreeSnapshotIssue {
                message: index_error_message(error),
            });
        }

        for (parent_id, node) in self.iter() {
            let mut seen_children = HashSet::new();
            for child in node.children.iter().copied() {
                if !seen_children.insert(child) {
                    issues.push(TreeSnapshotIssue {
                        message: format!("duplicate child parent={parent_id} child={child}"),
                    });
                }
                if self.get(child).is_none() {
                    issues.push(TreeSnapshotIssue {
                        message: format!("missing child parent={parent_id} child={child}"),
                    });
                    continue;
                }
                if self.parents.get(&child).copied() != Some(parent_id) {
                    issues.push(TreeSnapshotIssue {
                        message: format!(
                            "parent map mismatch child={child} expected_parent={parent_id} actual_parent={:?}",
                            self.parents.get(&child).copied()
                        ),
                    });
                }
            }
        }

        for (child, parent) in &self.parents {
            if self.get(*child).is_none() {
                issues.push(TreeSnapshotIssue {
                    message: format!(
                        "parent map contains missing child child={child} parent={parent}"
                    ),
                });
                continue;
            }
            let Some(parent_node) = self.get(*parent) else {
                issues.push(TreeSnapshotIssue {
                    message: format!(
                        "parent map contains missing parent child={child} parent={parent}"
                    ),
                });
                continue;
            };
            if !parent_node.children.contains(child) {
                issues.push(TreeSnapshotIssue {
                    message: format!(
                        "parent map child not present in parent children child={child} parent={parent}"
                    ),
                });
            }
        }

        issues
    }

    fn format_snapshot_node(&self, node_id: NodeId, depth: usize, level: TreeDumpLevel) -> String {
        let Some(node) = self.get(node_id) else {
            return format!("{}node={node_id} <missing>", "  ".repeat(depth));
        };
        let indent = "  ".repeat(depth);
        let parent = self.parents.get(&node_id).copied();
        let dirty = self.snapshot_dirty_flags(node_id);
        let paint_boundary = RepaintBoundaryId(node_id);
        let normal = format!(
            "{indent}node={node_id} stable_id={:?} kind={} parent={parent:?} children={:?} rect={:?} visible={} dirty={} layout_dirty_boundary={} layout_dirty_text={} paint_dirty_boundary={} paint_dirty_order={} composite_dirty={} layout_boundary={:?} paint_boundary={:?} layout_rev=(style:{} text:{} children:{} rect:{} dependency:{}) paint_rev=(visual:{} text:{} order:{} fragment:{}) runtime_slots={}",
            node.id.as_ref(),
            normal_kind_summary(&node.kind),
            node.children,
            node.rect,
            node.local_runtime.visible,
            dirty,
            self.layout_dirty.boundaries.contains(&node_id),
            self.layout_dirty.text_nodes.contains(&node_id),
            self.paint_dirty.boundaries.contains(&paint_boundary),
            self.paint_dirty.paint_order.contains(&paint_boundary),
            self.paint_dirty.composite.contains(&node_id),
            node.layout_meta.boundary,
            node.paint_meta.boundary,
            node.layout_meta.style_revision.get(),
            node.layout_meta.text_revision.get(),
            node.layout_meta.children_revision.get(),
            node.layout_meta.explicit_rect_revision.get(),
            node.layout_meta.layout_dependency_revision.get(),
            node.paint_meta.visual_revision.get(),
            node.paint_meta.text_paint_revision.get(),
            node.paint_meta.paint_order_revision.get(),
            node.paint_meta.fragment_revision.get(),
            node.runtime_slots.len(),
        );

        if level == TreeDumpLevel::Normal {
            return normal;
        }

        format!(
            "{normal} props={:?} style={:?} decoration={:?} local_runtime={:?} layout_meta={:?} paint_meta={:?} kind_full={} runtime_slots_full=[{}] cache=(paint_fragment_cached:{}) retained_runtime_present={} paint_dirty_boundaries={:?} paint_dirty_order={:?} composite_dirty_nodes={:?}",
            node.props,
            node.style,
            node.decoration,
            node.local_runtime,
            node.layout_meta,
            node.paint_meta,
            full_kind_summary(&node.kind),
            runtime_slots_full(&node.runtime_slots),
            node.paint_meta
                .boundary
                .is_some_and(|_| self.paint_cache.borrow().contains(paint_boundary)),
            self.retained_runtime.contains(node.id.as_ref()),
            repaint_boundary_list(self.paint_dirty.boundaries.iter().copied()),
            repaint_boundary_list(self.paint_dirty.paint_order.iter().copied()),
            self.paint_dirty.composite,
        )
    }

    fn snapshot_dirty_flags(&self, node: NodeId) -> DirtyFlags {
        let mut flags = DirtyFlags::NONE;
        if self.dirty.structure.contains(&node) {
            flags |= DirtyFlags::STRUCTURE;
        }
        if self.dirty.layout.contains(&node) {
            flags |= DirtyFlags::LAYOUT;
        }
        if self.dirty.text_layout.contains(&node) {
            flags |= DirtyFlags::TEXT_LAYOUT;
        }
        if self.dirty.paint.contains(&node) {
            flags |= DirtyFlags::PAINT;
        }
        if self.dirty.hit.contains(&node) {
            flags |= DirtyFlags::HIT;
        }
        if self.dirty.paint_order.contains(&node) {
            flags |= DirtyFlags::PAINT_ORDER;
        }
        if self.dirty.composite.contains(&node) {
            flags |= DirtyFlags::COMPOSITE;
        }
        flags
    }
}

fn collect_snapshot_order(
    tree: &Tree,
    node: NodeId,
    depth: usize,
    visited: &mut HashSet<NodeId>,
    ordered: &mut Vec<(usize, NodeId)>,
) {
    if !visited.insert(node) {
        return;
    }
    ordered.push((depth, node));
    let Some(tree_node) = tree.get(node) else {
        return;
    };
    for child in tree_node.children.iter().copied() {
        collect_snapshot_order(tree, child, depth + 1, visited, ordered);
    }
}

fn normal_kind_summary(kind: &NodeKind) -> String {
    match kind {
        NodeKind::Container => "Container".to_string(),
        NodeKind::Leaf(LeafKind::Text { content, .. }) => {
            format!(
                "Leaf::Text(bytes:{} chars:{})",
                content.len(),
                content.chars().count()
            )
        }
        NodeKind::Leaf(leaf) => format!("Leaf::{leaf:?}"),
    }
}

fn full_kind_summary(kind: &NodeKind) -> String {
    match kind {
        NodeKind::Container => "Container".to_string(),
        NodeKind::Leaf(LeafKind::Text {
            content,
            style,
            layout,
        }) => {
            format!("Leaf::Text {{ content: {content:?}, style: {style:?}, layout: {layout:?} }}")
        }
        NodeKind::Leaf(leaf) => format!("Leaf::{leaf:?}"),
    }
}

fn runtime_slots_full(slots: &RuntimeSlots) -> String {
    slots
        .debug_entries()
        .into_iter()
        .map(|entry| {
            format!(
                "{} policy={:?} value={}",
                entry.type_name, entry.policy, entry.value
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
