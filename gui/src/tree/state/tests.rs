use super::Tree;
use crate::renderer::Rect;
use crate::tree::layout::{
    BoxStyle, LayoutConstraints, LayoutOutput, LeafKind, RelayoutBoundaryReason, Size, TextLayout,
};
use crate::tree::{
    DirtyFlags, DirtyQueues, RepaintBoundaryId, RepaintBoundaryReason, RuntimeRetention,
    RuntimeSlot, RuntimeSlotPolicy, StableId, StylePatch, TreeDumpLevel, TreeIndexError,
    TreeMutation, TreeNode, TreeNodeBuilder, TreeSnapshotOptions,
};

#[derive(Debug, Default)]
struct TestRuntime {
    value: usize,
}

impl RuntimeSlot for TestRuntime {}

#[derive(Debug, Default)]
struct RetainedTestRuntime {
    value: usize,
}

impl RuntimeSlot for RetainedTestRuntime {
    fn default_policy() -> RuntimeSlotPolicy {
        RuntimeSlotPolicy {
            retention: RuntimeRetention::KeepWhileStableNodeExists,
            ..RuntimeSlotPolicy::default()
        }
    }
}

fn container_node(id: &'static str) -> TreeNode {
    TreeNodeBuilder::container(id, BoxStyle::default()).build()
}

fn text_node(id: &'static str, content: &'static str) -> TreeNode {
    TreeNodeBuilder::leaf(
        id,
        LeafKind::Text {
            content: content.to_string(),
            style: crate::renderer::TextStyle::new(crate::renderer::Color::BLACK, 12.0),
            layout: TextLayout::default(),
        },
        BoxStyle::default(),
    )
    .rect(Rect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    })
    .build()
}

fn layout_output(rect: Rect) -> LayoutOutput {
    LayoutOutput {
        rect,
        content_rect: rect,
        intrinsic_width: rect.w,
        intrinsic_height: rect.h,
        baseline: None,
    }
}

#[test]
fn tree_dump_normal_uses_compact_fields() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    let text = tree.insert(text_node("text", "secret text"));
    tree.set_root(root);
    tree.set_children(root, vec![text]);

    let snapshot = tree.debug_snapshot(TreeSnapshotOptions::normal(500));

    assert_eq!(snapshot.summary.level, TreeDumpLevel::Normal);
    assert!(snapshot
        .nodes
        .iter()
        .any(|node| node.line.contains("Leaf::Text(bytes:11")));
    assert!(!snapshot
        .nodes
        .iter()
        .any(|node| node.line.contains("secret text")));
}

#[test]
fn tree_dump_full_prints_all_node_fields_and_raw_text() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    let text = tree.insert(text_node("text", "secret text"));
    tree.set_root(root);
    tree.set_children(root, vec![text]);

    let snapshot = tree.debug_snapshot(TreeSnapshotOptions::full());
    let text_line = snapshot
        .nodes
        .iter()
        .find(|node| node.stable_id == "text")
        .expect("text node")
        .line
        .as_str();

    assert!(text_line.contains("props="));
    assert!(text_line.contains("style="));
    assert!(text_line.contains("layout_meta="));
    assert!(text_line.contains("paint_meta="));
    assert!(text_line.contains("secret text"));
}

#[test]
fn tree_dump_full_prints_runtime_slot_debug_values() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    tree.set_root(root);
    tree.ensure_runtime_slot::<TestRuntime>(root)
        .expect("runtime slot")
        .value = 7;

    let snapshot = tree.debug_snapshot(TreeSnapshotOptions::full());
    let root_line = &snapshot.nodes[0].line;

    assert!(root_line.contains("TestRuntime"));
    assert!(root_line.contains("value: 7"));
}

#[test]
fn tree_dump_detects_parent_mismatch_duplicate_children_and_stale_index() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    let child = tree.insert(container_node("child"));
    tree.set_root(root);
    tree.get_mut(root).expect("root").children = vec![child, child];

    let snapshot = tree.debug_snapshot(TreeSnapshotOptions::full());
    let issues = snapshot
        .issues
        .iter()
        .map(|issue| issue.message.as_str())
        .collect::<Vec<_>>();

    assert!(issues.iter().any(|issue| issue.contains("duplicate child")));
    assert!(issues
        .iter()
        .any(|issue| issue.contains("parent map mismatch")));
}

