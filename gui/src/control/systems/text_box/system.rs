use crate::control::state::TextBoxStore;

pub(crate) struct TextBoxSystem {
    pub(super) store: TextBoxStore,
    pub(super) active_drag_text_box: Option<String>,
}

impl TextBoxSystem {
    pub(crate) fn new() -> Self {
        Self {
            store: TextBoxStore::new(),
            active_drag_text_box: None,
        }
    }

    pub(crate) fn store(&self) -> &TextBoxStore {
        &self.store
    }

    pub(crate) fn store_mut(&mut self) -> &mut TextBoxStore {
        &mut self.store
    }
}

impl Default for TextBoxSystem {
    fn default() -> Self {
        Self::new()
    }
}
