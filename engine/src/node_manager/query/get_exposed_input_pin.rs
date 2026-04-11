use crate::node_manager::{ExposedPinDef, ExposedPinKind, NodeManager};

impl NodeManager {
    pub fn get_exposed_input_pin(&self, type_id: &str, pin_name: &str) -> Option<ExposedPinDef> {
        self.list_exposed_pins(type_id)?
            .into_iter()
            .find(|pin| pin.kind == ExposedPinKind::Input && pin.name == pin_name)
    }
}
