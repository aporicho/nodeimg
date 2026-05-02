use std::time::Instant;

use super::arena::GestureArena;
use super::resize::ResizeRecognizer;
use super::{DragRecognizer, Gesture, GestureRecognizer, LongPressRecognizer, TapRecognizer};
use crate::tree::{HitChain, ResizeHit, Tree};

/// 根据命中链自动创建手势竞技场。
///
/// 规则：
/// - 容器级 draggable/resizable 会注册对应识别器
/// - 显式 gestures 保持兼容，供低层控件直接声明手势
/// - 所有识别器都绑定声明能力的节点本身，业务 owner 由事件层解析
pub(crate) fn arena_from_hit_chain(
    tree: &Tree,
    chain: &HitChain,
    x: f32,
    y: f32,
    last_tap_time: Option<Instant>,
) -> Option<GestureArena> {
    let target_id = default_target_id(tree, chain)?;
    let mut arena = GestureArena::new(target_id);
    let first_control_index = first_control_index(tree, chain);

    for (index, node_id) in chain.iter().enumerate() {
        let Some(node) = tree.get(node_id) else {
            continue;
        };
        let target_id = node.id.to_string();
        let gestures = &node.style.gestures;
        let allow_semantic_drag = allows_semantic_drag(index, first_control_index);
        tracing::debug!(
            target: "gui::gesture",
            node_id = %node.id,
            index,
            draggable = node.style.draggable,
            resizable = node.style.resizable,
            allow_semantic_drag,
            gestures = ?gestures,
            "inspect hit-chain node for gestures"
        );

        if gestures.contains(&Gesture::Tap) || gestures.contains(&Gesture::DoubleTap) {
            let mut rec = TapRecognizer::new(target_id.clone(), last_tap_time);
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }

        if gestures.contains(&Gesture::Drag) || (node.style.draggable && allow_semantic_drag) {
            let mut rec = DragRecognizer::new(target_id.clone());
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }

        if gestures.contains(&Gesture::LongPress) {
            let mut rec = LongPressRecognizer::new(target_id.clone());
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }
    }

    if arena.is_empty() {
        None
    } else {
        Some(arena)
    }
}

pub(crate) fn arena_from_resize_hit(
    tree: &Tree,
    hit: ResizeHit,
    x: f32,
    y: f32,
) -> Option<GestureArena> {
    let node = tree.get(hit.node_id)?;
    let target_id = node.id.to_string();
    let mut arena = GestureArena::new(target_id.clone());
    let mut rec = ResizeRecognizer::new(target_id.clone(), hit.edge);
    if rec.on_pointer_down(x, y) {
        tracing::debug!(
            target: "gui::gesture",
            node_id = %node.id,
            resize_edge = ?hit.edge,
            "add resize recognizer from interaction hit"
        );
        arena.add(Box::new(rec));
    }

    if arena.is_empty() {
        None
    } else {
        Some(arena)
    }
}

fn default_target_id(tree: &Tree, chain: &HitChain) -> Option<String> {
    chain
        .leaf()
        .and_then(|id| tree.get(id))
        .map(|node| node.id.to_string())
}

fn first_control_index(tree: &Tree, chain: &HitChain) -> Option<usize> {
    chain.iter().enumerate().find_map(|(index, node_id)| {
        tree.get(node_id)
            .is_some_and(|node| node.props.semantic_role.is_some())
            .then_some(index)
    })
}

fn allows_semantic_drag(index: usize, first_control_index: Option<usize>) -> bool {
    first_control_index
        .map(|control_index| index <= control_index)
        .unwrap_or(true)
}
