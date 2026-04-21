use super::runtime_policy::RuntimeSlotPolicy;
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub trait RuntimeSlot: Any + Default {
    fn default_policy() -> RuntimeSlotPolicy {
        RuntimeSlotPolicy::default()
    }
}

#[derive(Default)]
pub struct RuntimeSlots {
    slots: HashMap<TypeId, RuntimeSlotEntry>,
}

impl RuntimeSlots {
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
                value: Box::new(init()),
                policy,
            })
            .value
            .downcast_mut::<T>()
            .expect("runtime slot type id must match stored value")
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
}

struct RuntimeSlotEntry {
    value: Box<dyn Any>,
    policy: RuntimeSlotPolicy,
}
