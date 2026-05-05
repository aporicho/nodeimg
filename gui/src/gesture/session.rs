use std::time::Instant;

use crate::animation::AnimationStore;
use crate::input::{PointerHitResolver, PointerHitSnapshot};
use crate::shell::{AppEvent, MouseButton};
use crate::tree::Tree;

use super::arena::GestureArena;
use super::factory::{arena_from_hit_chain, arena_from_resize_hit};
use super::signal::GestureSignal;

#[derive(Debug, Default)]
pub(crate) struct GestureSessionUpdate {
    pub(crate) consumed: bool,
    pub(crate) signal: Option<GestureSignal>,
}

impl GestureSessionUpdate {
    fn consumed() -> Self {
        Self {
            consumed: true,
            ..Self::default()
        }
    }

    fn signal(signal: GestureSignal) -> Self {
        Self {
            consumed: true,
            signal: Some(signal),
        }
    }
}

pub(crate) struct GestureSession {
    arena: Option<GestureArena>,
    last_tap_time: Option<Instant>,
}

impl GestureSession {
    pub(crate) fn new() -> Self {
        Self {
            arena: None,
            last_tap_time: None,
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        tree: &Tree,
        animations: Option<&AnimationStore>,
        event: &AppEvent,
        hit: Option<&PointerHitSnapshot>,
    ) -> GestureSessionUpdate {
        match *event {
            AppEvent::MousePress { x, y, button }
                if button == MouseButton::Left && self.arena.is_none() =>
            {
                let resolver = PointerHitResolver::new(tree, animations);
                if let Some(resize_hit) = resolver.resize_hit_at(hit, x, y) {
                    if let Some(arena) = arena_from_resize_hit(tree, resize_hit, x, y) {
                        self.arena = Some(arena);
                        return GestureSessionUpdate::consumed();
                    }
                }

                let chain = resolver.chain_at(hit, x, y);
                if let Some(arena) = arena_from_hit_chain(tree, &chain, x, y, self.last_tap_time) {
                    self.arena = Some(arena);
                    return GestureSessionUpdate::consumed();
                }
                GestureSessionUpdate::default()
            }
            AppEvent::MouseMove { x, y } => {
                let Some(arena) = self.arena.as_mut() else {
                    return GestureSessionUpdate::default();
                };
                arena
                    .pointer_move(x, y)
                    .map(|signal| self.record_signal(signal))
                    .unwrap_or_else(GestureSessionUpdate::consumed)
            }
            AppEvent::MouseRelease {
                x,
                y,
                button: MouseButton::Left,
            } => {
                let Some(mut arena) = self.arena.take() else {
                    return GestureSessionUpdate::default();
                };
                arena
                    .pointer_up(x, y)
                    .map(|signal| self.record_signal(signal))
                    .unwrap_or_else(GestureSessionUpdate::consumed)
            }
            AppEvent::Unfocused => {
                self.cancel();
                GestureSessionUpdate::default()
            }
            _ => GestureSessionUpdate::default(),
        }
    }

    pub(crate) fn cancel(&mut self) {
        self.arena = None;
    }

    fn record_signal(&mut self, signal: GestureSignal) -> GestureSessionUpdate {
        match &signal {
            GestureSignal::Click(_) => {
                self.last_tap_time = Some(Instant::now());
            }
            GestureSignal::DoubleClick(_) => {
                self.last_tap_time = None;
            }
            _ => {}
        }
        GestureSessionUpdate::signal(signal)
    }
}

impl Default for GestureSession {
    fn default() -> Self {
        Self::new()
    }
}