#[test]
fn tree_dump_respects_max_nodes_in_normal_mode_but_not_full_mode() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    let first = tree.insert(container_node("first"));
    let second = tree.insert(container_node("second"));
    tree.set_root(root);
    tree.set_children(root, vec![first, second]);

    let normal = tree.debug_snapshot(TreeSnapshotOptions::normal(1));
    let full = tree.debug_snapshot(TreeSnapshotOptions {
        level: TreeDumpLevel::Full,
        max_nodes: crate::tree::TreeSnapshotMaxNodes::Limit(1),
    });

    assert!(normal.summary.truncated);
    assert_eq!(normal.nodes.len(), 1);
    assert!(!full.summary.truncated);
    assert_eq!(full.nodes.len(), 3);
}

#[test]
fn tree_dump_does_not_mutate_frame_stats() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    tree.set_root(root);
    let before = tree.frame_stats_snapshot();

    let _ = tree.debug_snapshot(TreeSnapshotOptions::full());
    let after = tree.frame_stats_snapshot();

    assert_eq!(before, after);
}

#[test]
fn runtime_slot_roundtrips_by_type() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    tree.set_root(root);

    tree.ensure_runtime_slot::<TestRuntime>(root)
        .expect("slot")
        .value = 42;

    assert_eq!(
        tree.runtime_slot::<TestRuntime>(root)
            .expect("stored runtime")
            .value,
        42
    );

    let removed = tree
        .remove_runtime_slot::<TestRuntime>(root)
        .expect("removed runtime");
    assert_eq!(removed.value, 42);
    assert!(tree.runtime_slot::<TestRuntime>(root).is_none());
}

#[test]
fn declarative_runtime_slot_overrides_retained_slot_on_insert() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    tree.set_root(root);
    tree.ensure_runtime_slot::<RetainedTestRuntime>(root)
        .expect("slot")
        .value = 1;
    tree.remove(root);

    let mut replacement = container_node("root");
    replacement
        .runtime_slots
        .ensure::<RetainedTestRuntime>()
        .value = 2;
    let replacement = tree.insert(replacement);

    assert_eq!(
        tree.runtime_slot::<RetainedTestRuntime>(replacement)
            .expect("runtime slot")
            .value,
        2
    );
}

#[test]
fn drop_when_node_missing_runtime_slot_is_not_retained() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    tree.set_root(root);
    tree.ensure_runtime_slot::<TestRuntime>(root)
        .expect("slot")
        .value = 1;

    tree.remove(root);
    let replacement = tree.insert(container_node("root"));

    assert!(tree.runtime_slot::<TestRuntime>(replacement).is_none());
}

#[test]
fn stable_id_lookup_uses_tree_node_identity() {
    let mut tree = Tree::new();
    let root = tree.insert(container_node("root"));
    tree.set_root(root);

    assert_eq!(tree.node_by_stable_id(&StableId::from("root")), tree.root());
    assert_eq!(tree.node_by_str("root"), tree.root());
}

#[test]
fn tree_index_registers_and_removes_live_nodes() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("insert");
    tree.set_root(root);

    assert_eq!(tree.node_by_str("root"), Some(root));
    assert!(tree.contains_stable_id("root"));
    tree.validate_index().expect("valid index");

    tree.remove(root);

    assert_eq!(tree.node_by_str("root"), None);
    tree.validate_index().expect("valid after remove");
}

#[test]
fn tree_index_rejects_duplicate_stable_ids() {
    let mut tree = Tree::new();
    tree.insert_checked(container_node("duplicate"))
        .expect("first insert");

    let err = tree
        .insert_checked(container_node("duplicate"))
        .expect_err("duplicate should fail checked insertion");

    assert!(matches!(
        err,
        TreeIndexError::DuplicateStableId {
            stable_id,
            existing: _,
            duplicate: _
        } if stable_id.as_ref() == "duplicate"
    ));
}

#[test]
fn tree_index_clears_before_node_id_reuse() {
    let mut tree = Tree::new();
    let first = tree.insert_checked(container_node("first")).expect("first");
    tree.remove(first);

    let second = tree
        .insert_checked(container_node("second"))
        .expect("second");

    assert_eq!(first, second);
    assert_eq!(tree.node_by_str("first"), None);
    assert_eq!(tree.node_by_str("second"), Some(second));
    tree.validate_index().expect("valid reused index");
}

