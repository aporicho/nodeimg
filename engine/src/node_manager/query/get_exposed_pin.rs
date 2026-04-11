use crate::node_manager::{ExposedPinDef, NodeManager};

impl NodeManager {
    pub fn get_exposed_pin(&self, type_id: &str, pin_name: &str) -> Option<ExposedPinDef> {
        self.list_exposed_pins(type_id)?
            .into_iter()
            .find(|pin| pin.name == pin_name)
    }
}
