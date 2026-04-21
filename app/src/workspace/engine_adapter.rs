use crate::panels::EnginePanelState;
use crate::workspace::node_palette::{NodePaletteItem, NodePaletteState};
use engine::events::ExecutionStatus;
use engine::facade::EngineFacade;
use engine::Engine;
use gui::canvas::node_card::CanvasNodeView;
use gui::canvas::{CanvasNodeIdentity, CanvasNodeLayout};
use gui::renderer::Rect;
use std::collections::HashMap;

const CANVAS_NODE_WIDTH: f32 = 220.0;
const CANVAS_NODE_HEIGHT: f32 = 96.0;
const CANVAS_NODE_COLUMN_GAP: f32 = 280.0;
const CANVAS_NODE_ROW_GAP: f32 = 140.0;
const CANVAS_NODE_COLUMNS: usize = 3;

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

pub(crate) fn canvas_node_identities(engine: &Engine) -> Vec<CanvasNodeIdentity> {
    let graph = engine.query_graph_snapshot();
    let mut node_ids = graph.nodes.keys().copied().collect::<Vec<_>>();
    node_ids.sort_by_key(|id| id.0);

    node_ids
        .into_iter()
        .enumerate()
        .map(|(index, node_id)| CanvasNodeIdentity {
            owner_id: engine_node_owner_id(node_id),
            default_rect: default_canvas_node_rect(index),
        })
        .collect()
}

pub(crate) fn canvas_node_views(
    engine: &Engine,
    layouts: Vec<CanvasNodeLayout>,
) -> Vec<CanvasNodeView> {
    let graph = engine.query_graph_snapshot();
    let defs_by_type = engine
        .list_node_defs()
        .into_iter()
        .map(|def| (def.type_id.as_str(), def.name.as_str()))
        .collect::<HashMap<_, _>>();

    let mut views = layouts
        .into_iter()
        .filter_map(|layout| {
            let node_id = parse_engine_node_owner_id(&layout.owner_id)?;
            let node = graph.nodes.get(&node_id)?;
            let title = defs_by_type
                .get(node.type_id.as_str())
                .copied()
                .unwrap_or(node.type_id.as_str())
                .to_string();

            Some(CanvasNodeView {
                owner_id: layout.owner_id.clone(),
                title,
                subtitle: node.type_id.clone(),
                layout,
            })
        })
        .collect::<Vec<_>>();

    views.sort_by(|a, b| {
        a.layout
            .z_index
            .cmp(&b.layout.z_index)
            .then_with(|| a.owner_id.cmp(&b.owner_id))
    });
    views
}

fn default_canvas_node_rect(index: usize) -> Rect {
    let column = index % CANVAS_NODE_COLUMNS;
    let row = index / CANVAS_NODE_COLUMNS;
    Rect {
        x: column as f32 * CANVAS_NODE_COLUMN_GAP,
        y: row as f32 * CANVAS_NODE_ROW_GAP,
        w: CANVAS_NODE_WIDTH,
        h: CANVAS_NODE_HEIGHT,
    }
}

fn engine_node_owner_id(node_id: types::NodeId) -> String {
    format!("engine_node::{}", node_id.0)
}

fn parse_engine_node_owner_id(owner_id: &str) -> Option<types::NodeId> {
    let id = owner_id.strip_prefix("engine_node::")?.parse().ok()?;
    Some(types::NodeId(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_node_owner_id_roundtrips() {
        let owner_id = engine_node_owner_id(types::NodeId(42));

        assert_eq!(
            parse_engine_node_owner_id(&owner_id),
            Some(types::NodeId(42))
        );
        assert_eq!(parse_engine_node_owner_id("panel::42"), None);
    }
}