#[test]
fn runtime_slot_lookup_uses_tree_index() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    tree.ensure_runtime_slot::<TestRuntime>(root)
        .expect("slot")
        .value = 9;
    tree.clear_frame_stats();

    let value = tree
        .runtime_slot_by_stable_id::<TestRuntime>("root")
        .expect("runtime")
        .value;
    let stats = tree.frame_stats_snapshot();

    assert_eq!(value, 9);
    assert_eq!(stats.stable_id_lookups, 1);
    assert_eq!(stats.parent_lookup_fallback_scans, 0);
}

#[test]
fn dirty_take_clears_queues() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);

    tree.mark_dirty(root, DirtyFlags::LAYOUT | DirtyFlags::PAINT);
    let dirty = tree.take_dirty();

    assert!(dirty.layout.contains(&root));
    assert!(dirty.paint.contains(&root));
    assert!(tree.take_dirty().layout.is_empty());
    assert!(tree.take_dirty().paint.is_empty());
}

#[test]
fn layout_dirty_bubbles_to_nearest_relayout_boundary() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let mut boundary_node = container_node("boundary");
    boundary_node
        .layout_meta
        .set_boundary(crate::tree::layout::RelayoutBoundaryReason::Panel);
    let boundary = tree.insert_checked(boundary_node).expect("boundary");
    let child = tree.insert_checked(container_node("child")).expect("child");
    tree.append_child(root, boundary);
    tree.append_child(boundary, child);

    tree.mark_layout_dirty(child, crate::tree::layout::LayoutDirtyReason::Size);
    let dirty = tree.take_layout_dirty();

    assert!(dirty.boundaries.contains(&boundary));
    assert!(!dirty.boundaries.contains(&root));
}

#[test]
fn set_rect_move_invalidates_cached_boundary_layout_input() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let mut panel_node = container_node("panel");
    panel_node
        .layout_meta
        .set_boundary(RelayoutBoundaryReason::Panel);
    panel_node.mutation_meta.rect_move =
        crate::tree::RectMoveInvalidation::LayoutAndBoundaryPlacement;
    let panel = tree.insert_checked(panel_node).expect("panel");
    let child = tree.insert_checked(container_node("child")).expect("child");
    tree.append_child(root, panel);
    tree.append_child(panel, child);
    let constraints = LayoutConstraints::from_available(Rect {
        x: 0.0,
        y: 0.0,
        w: 180.0,
        h: 120.0,
    });
    let input = tree
        .layout_input_for(panel, constraints)
        .expect("layout input");
    tree.layout_cache_set(
        input,
        layout_output(Rect {
            x: 0.0,
            y: 0.0,
            w: 180.0,
            h: 120.0,
        }),
    );
    let before_dependency = tree
        .get(panel)
        .expect("panel")
        .layout_meta
        .layout_dependency_revision;

    tree.apply_mutation(
        &crate::template::TemplateRegistry::new(),
        TreeMutation::SetRect {
            node: panel,
            rect: Rect {
                x: 24.0,
                y: 18.0,
                w: 0.0,
                h: 0.0,
            },
        },
    )
    .expect("move panel");

    let after_dependency = tree
        .get(panel)
        .expect("panel")
        .layout_meta
        .layout_dependency_revision;
    let moved_input = tree
        .layout_input_for(panel, constraints)
        .expect("moved layout input");
    assert!(after_dependency > before_dependency);
    assert!(tree.layout_cache_get(input).is_none());
    assert!(tree.layout_cache_get(moved_input).is_none());
}

#[test]
fn child_style_change_invalidates_cached_ancestor_boundary_layout_input() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let mut boundary_node = container_node("boundary");
    boundary_node
        .layout_meta
        .set_boundary(RelayoutBoundaryReason::Panel);
    let boundary = tree.insert_checked(boundary_node).expect("boundary");
    let child = tree.insert_checked(container_node("child")).expect("child");
    tree.append_child(root, boundary);
    tree.append_child(boundary, child);
    let constraints = LayoutConstraints::from_available(Rect {
        x: 0.0,
        y: 0.0,
        w: 240.0,
        h: 160.0,
    });
    let input = tree
        .layout_input_for(boundary, constraints)
        .expect("layout input");
    tree.layout_cache_set(
        input,
        layout_output(Rect {
            x: 0.0,
            y: 0.0,
            w: 240.0,
            h: 160.0,
        }),
    );
    let before_dependency = tree
        .get(boundary)
        .expect("boundary")
        .layout_meta
        .layout_dependency_revision;

    tree.apply_mutation(
        &crate::template::TemplateRegistry::new(),
        TreeMutation::SetStyle {
            node: child,
            patch: StylePatch {
                width: Some(Size::Fixed(80.0)),
                ..StylePatch::default()
            },
        },
    )
    .expect("style child");

    let after_dependency = tree
        .get(boundary)
        .expect("boundary")
        .layout_meta
        .layout_dependency_revision;
    assert!(after_dependency > before_dependency);
    assert!(tree.layout_cache_get(input).is_none());
}

