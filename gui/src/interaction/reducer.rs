use crate::animation::AnimationStore;
use crate::input::{PointerHitResolver, PointerHitSnapshot};
use crate::shell::{AppEvent, Key, MouseButton};
use crate::tree::{TargetChain, Tree};

use super::state::InteractionState;

pub(crate) fn apply_event(
    store: &mut InteractionState,
    tree: &Tree,
    animations: Option<&AnimationStore>,
    event: &AppEvent,
    hit: Option<&PointerHitSnapshot>,
) {
    match *event {
        AppEvent::MouseMove { x, y } => {
            if store.captured().is_none() {
                let chain = PointerHitResolver::new(tree, animations).chain_at(hit, x, y);
                let targets = TargetChain::from_hit_chain(tree, &chain);
                store.set_hovered(targets.input_target());
            }
        }
        AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        } => {
            let targets = PointerHitResolver::new(tree, animations).target_chain_at(hit, x, y);
            let input_target = targets.input_target();
            let focus_target = targets.focus_target();
            store.set_hovered(input_target);
            store.set_pressed(input_target);
            store.set_captured(input_target);
            if let Some(target) = focus_target {
                store.focus(target);
            } else {
                store.blur();
            }
        }
        AppEvent::MouseRelease {
            button: MouseButton::Left,
            ..
        } => {
            store.set_pressed(None);
            store.set_captured(None);
        }
        AppEvent::KeyPress {
            key: Key::Tab,
            modifiers,
        } => {
            if modifiers.shift {
                store.tab_prev();
            } else {
                store.tab_next();
            }
        }
        AppEvent::Unfocused => {
            store.clear_pointer_state();
            store.blur();
        }
        _ => {}
    }
}
