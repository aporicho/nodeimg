use super::{paint_to_target, PaintCx, PaintTraversal};
use crate::animation::AnimationStore;
use crate::geometry::{Point, Rect};
use crate::paint::{
    DisplayList, PaintBuildError, RecordingPaintTarget, ResolvedClip, ResolvedPaintCommand,
};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, LeafKind};
use crate::tree::node::{NodeId, NodeKind, NodeLocalRuntime, TreeNode};
use crate::tree::{NodeProps, RuntimeSlots, Tree};
use std::borrow::Cow;

const EPS: f32 = 1e-5;

pub(super) fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

pub(super) fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}

pub(super) fn assert_point_near(actual: Point, expected: Point) {
    assert!(
        (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
        "actual={actual:?}, expected={expected:?}"
    );
}

pub(super) fn assert_rect_near(actual: Rect, expected: Rect) {
    assert!(
        (actual.x - expected.x).abs() < EPS
            && (actual.y - expected.y).abs() < EPS
            && (actual.w - expected.w).abs() < EPS
            && (actual.h - expected.h).abs() < EPS,
        "actual={actual:?}, expected={expected:?}"
    );
}

pub(super) fn leaf_node(id: &'static str, kind: LeafKind, rect: Rect) -> TreeNode {
    TreeNode {
        id: Cow::Borrowed(id).into(),
        props: NodeProps::default(),
        style: BoxStyle::default(),
        decoration: None,
        kind: NodeKind::Leaf(kind),
        rect,
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: Default::default(),
        paint_meta: Default::default(),
        mutation_meta: Default::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

pub(super) fn container_node(id: &'static str, rect: Rect, children: Vec<NodeId>) -> TreeNode {
    TreeNode {
        id: Cow::Borrowed(id).into(),
        props: NodeProps::default(),
        style: BoxStyle::default(),
        decoration: None,
        kind: NodeKind::Container,
        rect,
        children,
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: Default::default(),
        paint_meta: Default::default(),
        mutation_meta: Default::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

pub(super) fn build_display_list_for_test(
    tree: &Tree,
    root: NodeId,
    theme: &Theme,
) -> Result<DisplayList, PaintBuildError> {
    let mut target = RecordingPaintTarget::new();
    let mut traversal = PaintTraversal::Full;
    paint_to_target(
        tree,
        root,
        &mut target,
        None,
        None,
        None,
        theme,
        &mut traversal,
    );
    target.display_list()
}

pub(super) fn paint_tree(tree: &Tree, root: NodeId) -> DisplayList {
    build_display_list_for_test(tree, root, &Theme::default()).unwrap()
}

pub(super) fn paint_tree_with_animations(
    tree: &Tree,
    root: NodeId,
    animations: &AnimationStore,
) -> DisplayList {
    let mut target = RecordingPaintTarget::new();
    let mut traversal = PaintTraversal::Full;
    paint_to_target(
        tree,
        root,
        &mut target,
        None,
        None,
        Some(animations),
        &Theme::default(),
        &mut traversal,
    );
    target.display_list().unwrap()
}

pub(super) fn paint_single_leaf(kind: LeafKind, rect: Rect) -> DisplayList {
    let mut tree = Tree::new();
    let root = tree.insert(leaf_node("leaf", kind, rect));
    tree.set_root(root);
    paint_tree(&tree, root)
}

pub(super) fn paint_cx<'a>(theme: &'a Theme) -> PaintCx<'a> {
    PaintCx {
        interaction: None,
        text_boxes: None,
        animations: None,
        theme,
    }
}

pub(super) fn only_command(list: &DisplayList) -> &ResolvedPaintCommand {
    assert_eq!(list.commands.len(), 1);
    &list.commands[0]
}

pub(super) fn only_clip(list: &DisplayList) -> &ResolvedClip {
    assert_eq!(list.clips.len(), 1);
    &list.clips[0]
}

pub(super) fn first_path(list: &DisplayList) -> &crate::paint::PathPaint {
    list.commands
        .iter()
        .find_map(|command| match &command.command {
            crate::paint::PaintCommand::Path(path) => Some(path),
            _ => None,
        })
        .expect("expected path command")
}