#[test]
fn text_layout_dirty_records_text_node_and_boundary() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let text = tree
        .insert_checked(text_node("text", "value"))
        .expect("text");
    tree.append_child(root, text);

    tree.mark_dirty(text, DirtyFlags::TEXT_LAYOUT);
    let dirty = tree.take_layout_dirty();

    assert!(dirty.text_nodes.contains(&text));
    assert!(dirty.boundaries.contains(&root));
}

#[test]
fn paint_dirty_bubbles_to_nearest_repaint_boundary() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let boundary = tree
        .insert_checked(container_node("node_card"))
        .expect("boundary");
    let child = tree.insert_checked(container_node("child")).expect("child");
    tree.set_repaint_boundary(boundary, RepaintBoundaryReason::CanvasNodeCard);
    tree.append_child(root, boundary);
    tree.append_child(boundary, child);

    tree.mark_dirty(child, DirtyFlags::PAINT);
    let dirty = tree.take_paint_dirty();

    assert!(dirty.boundaries.contains(&RepaintBoundaryId(boundary)));
    assert!(!dirty.boundaries.contains(&RepaintBoundaryId(root)));
}

#[test]
fn composite_dirty_does_not_mark_repaint_boundary() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);

    tree.mark_dirty(root, DirtyFlags::COMPOSITE | DirtyFlags::HIT);
    let dirty = tree.take_paint_dirty();

    assert!(dirty.boundaries.is_empty());
    assert!(dirty.composite.contains(&root));
}

#[test]
fn tree_mutation_set_text_marks_text_layout_and_paint() {
    let mut tree = Tree::new();
    let text = tree.insert_checked(text_node("text", "old")).expect("text");
    tree.set_root(text);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetText {
                node: text,
                value: "new".to_string(),
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::TEXT_LAYOUT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    let dirty = tree.take_dirty();
    assert!(dirty.text_layout.contains(&text));
    assert!(dirty.paint.contains(&text));
}

#[test]
fn tree_mutation_noop_set_text_does_not_dirty_or_bump_revision() {
    let mut tree = Tree::new();
    let text = tree.insert_checked(text_node("text", "old")).expect("text");
    tree.set_root(text);
    let registry = crate::template::TemplateRegistry::new();
    let before_layout = tree.get(text).expect("text").layout_meta.text_revision;
    let before_paint = tree.get(text).expect("text").paint_meta.text_paint_revision;

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetText {
                node: text,
                value: "old".to_string(),
            },
        )
        .expect("mutation");

    assert_eq!(invalidation.flags, DirtyFlags::NONE);
    assert_eq!(
        tree.get(text).expect("text").layout_meta.text_revision,
        before_layout
    );
    assert_eq!(
        tree.get(text).expect("text").paint_meta.text_paint_revision,
        before_paint
    );
    assert_eq!(tree.take_dirty(), DirtyQueues::default());
}

#[test]
fn tree_mutation_set_rect_position_marks_layout_by_default() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: root,
                rect: Rect {
                    x: 10.0,
                    y: 10.0,
                    w: 0.0,
                    h: 0.0,
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    assert!(tree.take_dirty().layout.contains(&root));
}

