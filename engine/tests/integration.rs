use engine::facade::EngineFacade;
use engine::graph::model::events::{GraphChange, GraphEventKind};
use engine::graph::GraphController;
use engine::graph::{Connection, PinRef};
use engine::node_manager::{NodeDef, NodeManager, PinDef};
use engine::Engine;
use std::sync::Arc;
use types::{DataType, Value};

// === 图操作测试 ===

#[test]
fn test_add_node_and_connect() {
    let mut engine = Engine::new(None);
    let load = engine.add_node("load_image").unwrap();
    let brightness = engine.add_node("brightness").unwrap();
    engine
        .connect(Connection {
            from: PinRef {
                node: load,
                interface: "image".into(),
            },
            to: PinRef {
                node: brightness,
                interface: "image".into(),
            },
        })
        .unwrap();
    assert_eq!(engine.query_graph_snapshot().connections.len(), 1);
}

#[test]
fn test_unknown_node_type_rejected() {
    let mut engine = Engine::new(None);
    let result = engine.add_node("nonexistent_node_type");
    assert!(result.is_err());
}

#[test]
fn test_cycle_detection() {
    let mut engine = Engine::new(None);
    let a = engine.add_node("brightness").unwrap();
    let b = engine.add_node("brightness").unwrap();
    engine
        .connect(Connection {
            from: PinRef {
                node: a,
                interface: "image".into(),
            },
            to: PinRef {
                node: b,
                interface: "image".into(),
            },
        })
        .unwrap();
    let result = engine.connect(Connection {
        from: PinRef {
            node: b,
            interface: "image".into(),
        },
        to: PinRef {
            node: a,
            interface: "image".into(),
        },
    });
    assert!(result.is_err());
}

#[test]
fn test_self_connection_rejected() {
    let mut engine = Engine::new(None);
    let a = engine.add_node("brightness").unwrap();
    let result = engine.connect(Connection {
        from: PinRef {
            node: a,
            interface: "image".into(),
        },
        to: PinRef {
            node: a,
            interface: "image".into(),
        },
    });
    assert!(result.is_err());
}

// === undo/redo 测试 ===

#[test]
fn test_undo_redo() {
    let mut engine = Engine::new(None);
    engine.add_node("load_image").unwrap();
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 1);
    engine.undo();
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 0);
    engine.redo();
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 1);
}

#[test]
fn test_preview_does_not_affect_undo() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    engine
        .set_param(id, "brightness", Value::Float(0.5), true); // preview
    engine.undo(); // should undo add_node, not set_param
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 0);
}

#[test]
fn test_multiple_undo_redo() {
    let mut engine = Engine::new(None);
    let a = engine.add_node("load_image").unwrap();
    let b = engine.add_node("brightness").unwrap();
    engine
        .connect(Connection {
            from: PinRef {
                node: a,
                interface: "image".into(),
            },
            to: PinRef {
                node: b,
                interface: "image".into(),
            },
        })
        .unwrap();
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 2);
    assert_eq!(engine.query_graph_snapshot().connections.len(), 1);

    engine.undo(); // undo connect
    assert_eq!(engine.query_graph_snapshot().connections.len(), 0);

    engine.undo(); // undo add brightness
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 1);

    engine.undo(); // undo add load_image
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 0);

    engine.redo(); // redo add load_image
    assert_eq!(engine.query_graph_snapshot().nodes.len(), 1);
}

#[test]
fn test_preview_sets_has_preview_in_summary() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();

    let before = engine.graph_state_summary();
    assert!(!before.has_preview);

    engine.set_param(id, "brightness", Value::Float(0.5), true);

    let after = engine.graph_state_summary();
    assert!(after.has_preview);
    assert!(after.dirty);
}

#[test]
fn test_discard_preview_clears_has_preview() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    engine
        .set_param(id, "brightness", Value::Float(0.5), true);
    assert!(engine.graph_state_summary().has_preview);

    assert!(engine.discard_preview());
    assert!(!engine.graph_state_summary().has_preview);
}

#[test]
fn test_commit_clears_preview() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    engine
        .set_param(id, "brightness", Value::Float(0.5), true);
    assert!(engine.graph_state_summary().has_preview);

    engine
        .set_param(id, "brightness", Value::Float(0.75), false);

    let summary = engine.graph_state_summary();
    assert!(!summary.has_preview);
    assert!(summary.dirty);
}

