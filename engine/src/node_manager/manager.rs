use crate::node_manager::NodeDef;
use std::collections::HashMap;
use types::{DataType, Value};

pub struct NodeManager {
    nodes: HashMap<String, NodeDef>,
}

impl NodeManager {
    pub fn from_inventory() -> Self {
        let rust_defs = crate::node_manager::collect::collect_rust_defs::collect_rust_defs();
        let python_defs = crate::node_manager::collect::collect_python_defs::collect_python_defs();
        let defs = crate::node_manager::collect::merge_node_defs::merge_node_defs(rust_defs, python_defs);
        let nodes = crate::node_manager::store::defs_by_type::defs_by_type(defs);
        Self { nodes }
    }

    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: NodeDef) {
        self.nodes.insert(def.type_id.clone(), def);
    }

    pub(crate) fn nodes(&self) -> &HashMap<String, NodeDef> {
        &self.nodes
    }

    pub fn default_params(&self, type_id: &str) -> Option<HashMap<String, Value>> {
        self.get_node_def(type_id).map(|def| {
            def.params
                .iter()
                .map(|p| (p.name.clone(), p.default_value.clone()))
                .collect()
        })
    }

    pub fn pin_type(&self, type_id: &str, pin_name: &str) -> Option<&DataType> {
        let def = self.get_node_def(type_id)?;
        def.inputs
            .iter()
            .chain(def.outputs.iter())
            .find(|p| p.name == pin_name)
            .map(|p| &p.data_type)
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::collect::collect_python_defs::{
        parse_param_expose, python_node_decl_specs, PythonNodeDeclFormat,
        PythonParamExpose,
    };
    use crate::node_manager::{ExposedPinSource, ParamDef, ParamExpose, PinDef};
    use std::collections::HashSet;

    fn make_test_def(type_id: &str, category: &str) -> NodeDef {
        NodeDef {
            type_id: type_id.into(),
            name: type_id.into(),
            category: category.into(),
            inputs: vec![PinDef {
                name: "in".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "amount".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Control],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        assert!(nm.get_node_def("brightness").is_some());
        assert!(nm.get_node_def("nonexistent").is_none());
    }

    #[test]
    fn test_pin_type() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        assert_eq!(nm.pin_type("brightness", "in"), Some(&DataType::image()));
        assert_eq!(nm.pin_type("brightness", "out"), Some(&DataType::image()));
        assert_eq!(nm.pin_type("brightness", "nope"), None);
    }

    #[test]
    fn test_default_params() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        let params = nm.default_params("brightness").unwrap();
        assert!(matches!(params.get("amount"), Some(Value::Float(f)) if *f == 0.0));
    }

    #[test]
    fn test_list_node_defs_returns_all_registered_defs() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        nm.register(make_test_def("contrast", "color"));

        let type_ids: HashSet<_> = nm
            .list_node_defs()
            .into_iter()
            .map(|def| def.type_id.as_str())
            .collect();

        assert_eq!(type_ids.len(), 2);
        assert!(type_ids.contains("brightness"));
        assert!(type_ids.contains("contrast"));
    }

    #[test]
    fn test_list_nodes_by_category_filters_defs() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        nm.register(make_test_def("contrast", "color"));
        nm.register(make_test_def("blur", "filter"));

        let color_type_ids: HashSet<_> = nm
            .list_nodes_by_category("color")
            .into_iter()
            .map(|def| def.type_id.as_str())
            .collect();

        assert_eq!(color_type_ids.len(), 2);
        assert!(color_type_ids.contains("brightness"));
        assert!(color_type_ids.contains("contrast"));
        assert!(!color_type_ids.contains("blur"));

        let filter_type_ids: HashSet<_> = nm
            .list_nodes_by_category("filter")
            .into_iter()
            .map(|def| def.type_id.as_str())
            .collect();

        assert_eq!(filter_type_ids.len(), 1);
        assert!(filter_type_ids.contains("blur"));

        assert!(nm.list_nodes_by_category("missing").is_empty());
    }

    #[test]
    fn test_search_nodes_matches_name_and_type_id() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        nm.register(make_test_def("blur", "filter"));
        nm.register(make_test_def("contrast", "color"));

        let bright_matches: HashSet<_> = nm
            .search_nodes("bright")
            .into_iter()
            .map(|def| def.type_id.as_str())
            .collect();
        assert_eq!(bright_matches.len(), 1);
        assert!(bright_matches.contains("brightness"));

        let blur_matches: HashSet<_> = nm
            .search_nodes("blur")
            .into_iter()
            .map(|def| def.type_id.as_str())
            .collect();
        assert_eq!(blur_matches.len(), 1);
        assert!(blur_matches.contains("blur"));

        let uppercase_matches: HashSet<_> = nm
            .search_nodes("BRIGHT")
            .into_iter()
            .map(|def| def.type_id.as_str())
            .collect();
        assert_eq!(uppercase_matches.len(), 1);
        assert!(uppercase_matches.contains("brightness"));

        assert!(nm.search_nodes("missing").is_empty());
    }

    #[test]
    fn test_list_categories_returns_unique_sorted_categories() {
        let mut nm = NodeManager::new();
        nm.register(make_test_def("brightness", "color"));
        nm.register(make_test_def("contrast", "color"));
        nm.register(make_test_def("blur", "filter"));
        nm.register(make_test_def("load_image", "data"));

        let categories = nm.list_categories();

        assert_eq!(categories, vec!["color", "data", "filter"]);
    }

    #[test]
    fn test_from_inventory_list_node_defs_contains_builtins() {
        let nm = NodeManager::from_inventory();

        let type_ids: HashSet<_> = nm
            .list_node_defs()
            .into_iter()
            .map(|def| def.type_id.as_str())
            .collect();

        assert!(type_ids.contains("load_image"));
        assert!(type_ids.contains("save_image"));
        assert!(type_ids.contains("brightness"));
        assert!(type_ids.contains("contrast"));
    }

    #[test]
    fn test_python_node_decl_specs_are_generated() {
        let specs = python_node_decl_specs();
        assert!(!specs.is_empty());

        let load_checkpoint = specs
            .iter()
            .find(|spec| spec.path == "nodes/load_checkpoint.py")
            .expect("load_checkpoint.py should be scanned");
        assert_eq!(load_checkpoint.format, PythonNodeDeclFormat::Decorator);
        assert_eq!(load_checkpoint.type_id, "ai.load_checkpoint");
        assert_eq!(load_checkpoint.title, "Load Checkpoint");
        assert_eq!(load_checkpoint.category, "ai/model");
        assert!(load_checkpoint.inputs.is_empty());
        assert_eq!(load_checkpoint.outputs.len(), 3);
        assert_eq!(load_checkpoint.outputs[0].name, "model");
        assert_eq!(load_checkpoint.outputs[0].data_type, "MODEL");
        assert!(!load_checkpoint.outputs[0].required);
        assert_eq!(load_checkpoint.params.len(), 1);
        assert_eq!(load_checkpoint.params[0].name, "checkpoint_path");
        assert_eq!(load_checkpoint.params[0].data_type, "STRING");
        assert_eq!(load_checkpoint.params[0].default_expr, "None");

        let clip_text_encode = specs
            .iter()
            .find(|spec| spec.path == "nodes/clip_text_encode.py")
            .expect("clip_text_encode.py should be scanned");
        assert_eq!(clip_text_encode.inputs.len(), 1);
        assert_eq!(clip_text_encode.inputs[0].name, "clip");
        assert!(clip_text_encode.inputs[0].required);
        assert_eq!(clip_text_encode.params.len(), 1);
        assert_eq!(clip_text_encode.params[0].name, "text");
        assert_eq!(clip_text_encode.params[0].default_expr, "\"\"");

        let decorator_node = specs
            .iter()
            .find(|spec| spec.path == "nodes/clip_text_encode.py")
            .expect("clip_text_encode.py should be scanned as decorator node");
        assert_eq!(decorator_node.format, PythonNodeDeclFormat::Decorator);
        assert_eq!(decorator_node.type_id, "ai.clip_text_encode");
        assert_eq!(decorator_node.title, "CLIP Text Encode");
        assert_eq!(decorator_node.category, "ai/conditioning");
        assert_eq!(decorator_node.inputs.len(), 1);
        assert_eq!(decorator_node.inputs[0].name, "clip");
        assert!(decorator_node.inputs[0].required);
        assert_eq!(decorator_node.outputs.len(), 1);
        assert_eq!(decorator_node.outputs[0].name, "conditioning");
        assert_eq!(decorator_node.params.len(), 1);
        assert_eq!(decorator_node.params[0].name, "text");
        assert_eq!(decorator_node.params[0].default_expr, "\"\"");
        assert_eq!(decorator_node.params[0].expose_expr, &["[\"control\", \"input\"]"]);
    }

    #[test]
    fn test_from_inventory_includes_python_node_defs() {
        let nm = NodeManager::from_inventory();

        let load_checkpoint = nm
            .get_node_def("ai.load_checkpoint")
            .expect("load_checkpoint node def should exist");
        assert_eq!(load_checkpoint.name, "Load Checkpoint");
        assert_eq!(load_checkpoint.category, "ai/model");
        assert_eq!(load_checkpoint.outputs.len(), 3);
        assert_eq!(load_checkpoint.outputs[0].name, "model");
        assert_eq!(load_checkpoint.outputs[0].data_type, DataType("ai.model".into()));
        assert_eq!(load_checkpoint.params.len(), 1);
        assert_eq!(load_checkpoint.params[0].name, "checkpoint_path");
        assert_eq!(load_checkpoint.params[0].data_type, DataType::string());
        assert!(matches!(
            &load_checkpoint.params[0].default_value,
            Value::String(value) if value.is_empty()
        ));

        let ksampler = nm
            .get_node_def("ai.ksampler")
            .expect("ksampler node def should exist");
        let seed = ksampler
            .params
            .iter()
            .find(|param| param.name == "seed")
            .expect("seed param should exist");
        assert_eq!(seed.data_type, DataType::int());
        assert!(matches!(seed.default_value, Value::Int(0)));
        assert!(seed.constraint.is_some());

        let sampler_name = ksampler
            .params
            .iter()
            .find(|param| param.name == "sampler_name")
            .expect("sampler_name param should exist");
        assert_eq!(sampler_name.data_type, DataType::string());
        assert!(matches!(
            &sampler_name.default_value,
            Value::String(value) if value == "euler"
        ));
        assert!(sampler_name.constraint.is_some());

        let clip_text_encode = nm
            .get_node_def("ai.clip_text_encode")
            .expect("clip_text_encode node def should exist");
        assert_eq!(clip_text_encode.name, "CLIP Text Encode");
        assert_eq!(clip_text_encode.category, "ai/conditioning");
        assert_eq!(clip_text_encode.inputs.len(), 1);
        assert_eq!(clip_text_encode.inputs[0].name, "clip");
        assert!(!clip_text_encode.inputs[0].optional);
        assert_eq!(clip_text_encode.outputs.len(), 1);
        assert_eq!(clip_text_encode.outputs[0].name, "conditioning");
        assert_eq!(clip_text_encode.params.len(), 1);

        let text = clip_text_encode
            .params
            .iter()
            .find(|param| param.name == "text")
            .expect("text param should exist");
        assert_eq!(text.data_type, DataType::string());
        assert!(matches!(&text.default_value, Value::String(v) if v.is_empty()));
        assert_eq!(text.expose, vec![ParamExpose::Control, ParamExpose::Input]);

        let ksampler = nm
            .get_node_def("ai.ksampler")
            .expect("ksampler node def should exist");

        let sampler_name = ksampler
            .params
            .iter()
            .find(|param| param.name == "sampler_name")
            .expect("sampler_name param should exist");
        assert_eq!(sampler_name.data_type, DataType::string());
        assert!(matches!(
            &sampler_name.default_value,
            Value::String(value) if value == "euler"
        ));
        assert!(sampler_name.constraint.is_some());

        let scheduler = ksampler
            .params
            .iter()
            .find(|param| param.name == "scheduler")
            .expect("scheduler param should exist");
        assert!(scheduler.constraint.is_some());

        let empty_latent = nm
            .get_node_def("ai.empty_latent_image")
            .expect("empty_latent_image node def should exist");
        let width = empty_latent
            .params
            .iter()
            .find(|param| param.name == "width")
            .expect("width param should exist");
        assert!(width.constraint.is_some());

        let decorator = nm
            .get_node_def("ai.load_checkpoint")
            .expect("load_checkpoint node def should exist");
        assert_eq!(decorator.name, "Load Checkpoint");
        assert_eq!(decorator.category, "ai/model");
        assert!(decorator.inputs.is_empty());
        assert_eq!(decorator.outputs.len(), 3);

        let checkpoint_path = decorator
            .params
            .iter()
            .find(|param| param.name == "checkpoint_path")
            .expect("checkpoint_path param should exist");
        assert_eq!(checkpoint_path.expose, vec![ParamExpose::Control]);
    }

    #[test]
    fn test_parse_param_expose_normalizes_decorator_expose_expr() {
        let specs = python_node_decl_specs();
        let decorator_node = specs
            .iter()
            .find(|spec| spec.path == "nodes/clip_text_encode.py")
            .expect("clip_text_encode.py should be scanned");

        let text = decorator_node
            .params
            .iter()
            .find(|param| param.name == "text")
            .expect("text param should be scanned");
        assert_eq!(
            parse_param_expose(text),
            vec![PythonParamExpose::Control, PythonParamExpose::Input]
        );

        let ksampler = specs
            .iter()
            .find(|spec| spec.path == "nodes/ksampler.py")
            .expect("ksampler.py should be scanned");
        let seed = ksampler
            .params
            .iter()
            .find(|param| param.name == "seed")
            .expect("seed param should be scanned");
        assert_eq!(
            parse_param_expose(seed),
            vec![PythonParamExpose::Control, PythonParamExpose::Input]
        );
    }

    #[test]
    fn test_list_exposed_input_pins_includes_param_input_expose() {
        let nm = NodeManager::from_inventory();
        let pins = nm
            .list_exposed_input_pins("ai.clip_text_encode")
            .expect("clip_text_encode should expose inputs");

        let names = pins.iter().map(|pin| pin.name.as_str()).collect::<HashSet<_>>();
        assert!(names.contains("clip"));
        assert!(names.contains("text"));

        let text = pins
            .iter()
            .find(|pin| pin.name == "text")
            .expect("text should be exposed as input");
        assert!(text.optional);
        assert_eq!(text.source, ExposedPinSource::Param);
    }

    #[test]
    fn test_list_exposed_pins_contains_pin_and_param_views() {
        let nm = NodeManager::from_inventory();
        let pins = nm
            .list_exposed_pins("ai.load_checkpoint")
            .expect("load_checkpoint should expose pins");

        let names = pins.iter().map(|pin| pin.name.as_str()).collect::<HashSet<_>>();
        assert!(names.contains("model"));
        assert!(names.contains("clip"));
        assert!(names.contains("vae"));

        let checkpoint_path_pin_count = pins
            .iter()
            .filter(|pin| pin.name == "checkpoint_path")
            .count();
        assert_eq!(checkpoint_path_pin_count, 0);

        assert_eq!(pins.len(), 3);
    }

    #[test]
    fn test_list_exposed_output_pins_includes_param_output_expose() {
        let nm = NodeManager::from_inventory();
        let pins = nm
            .list_exposed_output_pins("ai.ksampler")
            .expect("ksampler should expose outputs");

        let names = pins.iter().map(|pin| pin.name.as_str()).collect::<HashSet<_>>();
        assert!(names.contains("latent"));
        assert!(!names.contains("seed"));

        let latent = pins
            .iter()
            .find(|pin| pin.name == "latent")
            .expect("latent should be exposed as output");
        assert_eq!(latent.source, ExposedPinSource::Pin);
        assert!(!latent.optional);
    }

    #[test]
    fn test_get_exposed_pin_queries_general_and_directional_views() {
        let nm = NodeManager::from_inventory();

        let text_input = nm
            .get_exposed_input_pin("ai.clip_text_encode", "text")
            .expect("text should be exposed as input");
        assert_eq!(text_input.source, ExposedPinSource::Param);
        assert!(text_input.optional);

        let latent_output = nm
            .get_exposed_output_pin("ai.ksampler", "latent")
            .expect("latent should be exposed as output");
        assert_eq!(latent_output.source, ExposedPinSource::Pin);

        let conditioning = nm
            .get_exposed_pin("ai.clip_text_encode", "conditioning")
            .expect("conditioning should exist in exposed pins");
        assert_eq!(conditioning.source, ExposedPinSource::Pin);

        assert!(nm
            .get_exposed_input_pin("ai.clip_text_encode", "conditioning")
            .is_none());
        assert!(nm
            .get_exposed_output_pin("ai.clip_text_encode", "text")
            .is_none());
        assert!(nm.get_exposed_pin("ai.clip_text_encode", "missing").is_none());
    }
}
