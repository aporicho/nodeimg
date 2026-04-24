use crate::animation::AnimationStore;
use crate::shell::{AppEvent, Key, MouseButton};
use crate::tree::{HitChain, Tree};

use super::state::InteractionState;
use super::target;

pub(crate) fn apply_event(
    store: &mut InteractionState,
    tree: &Tree,
    animations: Option<&AnimationStore>,
    event: &AppEvent,
) {
    match *event {
        AppEvent::MouseMove { x, y } => {
            if store.captured().is_none() {
                let chain = hit_chain(tree, animations, x, y);
                store.set_hovered(target::input_target(tree, &chain));
            }
        }
        AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        } => {
            let chain = hit_chain(tree, animations, x, y);
            let input_target = target::input_target(tree, &chain);
            let focus_target = target::interactive_target(tree, &chain);
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

fn hit_chain(tree: &Tree, animations: Option<&AnimationStore>, x: f32, y: f32) -> HitChain {
    match tree.root() {
        Some(root) => crate::tree::hit_test_with_animations(tree, root, x, y, animations),
        None => HitChain::empty(),
    }
}
