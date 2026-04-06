use engine::Engine;
use engine::graph::Connection;
use engine::graph_controller::GraphController;
use engine::registry::{NodeDef, NodeManager, PinDef};
use types::{DataType, Value, Vec2};
use std::sync::Arc;

// === 图操作测试 ===

#[test]
fn test_add_node_and_connect() {
    let mut engine = Engine::new(None);
    let load = engine.graph.add_node("load_image", Vec2::default()).unwrap();
    let brightness = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    engine.graph.connect(Connection {
        from_node: load, from_pin: "image".into(),
        to_node: brightness, to_pin: "image".into(),
    }).unwrap();
    assert_eq!(engine.graph.current().connections.len(), 1);
}

#[test]
fn test_unknown_node_type_rejected() {
    let mut engine = Engine::new(None);
    let result = engine.graph.add_node("nonexistent_node_type", Vec2::default());
    assert!(result.is_err());
}

#[test]
fn test_cycle_detection() {
    let mut engine = Engine::new(None);
    let a = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    let b = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    engine.graph.connect(Connection {
        from_node: a, from_pin: "image".into(),
        to_node: b, to_pin: "image".into(),
    }).unwrap();
    let result = engine.graph.connect(Connection {
        from_node: b, from_pin: "image".into(),
        to_node: a, to_pin: "image".into(),
    });
    assert!(result.is_err());
}

#[test]
fn test_self_connection_rejected() {
    let mut engine = Engine::new(None);
    let a = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    let result = engine.graph.connect(Connection {
        from_node: a, from_pin: "image".into(),
        to_node: a, to_pin: "image".into(),
    });
    assert!(result.is_err());
}

// === undo/redo 测试 ===

#[test]
fn test_undo_redo() {
    let mut engine = Engine::new(None);
    engine.graph.add_node("load_image", Vec2::default()).unwrap();
    assert_eq!(engine.graph.current().nodes.len(), 1);
    engine.graph.undo();
    assert_eq!(engine.graph.current().nodes.len(), 0);
    engine.graph.redo();
    assert_eq!(engine.graph.current().nodes.len(), 1);
}

#[test]
fn test_preview_does_not_affect_undo() {
    let mut engine = Engine::new(None);
    let id = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    engine.graph.set_param(id, "brightness", Value::Float(0.5), true); // preview
    engine.graph.undo(); // should undo add_node, not set_param
    assert_eq!(engine.graph.current().nodes.len(), 0);
}

#[test]
fn test_multiple_undo_redo() {
    let mut engine = Engine::new(None);
    let a = engine.graph.add_node("load_image", Vec2::default()).unwrap();
    let b = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    engine.graph.connect(Connection {
        from_node: a, from_pin: "image".into(),
        to_node: b, to_pin: "image".into(),
    }).unwrap();
    assert_eq!(engine.graph.current().nodes.len(), 2);
    assert_eq!(engine.graph.current().connections.len(), 1);

    engine.graph.undo(); // undo connect
    assert_eq!(engine.graph.current().connections.len(), 0);

    engine.graph.undo(); // undo add brightness
    assert_eq!(engine.graph.current().nodes.len(), 1);

    engine.graph.undo(); // undo add load_image
    assert_eq!(engine.graph.current().nodes.len(), 0);

    engine.graph.redo(); // redo add load_image
    assert_eq!(engine.graph.current().nodes.len(), 1);
}

// === inventory 测试 ===

#[test]
fn test_inventory_collects_all_builtins() {
    let nm = NodeManager::from_inventory();
    let expected = ["load_image", "save_image", "brightness", "contrast"];
    for name in &expected {
        assert!(nm.get(name).is_some(), "Missing builtin node: {}", name);
    }
}

#[test]
fn test_node_default_params() {
    let mut engine = Engine::new(None);
    let id = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    let node = engine.graph.current().nodes.get(&id).unwrap();
    match node.params.get("brightness") {
        Some(Value::Float(v)) => assert_eq!(*v, 0.0),
        other => panic!("Expected Float(0.0), got {:?}", other),
    }
}

// === 执行测试 ===

#[tokio::test]
async fn test_evaluate_gpu_node_without_gpu_returns_error() {
    let mut engine = Engine::new(None); // 无 GPU
    let id = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    let result = engine.evaluate(id).await;
    assert!(result.is_err(), "GPU node should fail without GPU");
}

#[tokio::test]
async fn test_evaluate_load_image_empty_path() {
    let mut engine = Engine::new(None);
    let id = engine.graph.add_node("load_image", Vec2::default()).unwrap();
    // path is empty string (default), should return empty outputs
    let result = engine.evaluate(id).await;
    assert!(result.is_ok());
    let outputs = result.unwrap();
    let node_output = outputs.get(&id).unwrap();
    assert!(node_output.is_empty(), "Empty path should produce empty output");
}

// === 类型兼容性测试 ===

#[test]
fn test_type_incompatible_connection_rejected() {
    let mut nm = NodeManager::new();
    nm.register(NodeDef {
        type_id: "float_source".into(),
        name: "Float Source".into(),
        category: "test".into(),
        inputs: vec![],
        outputs: vec![PinDef {
            name: "value".into(),
            data_type: DataType::float(),
            optional: false,
        }],
        params: vec![],
        execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
    });
    nm.register(NodeDef {
        type_id: "image_sink".into(),
        name: "Image Sink".into(),
        category: "test".into(),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataType::image(),
            optional: false,
        }],
        outputs: vec![],
        params: vec![],
        execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
    });

    let mut gc = GraphController::new(Arc::new(nm), 50);
    let a = gc.add_node("float_source", Vec2::default()).unwrap();
    let b = gc.add_node("image_sink", Vec2::default()).unwrap();

    let result = gc.connect(Connection {
        from_node: a, from_pin: "value".into(),
        to_node: b, to_pin: "image".into(),
    });
    assert!(result.is_err(), "Float -> Image connection should be rejected");
}

// === 断开连接测试 ===

#[test]
fn test_disconnect() {
    let mut engine = Engine::new(None);
    let a = engine.graph.add_node("load_image", Vec2::default()).unwrap();
    let b = engine.graph.add_node("brightness", Vec2::default()).unwrap();
    engine.graph.connect(Connection {
        from_node: a, from_pin: "image".into(),
        to_node: b, to_pin: "image".into(),
    }).unwrap();
    assert_eq!(engine.graph.current().connections.len(), 1);

    engine.graph.disconnect(a, "image", b, "image");
    assert_eq!(engine.graph.current().connections.len(), 0);
}
