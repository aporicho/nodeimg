use super::runtime_policy::{RuntimeRetention, RuntimeSlotPolicy};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;

pub trait RuntimeSlot: Any + Default + fmt::Debug {
    fn default_policy() -> RuntimeSlotPolicy {
        RuntimeSlotPolicy::default()
    }
}

#[derive(Default)]
pub struct RuntimeSlots {
    slots: HashMap<TypeId, RuntimeSlotEntry>,
}

pub struct RuntimeSlotDebugEntry<'a> {
    pub type_name: &'static str,
    pub policy: &'a RuntimeSlotPolicy,
    pub value: String,
}

impl RuntimeSlots {
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn get<T: RuntimeSlot>(&self) -> Option<&T> {
        self.slots
            .get(&TypeId::of::<T>())
            .and_then(|entry| entry.value.downcast_ref::<T>())
    }

    pub fn get_mut<T: RuntimeSlot>(&mut self) -> Option<&mut T> {
        self.slots
            .get_mut(&TypeId::of::<T>())
            .and_then(|entry| entry.value.downcast_mut::<T>())
    }

    pub fn ensure<T: RuntimeSlot>(&mut self) -> &mut T {
        self.ensure_with_policy::<T>(T::default_policy(), T::default)
    }

    pub fn ensure_with_policy<T: RuntimeSlot>(
        &mut self,
        policy: RuntimeSlotPolicy,
        init: impl FnOnce() -> T,
    ) -> &mut T {
        self.slots
            .entry(TypeId::of::<T>())
            .or_insert_with(|| RuntimeSlotEntry {
                type_name: std::any::type_name::<T>(),
                value: Box::new(init()),
                policy,
                debug_value: debug_runtime_slot::<T>,
            })
            .value
            .downcast_mut::<T>()
            .expect("runtime slot type id must match stored value")
    }

    pub(crate) fn fill_missing_from(&mut self, other: RuntimeSlots) {
        for (type_id, entry) in other.slots {
            self.slots.entry(type_id).or_insert(entry);
        }
    }

    pub(crate) fn retained_when_node_missing(mut self) -> Option<Self> {
        self.slots
            .retain(|_, entry| entry.policy.retention != RuntimeRetention::DropWhenNodeMissing);
        (!self.slots.is_empty()).then_some(self)
    }

    pub fn remove<T: RuntimeSlot>(&mut self) -> Option<T> {
        self.slots
            .remove(&TypeId::of::<T>())
            .and_then(|entry| entry.value.downcast::<T>().ok())
            .map(|value| *value)
    }

    pub fn policy<T: RuntimeSlot>(&self) -> Option<&RuntimeSlotPolicy> {
        self.slots
            .get(&TypeId::of::<T>())
            .map(|entry| &entry.policy)
    }

    pub fn debug_entries(&self) -> Vec<RuntimeSlotDebugEntry<'_>> {
        let mut entries = self
            .slots
            .values()
            .map(|entry| RuntimeSlotDebugEntry {
                type_name: entry.type_name,
                policy: &entry.policy,
                value: (entry.debug_value)(entry.value.as_ref()),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.type_name.cmp(b.type_name));
        entries
    }
}

struct RuntimeSlotEntry {
    type_name: &'static str,
    value: Box<dyn Any>,
    policy: RuntimeSlotPolicy,
    debug_value: fn(&dyn Any) -> String,
}

fn debug_runtime_slot<T: RuntimeSlot>(value: &dyn Any) -> String {
    value
        .downcast_ref::<T>()
        .map(|value| format!("{value:?}"))
        .unwrap_or_else(|| "<runtime slot type mismatch>".to_string())
}
