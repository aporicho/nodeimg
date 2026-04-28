use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextBoxRegistry {
    instances: HashMap<String, TextBoxRegistryEntry>,
    dirty_intrinsics: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextBoxRegistryEntry {
    pub(crate) external_value: String,
    pub(crate) editor_value: String,
}

impl TextBoxRegistry {
    pub(crate) fn register_instance(
        &mut self,
        stable_id: impl Into<String>,
        value: impl Into<String>,
    ) {
        let value = value.into();
        self.instances
            .entry(stable_id.into())
            .or_insert_with(|| TextBoxRegistryEntry {
                external_value: value.clone(),
                editor_value: value,
            });
    }

    pub(crate) fn update_external_value(&mut self, stable_id: &str, value: impl Into<String>) {
        let value = value.into();
        let entry = self.instances.entry(stable_id.to_string()).or_default();
        if entry.external_value != value {
            entry.external_value = value.clone();
            if entry.editor_value != value {
                entry.editor_value = value;
                self.dirty_intrinsics.insert(stable_id.to_string());
            }
        }
    }

    pub(crate) fn handle_editor_change(&mut self, stable_id: &str, value: impl Into<String>) {
        let value = value.into();
        let entry = self.instances.entry(stable_id.to_string()).or_default();
        if entry.editor_value != value {
            entry.editor_value = value;
            self.dirty_intrinsics.insert(stable_id.to_string());
        }
    }

    pub(crate) fn mark_dirty_intrinsic(&mut self, stable_id: &str) {
        self.dirty_intrinsics.insert(stable_id.to_string());
    }

    pub(crate) fn dirty_intrinsics(&self) -> impl Iterator<Item = &str> {
        self.dirty_intrinsics.iter().map(String::as_str)
    }

    pub(crate) fn take_dirty_intrinsics(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.dirty_intrinsics)
    }
}
