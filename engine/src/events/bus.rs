use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use super::types::EventRecord;

#[derive(Clone)]
pub struct EventBus<T> {
    inner: Arc<Mutex<EventBusInner<T>>>,
}

pub struct EventSubscription<T> {
    inner: Arc<Mutex<EventBusInner<T>>>,
    next_seq: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PollResult<T> {
    Items(Vec<EventRecord<T>>),
    Lagged { next_valid_seq: u64 },
    Empty,
}

struct EventBusInner<T> {
    records: VecDeque<EventRecord<T>>,
    next_seq: u64,
    capacity: usize,
}

impl<T> EventBus<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(EventBusInner {
                records: VecDeque::new(),
                next_seq: 0,
                capacity: capacity.max(1),
            })),
        }
    }

    pub fn subscribe(&self) -> EventSubscription<T> {
        let next_seq = self.latest_seq();
        EventSubscription {
            inner: Arc::clone(&self.inner),
            next_seq,
        }
    }

    pub fn subscribe_from(&self, seq: u64) -> EventSubscription<T> {
        EventSubscription {
            inner: Arc::clone(&self.inner),
            next_seq: seq,
        }
    }

    pub fn latest_seq(&self) -> u64 {
        self.inner.lock().expect("event bus lock poisoned").next_seq
    }
}

impl<T: Clone> EventBus<T> {
    pub fn publish(&self, payload: T) -> EventRecord<T> {
        let mut inner = self.inner.lock().expect("event bus lock poisoned");
        let record = EventRecord {
            seq: inner.next_seq,
            at: SystemTime::now(),
            payload,
        };
        inner.next_seq += 1;
        inner.records.push_back(record.clone());
        while inner.records.len() > inner.capacity {
            inner.records.pop_front();
        }
        record
    }

    pub fn snapshot(&self) -> Vec<EventRecord<T>> {
        self.inner
            .lock()
            .expect("event bus lock poisoned")
            .records
            .iter()
            .cloned()
            .collect()
    }
}

impl<T: Clone> EventSubscription<T> {
    pub fn poll_pending(&mut self) -> PollResult<T> {
        let inner = self.inner.lock().expect("event bus lock poisoned");
        let oldest_seq = inner.records.front().map(|record| record.seq);
        let next_seq = inner.next_seq;

        if let Some(oldest_seq) = oldest_seq {
            if self.next_seq < oldest_seq {
                self.next_seq = oldest_seq;
                return PollResult::Lagged {
                    next_valid_seq: oldest_seq,
                };
            }
        }

        if self.next_seq >= next_seq {
            return PollResult::Empty;
        }

        let records = inner
            .records
            .iter()
            .filter(|record| record.seq >= self.next_seq)
            .cloned()
            .collect::<Vec<_>>();
        drop(inner);

        self.next_seq = next_seq;
        if records.is_empty() {
            PollResult::Empty
        } else {
            PollResult::Items(records)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventBus, PollResult};

    #[test]
    fn subscription_starts_at_tail() {
        let bus = EventBus::new(8);
        let _ = bus.publish(1_u32);
        let mut sub = bus.subscribe();

        assert!(matches!(sub.poll_pending(), PollResult::Empty));
        let _ = bus.publish(2_u32);

        assert!(
            matches!(sub.poll_pending(), PollResult::Items(items) if items.len() == 1 && items[0].payload == 2)
        );
    }

    #[test]
    fn subscription_can_replay_from_sequence() {
        let bus = EventBus::new(8);
        let _ = bus.publish(10_u32);
        let _ = bus.publish(20_u32);
        let mut sub = bus.subscribe_from(0);

        assert!(
            matches!(sub.poll_pending(), PollResult::Items(items) if items.len() == 2 && items[0].payload == 10 && items[1].payload == 20)
        );
    }

    #[test]
    fn subscription_reports_lagged_consumers() {
        let bus = EventBus::new(2);
        let _ = bus.publish(1_u32);
        let _ = bus.publish(2_u32);
        let _ = bus.publish(3_u32);
        let mut sub = bus.subscribe_from(0);

        assert!(matches!(
            sub.poll_pending(),
            PollResult::Lagged { next_valid_seq: 1 }
        ));
    }
}