#[test]
fn test_undo_and_redo_clear_preview() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    engine
        .set_param(id, "brightness", Value::Float(0.5), true);
    assert!(engine.graph_state_summary().has_preview);

    engine.undo();
    assert!(!engine.graph_state_summary().has_preview);

    engine
        .set_param(id, "brightness", Value::Float(0.25), true);
    assert!(engine.graph_state_summary().has_preview);

    engine.redo();
    assert!(!engine.graph_state_summary().has_preview);
}

#[test]
fn test_replace_clears_preview_and_sets_dirty() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    engine
        .set_param(id, "brightness", Value::Float(0.5), true);
    assert!(engine.graph_state_summary().has_preview);

    engine.replace_graph(engine::graph::Graph::new()).unwrap();

    let summary = engine.graph_state_summary();
    assert!(!summary.has_preview);
    assert!(summary.dirty);
}

#[test]
fn test_mark_saved_updates_summary() {
    let mut engine = Engine::new(None);
    engine.add_node("brightness").unwrap();
    assert!(engine.graph_state_summary().dirty);

    engine.mark_saved();

    let summary = engine.graph_state_summary();
    assert!(!summary.dirty);
    assert_eq!(summary.graph_version, engine.graph_version());
}

#[test]
fn test_add_node_emits_graph_changed() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    let events = engine.graph_events_snapshot();
    let last = events.last().expect("missing event");

    match &last.kind {
        GraphEventKind::GraphChanged(event) => {
            assert!(
                matches!(event.changes.as_slice(), [GraphChange::NodeAdded { node_id }] if *node_id == id)
            );
        }
        other => panic!("expected GraphChanged, got {:?}", other),
    }
}

#[test]
fn test_preview_and_discard_emit_preview_events() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    engine
        .set_param(id, "brightness", Value::Float(0.5), true);
    engine.discard_preview();

    let events = engine.graph_events_snapshot();
    match &events[1].kind {
        GraphEventKind::PreviewChanged(event) => {
            assert_eq!(event.node_id, id);
            assert_eq!(event.param, "brightness");
            assert!(event.has_preview);
        }
        other => panic!("expected PreviewChanged, got {:?}", other),
    }
    match &events[2].kind {
        GraphEventKind::PreviewChanged(event) => {
            assert_eq!(event.node_id, id);
            assert_eq!(event.param, "brightness");
            assert!(!event.has_preview);
        }
        other => panic!("expected PreviewChanged, got {:?}", other),
    }
}

#[test]
fn test_mark_saved_emits_empty_graph_changed() {
    let mut engine = Engine::new(None);
    engine.add_node("brightness").unwrap();
    engine.mark_saved();

    let events = engine.graph_events_snapshot();
    let last = events.last().expect("missing event");
    match &last.kind {
        GraphEventKind::GraphChanged(event) => {
            assert!(event.changes.is_empty());
            assert!(!event.dirty);
        }
        other => panic!("expected GraphChanged, got {:?}", other),
    }
}

#[test]
fn test_connect_replacing_input_emits_remove_and_add() {
    let mut engine = Engine::new(None);
    let a = engine.add_node("load_image").unwrap();
    let b = engine.add_node("load_image").unwrap();
    let c = engine.add_node("brightness").unwrap();

    engine
        .connect(Connection {
            from: PinRef {
                node: a,
                interface: "image".into(),
            },
            to: PinRef {
                node: c,
                interface: "image".into(),
            },
        })
        .unwrap();

    engine
        .connect(Connection {
            from: PinRef {
                node: b,
                interface: "image".into(),
            },
            to: PinRef {
                node: c,
                interface: "image".into(),
            },
        })
        .unwrap();

    let events = engine.graph_events_snapshot();
    let last = events.last().expect("missing event");
    match &last.kind {
        GraphEventKind::GraphChanged(event) => {
            assert_eq!(event.changes.len(), 2);
            assert!(matches!(
                &event.changes[0],
                GraphChange::ConnectionRemoved { from, to }
                if from.node == a && to.node == c
            ));
            assert!(matches!(
                &event.changes[1],
                GraphChange::ConnectionAdded { from, to }
                if from.node == b && to.node == c
            ));
        }
        other => panic!("expected GraphChanged, got {:?}", other),
    }
}

