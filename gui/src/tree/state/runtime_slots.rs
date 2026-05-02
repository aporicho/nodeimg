use super::Tree;
use crate::tree::node::NodeId;
use crate::tree::runtime_slots::RuntimeSlot;
use crate::tree::RuntimeSlots;

impl Tree {
    pub fn runtime_slot<T: RuntimeSlot>(&self, id: NodeId) -> Option<&T> {
        self.get(id)?.runtime_slots.get::<T>()
    }

    pub fn runtime_slot_mut<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<&mut T> {
        self.get_mut(id)?.runtime_slots.get_mut::<T>()
    }

    pub fn ensure_runtime_slot<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<&mut T> {
        Some(self.get_mut(id)?.runtime_slots.ensure::<T>())
    }

    pub fn remove_runtime_slot<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<T> {
        self.get_mut(id)?.runtime_slots.remove::<T>()
    }

    pub(crate) fn runtime_slot_by_stable_id<T: RuntimeSlot>(&self, id: &str) -> Option<&T> {
        if let Some(node_id) = self.node_by_str(id) {
            return self.runtime_slot::<T>(node_id);
        }
        self.retained_runtime.get::<T>(id)
    }

    pub(crate) fn runtime_slot_by_stable_id_mut<T: RuntimeSlot>(
        &mut self,
        id: &str,
    ) -> Option<&mut T> {
        if let Some(node_id) = self.node_by_str(id) {
            return self.runtime_slot_mut::<T>(node_id);
        }
        self.retained_runtime.get_mut::<T>(id)
    }

    pub(crate) fn ensure_runtime_slot_by_stable_id<T: RuntimeSlot>(&mut self, id: &str) -> &mut T {
        if let Some(node_id) = self.node_by_str(id) {
            return self
                .ensure_runtime_slot::<T>(node_id)
                .expect("node id came from this tree");
        }
        self.retained_runtime.ensure::<T>(id)
    }

    pub(crate) fn retained_runtime_values(&self) -> impl Iterator<Item = &RuntimeSlots> {
        self.retained_runtime.values()
    }

    pub(crate) fn retained_runtime_iter(&self) -> impl Iterator<Item = (&str, &RuntimeSlots)> {
        self.retained_runtime.iter()
    }

    pub(crate) fn retain_retained_runtime_slots(
        &mut self,
        keep: impl FnMut(&str, &RuntimeSlots) -> bool,
    ) {
        self.retained_runtime.retain(keep);
    }
}
