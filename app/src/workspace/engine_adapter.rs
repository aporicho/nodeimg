use crate::panels::EnginePanelState;
use crate::workspace::node_palette::{NodePaletteItem, NodePaletteState};
use engine::events::ExecutionStatus;
use engine::facade::EngineFacade;
use engine::Engine;

pub(crate) fn engine_panel_state(engine: &Engine, last_action: &str) -> EnginePanelState {
    let graph = engine.query_graph_snapshot();
    let summary = engine.graph_state_summary();
    let execution_status = match engine.execution_state().status {
        ExecutionStatus::Idle => "Idle",
        ExecutionStatus::Running => "Running",
        ExecutionStatus::Cancelling => "Cancelling",
    };

    EnginePanelState {
        node_count: graph.nodes.len(),
        connection_count: graph.connections.len(),
        node_def_count: engine.list_node_defs().len(),
        graph_version: summary.graph_version,
        dirty: summary.dirty,
        execution_status: execution_status.to_string(),
        last_action: last_action.to_string(),
    }
}

pub(crate) fn node_palette_state(engine: &Engine) -> NodePaletteState {
    let mut items = engine
        .list_node_defs()
        .into_iter()
        .map(|node| NodePaletteItem {
            type_id: node.type_id.clone(),
            name: node.name.clone(),
            category: node.category.clone(),
            source: format!("{:?}", node.source),
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.type_id.cmp(&b.type_id))
    });

    NodePaletteState { items }
}