// === inventory 测试 ===

#[test]
fn test_inventory_collects_all_builtins() {
    let nm = NodeManager::from_inventory();
    let expected = ["load_image", "save_image", "brightness", "contrast"];
    for name in &expected {
        assert!(
            nm.get_node_def(name).is_some(),
            "Missing builtin node: {}",
            name
        );
    }
}

#[test]
fn test_node_default_params() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("brightness").unwrap();
    let graph = engine.query_graph_snapshot();
    let node = graph.nodes.get(&id).unwrap();
    match node.params.get("brightness") {
        Some(Value::Float(v)) => assert_eq!(*v, 0.0),
        other => panic!("Expected Float(0.0), got {:?}", other),
    }
}

#[test]
fn test_image_gen_schema_exposes_provider_and_model_params() {
    let engine = Engine::new(None);
    let schema = engine
        .resolve_node_schema("image_gen", &std::collections::HashMap::new())
        .unwrap();
    let param_names = schema
        .params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<Vec<_>>();

    assert!(param_names.contains(&"provider"));
    assert!(param_names.contains(&"model"));
    assert!(param_names.contains(&"quality"));
    assert!(param_names.contains(&"ratio"));
}

// === 执行测试 ===

#[tokio::test]
async fn test_evaluate_gpu_node_without_gpu_returns_error() {
    let mut engine = Engine::new(None); // 无 GPU
    let id = engine.add_node("brightness").unwrap();
    let result = engine.evaluate(id).await;
    assert!(result.is_err(), "GPU node should fail without GPU");
}

#[tokio::test]
async fn test_evaluate_load_image_empty_path() {
    let mut engine = Engine::new(None);
    let id = engine.add_node("load_image").unwrap();
    // path is empty string (default), should return empty outputs
    let result = engine.evaluate(id).await;
    assert!(result.is_ok());
    let outputs = result.unwrap();
    let node_output = outputs.get(&id).unwrap();
    assert!(
        node_output.is_empty(),
        "Empty path should produce empty output"
    );
}

// === 类型兼容性测试 ===

#[test]
fn test_type_incompatible_connection_rejected() {
    let mut nm = NodeManager::new();
    nm.register(NodeDef {
        type_id: "float_source".into(),
        version: 1,
        source: engine::node_manager::NodeSourceKind::Builtin,
        name: "Float Source".into(),
        category: "test".into(),
        requires: vec!["test.float_source".into()],
        purity: engine::node_manager::Purity::Pure,
        cooking_sensitivity: vec![],
        realtime_capable: true,
        execution: engine::node_manager::ExecutionPolicy::default(),
        api: None,
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
        version: 1,
        source: engine::node_manager::NodeSourceKind::Builtin,
        name: "Image Sink".into(),
        category: "test".into(),
        requires: vec!["test.image_sink".into()],
        purity: engine::node_manager::Purity::Pure,
        cooking_sensitivity: vec![],
        realtime_capable: true,
        execution: engine::node_manager::ExecutionPolicy::default(),
        api: None,
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
    let a = gc.add_node("float_source").unwrap();
    let b = gc.add_node("image_sink").unwrap();

    let result = gc.connect(Connection {
        from: PinRef {
            node: a,
            interface: "value".into(),
        },
        to: PinRef {
            node: b,
            interface: "image".into(),
        },
    });
    assert!(
        result.is_err(),
        "Float -> Image connection should be rejected"
    );
}

// === 断开连接测试 ===

#[test]
fn test_disconnect() {
    let mut engine = Engine::new(None);
    let a = engine.add_node("load_image").unwrap();
    let b = engine.add_node("brightness").unwrap();
    engine
        .connect(Connection {
            from: PinRef {
                node: a,
                interface: "image".into(),
            },
            to: PinRef {
                node: b,
                interface: "image".into(),
            },
        })
        .unwrap();
    assert_eq!(engine.query_graph_snapshot().connections.len(), 1);

    let _ = engine.disconnect(
        PinRef {
            node: a,
            interface: "image".into(),
        },
        PinRef {
            node: b,
            interface: "image".into(),
        },
    );
    assert_eq!(engine.query_graph_snapshot().connections.len(), 0);
}
