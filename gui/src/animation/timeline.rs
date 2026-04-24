use super::easing::Ease;
use super::props::AnimationProps;
use super::store::{AnimationId, AnimationStore};
use std::time::{Duration, Instant};

struct TimelineEntry {
    target: String,
    props: AnimationProps,
    duration: Duration,
    delay: Duration,
    ease: Ease,
}

pub struct TimelineBuilder<'a> {
    store: &'a mut AnimationStore,
    entries: Vec<TimelineEntry>,
    cursor: Duration,
    ease: Ease,
}

impl<'a> TimelineBuilder<'a> {
    pub(crate) fn new(store: &'a mut AnimationStore) -> Self {
        Self {
            store,
            entries: Vec::new(),
            cursor: Duration::ZERO,
            ease: Ease::default(),
        }
    }

    pub fn ease(mut self, ease: Ease) -> Self {
        self.ease = ease;
        self
    }

    pub fn to(
        mut self,
        target: impl Into<String>,
        props: AnimationProps,
        duration_ms: u64,
    ) -> Self {
        let duration = Duration::from_millis(duration_ms);
        self.entries.push(TimelineEntry {
            target: target.into(),
            props,
            duration,
            delay: self.cursor,
            ease: self.ease,
        });
        self.cursor += duration;
        self
    }

    pub fn play(self) -> Vec<AnimationId> {
        self.play_at(Instant::now())
    }

    pub fn play_at(self, now: Instant) -> Vec<AnimationId> {
        let store = self.store;
        self.entries
            .into_iter()
            .map(|entry| {
                store.play_at(
                    entry.target,
                    entry.props,
                    entry.duration,
                    entry.delay,
                    entry.ease,
                    now,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_schedules_entries_in_sequence() {
        let mut store = AnimationStore::new();
        let now = Instant::now();
        let ids = store
            .timeline()
            .ease(Ease::Linear)
            .to("node", AnimationProps::new().opacity(0.0), 100)
            .to("node", AnimationProps::new().translate([10.0, 0.0]), 100)
            .play_at(now);

        assert_eq!(ids.len(), 2);
        store.tick(now + Duration::from_millis(50));
        let visual = store.visual_for("node").expect("visual");
        assert_eq!(visual.opacity, 0.5);
        assert_eq!(visual.translate, [0.0, 0.0]);

        store.tick(now + Duration::from_millis(150));
        let visual = store.visual_for("node").expect("visual");
        assert_eq!(visual.opacity, 0.0);
        assert_eq!(visual.translate, [5.0, 0.0]);
    }
}
