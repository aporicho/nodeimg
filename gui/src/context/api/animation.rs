use super::super::Context;
use std::time::Instant;

pub struct AnimationApi<'a> {
    pub(in crate::context) ctx: &'a Context,
}

impl AnimationApi<'_> {
    pub fn active(&self) -> bool {
        self.ctx.animations_active()
    }
}

pub struct AnimationMutApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl AnimationMutApi<'_> {
    pub fn tick(&mut self, now: Instant) -> bool {
        self.ctx.tick_animations(now)
    }
}
