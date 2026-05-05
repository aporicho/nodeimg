use std::borrow::Cow;

use crate::gesture::Gesture;
use crate::renderer::Rect;
use crate::tree::layout::BoxStyle;
use crate::tree::{
    HitChain, SemanticRole, TargetChain, TargetDescriptor, TargetOwnerResolver, Tree,
    TreeNodeBuilder,
};

fn rect() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 40.0,
    }
}

fn node(id: &'static str, style: BoxStyle) -> crate::tree::TreeNode {
    TreeNodeBuilder::container(id, style).rect(rect()).build()
}

fn role_node(id: &'static str, role: SemanticRole) -> crate::tree::TreeNode {
    TreeNodeBuilder::container(
        id,
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
    )
    .rect(rect())
    .semantic_role(role)
    .build()
}

#[test]
fn descriptor_resolves_semantic_root_from_stable_id_prefix() {
    let mut tree = Tree::new();
    tree.insert(role_node("button", SemanticRole::Button));
    let label = tree.insert(node("button::label", BoxStyle::default()));

    let target = TargetDescriptor::from_node(&tree, label).expect("target");

    assert_eq!(target.own_role(), None);
    assert_eq!(target.root_role(), Some(SemanticRole::Button));
    assert!(target.uses_pointer_cursor());
}

#[test]
fn target_chain_skips_disabled_and_prefers_focusable_over_gesture_target() {
    let mut tree = Tree::new();
    let gesture = tree.insert(node(
        "gesture",
        BoxStyle {
            gestures: vec![Gesture::Tap],
            ..Default::default()
        },
    ));
    let mut disabled_button = role_node("disabled", SemanticRole::Button);
    disabled_button.props.enabled = false;
    let disabled = tree.insert(disabled_button);
    let enabled_button = tree.insert(role_node("enabled", SemanticRole::Button));
    let chain = HitChain::new(vec![gesture, disabled, enabled_button]);

    let targets = TargetChain::from_hit_chain(&tree, &chain);

    assert_eq!(targets.input_target(), Some(enabled_button));
    assert_eq!(targets.focus_target(), Some(enabled_button));
}

#[test]
fn target_chain_falls_back_to_first_enabled_gesture_target() {
    let mut tree = Tree::new();
    let gesture = tree.insert(node(
        "gesture",
        BoxStyle {
            gestures: vec![Gesture::Tap],
            ..Default::default()
        },
    ));
    let plain = tree.insert(node("plain", BoxStyle::default()));
    let chain = HitChain::new(vec![plain, gesture]);

    let targets = TargetChain::from_hit_chain(&tree, &chain);

    assert_eq!(targets.input_target(), Some(gesture));
    assert_eq!(targets.focus_target(), Some(gesture));
}

#[test]
fn target_chain_keeps_semantic_drag_guard_rule() {
    let mut tree = Tree::new();
    let leaf = tree.insert(node("button::label", BoxStyle::default()));
    let button = tree.insert(role_node("button", SemanticRole::Button));
    let parent = tree.insert(node(
        "panel",
        BoxStyle {
            draggable: true,
            ..Default::default()
        },
    ));
    let chain = HitChain::new(vec![leaf, button, parent]);

    let targets = TargetChain::from_hit_chain(&tree, &chain);

    assert!(targets.allows_semantic_drag(0));
    assert!(targets.allows_semantic_drag(1));
    assert!(!targets.allows_semantic_drag(2));
}

#[test]
fn owner_resolver_prefers_explicit_owner_then_semantic_prefix_then_retained_target() {
    let mut tree = Tree::new();
    let mut explicit = node("button::value", BoxStyle::default());
    explicit.props.owner_id = Some(Cow::Borrowed("explicit_owner"));
    tree.insert(explicit);
    tree.insert(role_node("button", SemanticRole::Button));
    tree.insert(node(
        "fallback",
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
    ));
    let resolver = TargetOwnerResolver::new(&tree);

    assert_eq!(resolver.owner_id("button::value"), "explicit_owner");
    assert_eq!(resolver.owner_id("button::label"), "button");
    assert_eq!(resolver.owner_id("fallback"), "fallback");
    assert_eq!(resolver.owner_id("unknown"), "unknown");
}
