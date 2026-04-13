use crate::shell::{AppEvent, Key};
use crate::tree::Tree;

use super::interaction::InteractionStore;
use super::registry;

pub fn apply_event(store: &mut InteractionStore, tree: &Tree, event: &AppEvent) {
    match *event {
        AppEvent::MouseMove { x, y } => {
            if store.captured().is_none() {
                let chain = match tree.root() {
                    Some(root) => crate::tree::hit_test(tree, root, x, y),
                    None => crate::tree::HitChain::empty(),
                };
                store.set_hovered(registry::input_target(tree, &chain));
            }
        }
        AppEvent::MousePress { x, y, button } if button == crate::shell::MouseButton::Left => {
            let chain = match tree.root() {
                Some(root) => crate::tree::hit_test(tree, root, x, y),
                None => crate::tree::HitChain::empty(),
            };
            let input_target = registry::input_target(tree, &chain);
            let focus_target = registry::interactive_target(tree, &chain);
            store.set_hovered(input_target);
            store.set_pressed(input_target);
            store.set_captured(input_target);
            if let Some(target) = focus_target {
                store.focus(target);
            }
        }
        AppEvent::MouseRelease { button, .. } if button == crate::shell::MouseButton::Left => {
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