#[test]
fn tree_mutation_set_rect_grid_move_marks_paint_not_layout() {
    let mut tree = Tree::new();
    let mut root_node = container_node("grid");
    root_node.mutation_meta.rect_move = crate::tree::RectMoveInvalidation::Repaint;
    let root = tree.insert_checked(root_node).expect("grid");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: root,
                rect: Rect {
                    x: 10.0,
                    y: 10.0,
                    w: 0.0,
                    h: 0.0,
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(!invalidation.flags.contains(DirtyFlags::LAYOUT));
    assert!(tree.take_dirty().paint.contains(&root));
}

#[test]
fn tree_mutation_set_rect_boundary_move_marks_placement_not_layout() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let mut child_node = container_node("child");
    child_node
        .paint_meta
        .set_boundary(RepaintBoundaryReason::CanvasNodeCard);
    child_node.mutation_meta.rect_move = crate::tree::RectMoveInvalidation::BoundaryPlacement;
    let child = tree.insert_checked(child_node).expect("child");
    tree.append_child(root, child);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: child,
                rect: Rect {
                    x: 10.0,
                    y: 10.0,
                    w: 0.0,
                    h: 0.0,
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::PAINT_PLACEMENT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(!invalidation.flags.contains(DirtyFlags::LAYOUT));
    let dirty = tree.take_paint_dirty();
    assert!(dirty.placement.contains(&RepaintBoundaryId(root)));
}

#[test]
fn tree_mutation_set_rect_layout_boundary_move_marks_layout_and_placement() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let mut panel_node = container_node("panel");
    panel_node
        .layout_meta
        .set_boundary(RelayoutBoundaryReason::Panel);
    panel_node
        .paint_meta
        .set_boundary(RepaintBoundaryReason::PanelFrame);
    panel_node.mutation_meta.rect_move =
        crate::tree::RectMoveInvalidation::LayoutAndBoundaryPlacement;
    let panel = tree.insert_checked(panel_node).expect("panel");
    tree.append_child(root, panel);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: panel,
                rect: Rect {
                    x: 24.0,
                    y: 18.0,
                    w: 0.0,
                    h: 0.0,
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT_PLACEMENT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(tree.take_layout_dirty().boundaries.contains(&panel));
    let dirty = tree.take_paint_dirty();
    assert!(dirty.boundaries.contains(&RepaintBoundaryId(panel)));
    assert!(dirty.placement.contains(&RepaintBoundaryId(root)));
}

#[test]
fn tree_mutation_set_rect_layout_boundary_resize_move_marks_layout_and_placement() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let mut panel_node = container_node("panel");
    panel_node
        .layout_meta
        .set_boundary(RelayoutBoundaryReason::Panel);
    panel_node
        .paint_meta
        .set_boundary(RepaintBoundaryReason::PanelFrame);
    panel_node.mutation_meta.rect_move =
        crate::tree::RectMoveInvalidation::LayoutAndBoundaryPlacement;
    let panel = tree.insert_checked(panel_node).expect("panel");
    tree.append_child(root, panel);
    tree.get_mut(panel).expect("panel").rect = Rect {
        x: 100.0,
        y: 80.0,
        w: 240.0,
        h: 160.0,
    };
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: panel,
                rect: Rect {
                    x: 60.0,
                    y: 40.0,
                    w: 280.0,
                    h: 200.0,
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT_PLACEMENT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(tree.take_layout_dirty().boundaries.contains(&panel));
    let dirty = tree.take_paint_dirty();
    assert!(dirty.boundaries.contains(&RepaintBoundaryId(panel)));
    assert!(dirty.placement.contains(&RepaintBoundaryId(root)));
}

#[test]
fn tree_mutation_set_rect_boundary_resize_move_marks_placement() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let mut child_node = container_node("child");
    child_node
        .paint_meta
        .set_boundary(RepaintBoundaryReason::CanvasNodeCard);
    child_node.mutation_meta.rect_move = crate::tree::RectMoveInvalidation::BoundaryPlacement;
    let child = tree.insert_checked(child_node).expect("child");
    tree.append_child(root, child);
    tree.get_mut(child).expect("child").rect = Rect {
        x: 100.0,
        y: 80.0,
        w: 240.0,
        h: 160.0,
    };
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: child,
                rect: Rect {
                    x: 60.0,
                    y: 40.0,
                    w: 280.0,
                    h: 200.0,
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT_PLACEMENT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    let dirty = tree.take_paint_dirty();
    assert!(dirty.placement.contains(&RepaintBoundaryId(root)));
}

#[test]
fn tree_mutation_set_rect_size_marks_layout_hit_paint() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: root,
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 100.0,
                    h: 80.0,
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
}

#[test]
fn tree_mutation_noop_set_rect_does_not_dirty_or_bump_revision() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();
    let before = tree
        .get(root)
        .expect("root")
        .layout_meta
        .explicit_rect_revision;

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: root,
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                },
            },
        )
        .expect("mutation");

    assert_eq!(invalidation.flags, DirtyFlags::NONE);
    assert_eq!(
        tree.get(root)
            .expect("root")
            .layout_meta
            .explicit_rect_revision,
        before
    );
    assert_eq!(tree.take_dirty(), DirtyQueues::default());
}

