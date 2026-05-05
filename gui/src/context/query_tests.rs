use super::*;
use crate::cursor::CursorKind;
use crate::geometry::ResizeEdge;
use crate::renderer::Rect;
use crate::tree::layout::BoxStyle;
use crate::tree::layout::Gesture;
use crate::tree::SemanticRole;
use crate::tree::{TreeNode, TreeNodeBuilder};

fn hittable_node(id: &'static str, rect: Rect) -> TreeNode {
    TreeNodeBuilder::container(
        id,
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
    )
    .rect(rect)
    .build()
}

fn node_with_style(id: &'static str, rect: Rect, style: BoxStyle) -> TreeNode {
    TreeNodeBuilder::container(id, style).rect(rect).build()
}

fn node_with_role(id: &'static str, rect: Rect, role: SemanticRole) -> TreeNode {
    TreeNodeBuilder::container(
        id,
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
    )
    .rect(rect)
    .semantic_role(role)
    .build()
}

#[test]
fn pointer_hit_snapshot_captures_point_and_chain() {
    let mut ctx = Context::new();
    let root = ctx.tree.insert(hittable_node(
        "root",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    ctx.tree.set_root(root);

    let hit = ctx.query().pointer_hit_at(20.0, 30.0);

    assert!(hit.matches_point(20.0, 30.0));
    assert_eq!(hit.chain().leaf(), Some(root));
    assert!(hit.resize_hit().is_none());
}

#[test]
fn cursor_for_hit_prefers_resize_hit() {
    let mut ctx = Context::new();
    let root = ctx.tree.insert(node_with_style(
        "resizable",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        BoxStyle {
            hittable: true,
            resizable: true,
            ..Default::default()
        },
    ));
    ctx.tree.set_root(root);

    let hit = ctx.query().pointer_hit_at(100.0, 50.0);

    assert_eq!(
        ctx.query().cursor_for_hit(&hit),
        CursorKind::Resize(ResizeEdge::Right)
    );
}

#[test]
fn cursor_for_hit_reports_draggable_as_move() {
    let mut ctx = Context::new();
    let root = ctx.tree.insert(node_with_style(
        "titlebar",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 32.0,
        },
        BoxStyle {
            hittable: true,
            draggable: true,
            gestures: vec![Gesture::Tap, Gesture::Drag],
            ..Default::default()
        },
    ));
    ctx.tree.set_root(root);

    let hit = ctx.query().pointer_hit_at(20.0, 10.0);

    assert_eq!(ctx.query().cursor_for_hit(&hit), CursorKind::Move);
    assert_eq!(ctx.query().cursor_at(20.0, 10.0), CursorKind::Move);
}

#[test]
fn cursor_for_hit_reports_clickable_as_pointer() {
    let mut ctx = Context::new();
    let root = ctx.tree.insert(node_with_role(
        "button",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 80.0,
            h: 28.0,
        },
        SemanticRole::Button,
    ));
    ctx.tree.set_root(root);

    let hit = ctx.query().pointer_hit_at(20.0, 10.0);

    assert_eq!(ctx.query().cursor_for_hit(&hit), CursorKind::Pointer);
}

#[test]
fn cursor_for_hit_reports_text_roles_as_text() {
    for role in [
        SemanticRole::TextInput,
        SemanticRole::TextArea,
        SemanticRole::NumberInput,
    ] {
        let mut ctx = Context::new();
        let root = ctx.tree.insert(node_with_role(
            "field",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 28.0,
            },
            role,
        ));
        ctx.tree.set_root(root);

        let hit = ctx.query().pointer_hit_at(20.0, 10.0);

        assert_eq!(ctx.query().cursor_for_hit(&hit), CursorKind::Text);
    }
}

#[test]
fn cursor_for_hit_defaults_for_plain_hittable_node() {
    let mut ctx = Context::new();
    let root = ctx.tree.insert(hittable_node(
        "plain",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    ctx.tree.set_root(root);

    let hit = ctx.query().pointer_hit_at(20.0, 30.0);

    assert_eq!(ctx.query().cursor_for_hit(&hit), CursorKind::Default);
}
