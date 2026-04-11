mod brightness;
mod contrast;
mod load_image;
mod save_image;

#[cfg(test)]
mod test_macro {
    use macros::node;

    // 基本测试：简单直通节点
    node! {
        name: "test_passthrough",
        title: "Test Passthrough",
        category: "test",
        inputs: [value: Float],
        outputs: [value: Float],
        params: [],
        execute: |_ctx, inputs| {
            Ok(inputs)
        },
    }

    // 完整测试：带参数、约束、required 标记
    node! {
        name: "test_full",
        title: "Test Full Node",
        category: "test",
        inputs: [image: Image required, mask: Image],
        outputs: [image: Image],
        params: [
            brightness: Float range(-1.0, 1.0) default(0.0),
            iterations: Int range(1.0, 10.0) default(1),
            enabled: Bool default(true),
            label: String default("hello"),
        ],
        execute: |_ctx, inputs| {
            Ok(inputs)
        },
    }

    // 空列表测试
    node! {
        name: "test_empty",
        title: "Test Empty",
        category: "test",
        inputs: [],
        outputs: [],
        params: [],
        execute: |_ctx, _inputs| {
            Ok(std::collections::HashMap::new())
        },
    }
}

#[cfg(test)]
mod test_inventory {
    use types::{DataType, Value};

    #[test]
    fn test_macro_node_collected() {
        let nm = crate::node_manager::NodeManager::from_inventory();
        assert!(
            nm.get("test_passthrough").is_some(),
            "test_passthrough should be registered via inventory"
        );
    }

    #[test]
    fn test_full_node_structure() {
        let nm = crate::node_manager::NodeManager::from_inventory();
        let def = nm.get("test_full").expect("test_full should exist");

        assert_eq!(def.type_id, "test_full");
        assert_eq!(def.name, "Test Full Node");
        assert_eq!(def.category, "test");

        // 输入引脚
        assert_eq!(def.inputs.len(), 2);
        assert_eq!(def.inputs[0].name, "image");
        assert_eq!(def.inputs[0].data_type, DataType::image());
        assert!(!def.inputs[0].optional); // required
        assert_eq!(def.inputs[1].name, "mask");
        assert!(def.inputs[1].optional); // not required

        // 输出引脚
        assert_eq!(def.outputs.len(), 1);
        assert_eq!(def.outputs[0].name, "image");
        assert_eq!(def.outputs[0].data_type, DataType::image());

        // 参数
        assert_eq!(def.params.len(), 4);
        assert_eq!(def.params[0].name, "brightness");
        assert_eq!(def.params[0].data_type, DataType::float());
        assert!(def.params[0].constraint.is_some());
        assert!(matches!(def.params[0].default_value, Value::Float(f) if f == 0.0));

        assert_eq!(def.params[1].name, "iterations");
        assert_eq!(def.params[1].data_type, DataType::int());
        assert!(matches!(def.params[1].default_value, Value::Int(1)));

        assert_eq!(def.params[2].name, "enabled");
        assert_eq!(def.params[2].data_type, DataType::bool());
        assert!(matches!(def.params[2].default_value, Value::Bool(true)));

        assert_eq!(def.params[3].name, "label");
        assert_eq!(def.params[3].data_type, DataType::string());
        assert!(matches!(&def.params[3].default_value, Value::String(s) if s == "hello"));
    }

    #[test]
    fn test_empty_node() {
        let nm = crate::node_manager::NodeManager::from_inventory();
        let def = nm.get("test_empty").expect("test_empty should exist");
        assert!(def.inputs.is_empty());
        assert!(def.outputs.is_empty());
        assert!(def.params.is_empty());
    }

    #[test]
    fn test_builtin_nodes_collected() {
        let nm = crate::node_manager::NodeManager::from_inventory();
        assert!(nm.get("load_image").is_some(), "load_image not found");
        assert!(nm.get("save_image").is_some(), "save_image not found");
        assert!(nm.get("brightness").is_some(), "brightness not found");
        assert!(nm.get("contrast").is_some(), "contrast not found");
    }
}
