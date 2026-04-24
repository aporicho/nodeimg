use super::easing::Ease;
use super::props::{AnimatedVisual, AnimationProps};
use super::timeline::TimelineBuilder;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnimationId(u64);

impl AnimationId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
struct ActiveTween {
    id: AnimationId,
    target: String,
    from: AnimatedVisual,
    to: AnimationProps,
    start: Instant,
    duration: Duration,
    ease: Ease,
}

impl ActiveTween {
    fn is_complete(&self, now: Instant) -> bool {
        now >= self.start + self.duration
    }

    fn sample(&self, now: Instant) -> AnimationProps {
        let t = if now <= self.start {
            0.0
        } else if self.duration.is_zero() {
            1.0
        } else {
            (now - self.start).as_secs_f32() / self.duration.as_secs_f32()
        };
        self.from.sample_to(self.to, self.ease.sample(t))
    }
}

#[derive(Debug, Default)]
pub struct AnimationStore {
    next_id: u64,
    active: Vec<ActiveTween>,
    settled: BTreeMap<String, AnimatedVisual>,
    last_tick: Option<Instant>,
}

impl AnimationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn animate(&mut self, target: impl Into<String>) -> AnimationBuilder<'_> {
        AnimationBuilder::new(self, target.into())
    }

    pub fn timeline(&mut self) -> TimelineBuilder<'_> {
        TimelineBuilder::new(self)
    }

    pub fn play_at(
        &mut self,
        target: impl Into<String>,
        to: AnimationProps,
        duration: Duration,
        delay: Duration,
        ease: Ease,
        now: Instant,
    ) -> AnimationId {
        self.last_tick = Some(now);
        let target = target.into();
        let id = self.allocate_id();
        let from = self.visual_for_at(&target, now);
        self.active.push(ActiveTween {
            id,
            target,
            from,
            to,
            start: now + delay,
            duration,
            ease,
        });
        id
    }

    pub fn cancel(&mut self, id: AnimationId) -> bool {
        let before = self.active.len();
        self.active.retain(|tween| tween.id != id);
        before != self.active.len()
    }

    pub fn clear_visual(&mut self, target: &str) -> bool {
        let removed_settled = self.settled.remove(target).is_some();
        let before = self.active.len();
        self.active.retain(|tween| tween.target != target);
        removed_settled || before != self.active.len()
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        self.last_tick = Some(now);
        if self.active.is_empty() {
            return false;
        }

        let completed_targets: BTreeSet<String> = self
            .active
            .iter()
            .filter(|tween| tween.is_complete(now))
            .map(|tween| tween.target.clone())
            .collect();

        for target in completed_targets {
            let visual = self.visual_for_at(&target, now);
            if visual.is_identity() {
                self.settled.remove(&target);
            } else {
                self.settled.insert(target, visual);
            }
        }

        self.active.retain(|tween| !tween.is_complete(now));
        self.active()
    }

    pub fn active(&self) -> bool {
        !self.active.is_empty()
    }

    pub fn visual_for(&self, target: &str) -> Option<AnimatedVisual> {
        let now = self.last_tick?;
        let visual = self.visual_for_at(target, now);
        (!visual.is_identity()).then_some(visual)
    }

    fn visual_for_at(&self, target: &str, now: Instant) -> AnimatedVisual {
        let mut visual = self.settled.get(target).copied().unwrap_or_default();
        for tween in self.active.iter().filter(|tween| tween.target == target) {
            visual.apply(tween.sample(now));
        }
        visual
    }

    fn allocate_id(&mut self) -> AnimationId {
        self.next_id = self.next_id.saturating_add(1);
        AnimationId(self.next_id)
    }
}

pub struct AnimationBuilder<'a> {
    store: &'a mut AnimationStore,
    target: String,
    to: AnimationProps,
    duration: Duration,
    delay: Duration,
    ease: Ease,
}

impl<'a> AnimationBuilder<'a> {
    fn new(store: &'a mut AnimationStore, target: String) -> Self {
        Self {
            store,
            target,
            to: AnimationProps::default(),
            duration: Duration::from_millis(180),
            delay: Duration::ZERO,
            ease: Ease::default(),
        }
    }

    pub fn to(mut self, props: AnimationProps) -> Self {
        self.to = props;
        self
    }

    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration = Duration::from_millis(duration_ms);
        self
    }

    pub fn delay_ms(mut self, delay_ms: u64) -> Self {
        self.delay = Duration::from_millis(delay_ms);
        self
    }

    pub fn ease(mut self, ease: Ease) -> Self {
        self.ease = ease;
        self
    }

    pub fn play(self) -> AnimationId {
        self.play_at(Instant::now())
    }

    pub fn play_at(self, now: Instant) -> AnimationId {
        self.store.play_at(
            self.target,
            self.to,
            self.duration,
            self.delay,
            self.ease,
            now,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_interpolates_and_settles_final_visual() {
        let mut store = AnimationStore::new();
        let now = Instant::now();
        store
            .animate("node")
            .to(AnimationProps::new().opacity(0.0).translate([10.0, 0.0]))
            .duration_ms(100)
            .ease(Ease::Linear)
            .play_at(now);

        store.tick(now + Duration::from_millis(50));
        let visual = store.visual_for("node").expect("active visual");
        assert_eq!(visual.opacity, 0.5);
        assert_eq!(visual.translate, [5.0, 0.0]);

        assert!(!store.tick(now + Duration::from_millis(100)));
        let visual = store.visual_for("node").expect("settled visual");
        assert_eq!(visual.opacity, 0.0);
        assert_eq!(visual.translate, [10.0, 0.0]);
    }

    #[test]
    fn cancel_removes_active_animation() {
        let mut store = AnimationStore::new();
        let now = Instant::now();
        let id = store
            .animate("node")
            .to(AnimationProps::new().opacity(0.0))
            .duration_ms(100)
            .play_at(now);

        assert!(store.cancel(id));
        store.tick(now + Duration::from_millis(50));
        assert!(store.visual_for("node").is_none());
    }
}
