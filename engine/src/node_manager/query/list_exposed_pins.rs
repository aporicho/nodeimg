use crate::node_manager::{
    ExposedPinDef, ExposedPinKind, ExposedPinSource, NodeManager, ParamExpose,
};

impl NodeManager {
    pub fn list_exposed_pins(&self, type_id: &str) -> Option<Vec<ExposedPinDef>> {
        let def = self.get_node_def(type_id)?;

        let mut pins = def
            .inputs
            .iter()
            .map(|pin| ExposedPinDef {
                name: pin.name.clone(),
                data_type: pin.data_type.clone(),
                optional: pin.optional,
                kind: ExposedPinKind::Input,
                source: ExposedPinSource::Pin,
            })
            .collect::<Vec<_>>();

        pins.extend(def.outputs.iter().map(|pin| ExposedPinDef {
            name: pin.name.clone(),
            data_type: pin.data_type.clone(),
            optional: false,
            kind: ExposedPinKind::Output,
            source: ExposedPinSource::Pin,
        }));

        pins.extend(def.params.iter().flat_map(|param| {
            let mut exposed = Vec::new();

            if param.expose.contains(&ParamExpose::Input) {
                exposed.push(ExposedPinDef {
                    name: param.name.clone(),
                    data_type: param.data_type.clone(),
                    optional: true,
                    kind: ExposedPinKind::Input,
                    source: ExposedPinSource::Param,
                });
            }

            if param.expose.contains(&ParamExpose::Output) {
                exposed.push(ExposedPinDef {
                    name: param.name.clone(),
                    data_type: param.data_type.clone(),
                    optional: false,
                    kind: ExposedPinKind::Output,
                    source: ExposedPinSource::Param,
                });
            }

            exposed
        }));

        Some(pins)
    }
}
