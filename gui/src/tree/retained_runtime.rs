use super::{RuntimeSlot, RuntimeSlots};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct RetainedRuntimeStore {
    slots: HashMap<String, RuntimeSlots>,
}

impl RetainedRuntimeStore {
    pub fn preserve(&mut self, stable_id: String, slots: RuntimeSlots) {
        if !slots.is_empty() {
            self.slots.insert(stable_id, slots);
        }
    }

    pub fn take(&mut self, stable_id: &str) -> Option<RuntimeSlots> {
        self.slots.remove(stable_id)
    }

    pub fn contains(&self, stable_id: &str) -> bool {
        self.slots.contains_key(stable_id)
    }

    pub fn get<T: RuntimeSlot>(&self, stable_id: &str) -> Option<&T> {
        self.slots.get(stable_id)?.get::<T>()
    }

    pub fn get_mut<T: RuntimeSlot>(&mut self, stable_id: &str) -> Option<&mut T> {
        self.slots.get_mut(stable_id)?.get_mut::<T>()
    }

    pub fn ensure<T: RuntimeSlot>(&mut self, stable_id: &str) -> &mut T {
        self.slots
            .entry(stable_id.to_string())
            .or_default()
            .ensure::<T>()
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&str, &RuntimeSlots) -> bool) {
        self.slots
            .retain(|stable_id, slots| keep(stable_id.as_str(), slots));
    }

    pub fn values(&self) -> impl Iterator<Item = &RuntimeSlots> {
        self.slots.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &RuntimeSlots)> {
        self.slots
            .iter()
            .map(|(stable_id, slots)| (stable_id.as_str(), slots))
    }
}