#[test]
fn tree_mutation_set_rect_survives_layout_flush() {
    let mut tree = Tree::new();
    let mut root_node = container_node("root");
    root_node.style.width = Size::Fixed(400.0);
    root_node.style.height = Size::Fixed(300.0);
    let root = tree.insert_checked(root_node).expect("root");
    tree.set_root(root);
    let child = tree.insert_checked(container_node("child")).expect("child");
    tree.append_child(root, child);
    let registry = crate::template::TemplateRegistry::new();
    let explicit = Rect {
        x: 32.0,
        y: 48.0,
        w: 120.0,
        h: 64.0,
    };

    tree.apply_mutation(
        &registry,
        TreeMutation::SetRect {
            node: child,
            rect: explicit,
        },
    )
    .expect("set rect");
    crate::tree::layout(
        &mut tree,
        root,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 300.0,
        },
        &mut |_text, _style| (0.0, 0.0),
    );

    assert_eq!(tree.get(child).expect("child").rect, explicit);
}

#[test]
fn tree_mutation_set_z_index_marks_paint_order_hit_paint() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetZIndex {
                node: root,
                z_index: 8,
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::PAINT_ORDER));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
}

#[test]
fn tree_mutation_noop_set_z_index_does_not_dirty_or_bump_revision() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();
    let before_layout = tree.get(root).expect("root").layout_meta.style_revision;
    let before_order = tree
        .get(root)
        .expect("root")
        .paint_meta
        .paint_order_revision;

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetZIndex {
                node: root,
                z_index: 0,
            },
        )
        .expect("mutation");

    assert_eq!(invalidation.flags, DirtyFlags::NONE);
    assert_eq!(
        tree.get(root).expect("root").layout_meta.style_revision,
        before_layout
    );
    assert_eq!(
        tree.get(root)
            .expect("root")
            .paint_meta
            .paint_order_revision,
        before_order
    );
    assert_eq!(tree.take_dirty(), DirtyQueues::default());
}

#[test]
fn tree_mutation_empty_style_patch_is_noop() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetStyle {
                node: root,
                patch: StylePatch::default(),
            },
        )
        .expect("mutation");

    assert_eq!(invalidation.flags, DirtyFlags::NONE);
    assert_eq!(tree.take_dirty(), DirtyQueues::default());
}

#[test]
fn tree_mutation_set_transform_marks_paint_not_composite() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::new();

    let invalidation = tree
        .apply_mutation(
            &registry,
            TreeMutation::SetStyle {
                node: root,
                patch: StylePatch {
                    transform: Some(Some(crate::geometry::TransformSpec::translate_scale(
                        [10.0, 20.0],
                        2.0,
                    ))),
                    ..StylePatch::default()
                },
            },
        )
        .expect("mutation");

    assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    assert!(invalidation.flags.contains(DirtyFlags::HIT));
    assert!(!invalidation.flags.contains(DirtyFlags::COMPOSITE));
}

#[test]
fn tree_mutation_mount_unmount_marks_structure_layout_hit_paint() {
    let mut tree = Tree::new();
    let root = tree.insert_checked(container_node("root")).expect("root");
    tree.set_root(root);
    let registry = crate::template::TemplateRegistry::with_builtin_templates();

    let mount = tree
        .apply_mutation(
            &registry,
            TreeMutation::MountTemplate {
                parent: root,
                template: crate::template::TemplateId::from(crate::template::TEXT_BOX_TEMPLATE),
                instance: crate::template::InstanceId::from("textbox"),
                payload: crate::template::TemplatePayload::from(
                    crate::template::SlotValues::default(),
                ),
            },
        )
        .expect("mount");

    assert!(mount.flags.contains(DirtyFlags::STRUCTURE));
    assert!(mount.flags.contains(DirtyFlags::LAYOUT));
    assert!(mount.flags.contains(DirtyFlags::HIT));
    assert!(mount.flags.contains(DirtyFlags::PAINT));
    let mounted = tree.node_by_str("textbox").expect("mounted root");

    let unmount = tree
        .apply_mutation(&registry, TreeMutation::Unmount { node: mounted })
        .expect("unmount");

    assert!(unmount.flags.contains(DirtyFlags::STRUCTURE));
    assert!(unmount.flags.contains(DirtyFlags::LAYOUT));
    assert!(unmount.flags.contains(DirtyFlags::HIT));
    assert!(unmount.flags.contains(DirtyFlags::PAINT));
    assert!(tree.node_by_str("textbox").is_none());
}
