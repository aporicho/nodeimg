use nodeimg_engine::_test_support::register_all;
use nodeimg_engine::NodeRegistry;

fn setup_registry() -> NodeRegistry {
    let mut reg = NodeRegistry::new();
    register_all(&mut reg);
    reg
}

#[test]
fn test_all_builtins_register_without_panic() {
    setup_registry();
}

#[test]
fn test_expected_node_count() {
    let reg = setup_registry();
    let all = reg.list(None);
    // 32 register() calls, but curves is a stub -> 31 actual nodes
    assert_eq!(all.len(), 31, "expected 31 builtin nodes, got {}", all.len());
}

#[test]
fn test_all_nodes_have_process_or_is_sink() {
    let reg = setup_registry();
    for def in reg.list(None) {
        // Every node must have at least one execution path
        assert!(
            def.process.is_some() || def.gpu_process.is_some(),
            "node '{}' has neither process nor gpu_process",
            def.type_id
        );
    }
}

#[test]
fn test_no_builtin_is_ai_node() {
    let reg = setup_registry();
    for def in reg.list(None) {
        assert!(
            !def.is_ai_node(),
            "builtin '{}' reports is_ai_node()=true",
            def.type_id
        );
    }
}

#[test]
fn test_instantiate_all_builtins_with_defaults() {
    let reg = setup_registry();
    for def in reg.list(None) {
        let instance = reg.instantiate(&def.type_id);
        assert!(
            instance.is_some(),
            "failed to instantiate '{}'",
            def.type_id
        );
        let instance = instance.unwrap();
        assert_eq!(
            instance.params.len(),
            def.params.len(),
            "node '{}': param count mismatch after instantiate",
            def.type_id
        );
    }
}
