use std::time::Instant;

use super::arena::GestureArena;
use super::resize::ResizeRecognizer;
use super::{DragRecognizer, GestureRecognizer, LongPressRecognizer, TapRecognizer};
use crate::tree::layout::Gesture;
use crate::tree::{HitChain, ResizeHit, TargetChain, TargetDescriptor, Tree};

/// 根据命中链自动创建手势竞技场。
///
/// 规则：
/// - 容器级 draggable/resizable 会注册对应识别器
/// - 显式 gestures 是低层控件声明手势能力的标准入口
/// - 所有识别器都绑定声明能力的节点本身，业务 owner 由事件层解析
pub(crate) fn arena_from_hit_chain(
    tree: &Tree,
    chain: &HitChain,
    x: f32,
    y: f32,
    last_tap_time: Option<Instant>,
) -> Option<GestureArena> {
    let mut arena = GestureArena::new();
    let targets = TargetChain::from_hit_chain(tree, chain);

    for (index, target) in targets.iter().enumerate() {
        let target_id = target.stable_id().to_string();
        let allow_semantic_drag = targets.allows_semantic_drag(index);
        tracing::debug!(
            target: "gui::gesture",
            node_id = %target.stable_id(),
            index,
            draggable = target.is_draggable(),
            resizable = target.is_resizable(),
            allow_semantic_drag,
            "inspect hit-chain node for gestures"
        );

        if target.has_gesture(Gesture::Tap) || target.has_gesture(Gesture::DoubleTap) {
            let mut rec = TapRecognizer::new(target_id.clone(), last_tap_time);
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }

        if target.has_gesture(Gesture::Drag) || (target.is_draggable() && allow_semantic_drag) {
            let mut rec = DragRecognizer::new(target_id.clone());
            if rec.on_pointer_down(x, y) {
                arena.add(Box::new(rec));
            }
        }

        if target.has_gesture(Gesture::LongPress) {
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
    let target = TargetDescriptor::from_node(tree, hit.node_id)?;
    let target_id = target.stable_id().to_string();
    let mut arena = GestureArena::new();
    let mut rec = ResizeRecognizer::new(target_id.clone(), hit.edge);
    if rec.on_pointer_down(x, y) {
        tracing::debug!(
            target: "gui::gesture",
            node_id = %target.stable_id(),
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
