use std::time::Instant;

use super::Context;
use crate::animation::{AnimationBuilder, AnimationId, TimelineBuilder};

impl Context {
    pub fn animate(&mut self, id: impl Into<String>) -> AnimationBuilder<'_> {
        self.animations.animate(id)
    }

    pub fn timeline(&mut self) -> TimelineBuilder<'_> {
        self.animations.timeline()
    }

    pub fn cancel_animation(&mut self, id: AnimationId) -> bool {
        self.animations.cancel(id)
    }

    pub fn clear_animation_visual(&mut self, id: &str) -> bool {
        self.animations.clear_visual(id)
    }

    pub(crate) fn tick_animations(&mut self, now: Instant) -> bool {
        self.animations.tick(now)
    }

    pub(crate) fn animations_active(&self) -> bool {
        self.animations.active()
    }
}
