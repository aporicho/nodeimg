use crate::node_manager::{ExposedPinDef, ExposedPinKind, NodeManager};

impl NodeManager {
    pub fn list_exposed_input_pins(&self, type_id: &str) -> Option<Vec<ExposedPinDef>> {
        Some(
            self.list_exposed_pins(type_id)?
                .into_iter()
                .filter(|pin| pin.kind == ExposedPinKind::Input)
                .collect(),
        )
    }
}
