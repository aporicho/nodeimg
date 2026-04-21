use crate::panels::EnginePanelState;
use crate::workspace::node_palette::{NodePaletteItem, NodePaletteState};
use engine::events::ExecutionStatus;
use engine::facade::EngineFacade;
use engine::Engine;
use gui::canvas::node_card::{CanvasNodeParamView, CanvasNodeView};
use gui::canvas::{
    canvas_port_stable_id, CanvasConnectionView, CanvasNodeIdentity, CanvasNodeLayout,
    CanvasPortConnectionState, CanvasPortSide, CanvasPortView,
};
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
        .map(|def| (def.type_id.as_str(), def))
        .collect::<HashMap<_, _>>();

    let mut views = layouts
        .into_iter()
        .filter_map(|layout| {
            let node_id = parse_engine_node_owner_id(&layout.owner_id)?;
            let node = graph.nodes.get(&node_id)?;
            let node_def = defs_by_type.get(node.type_id.as_str()).copied();
            let title = node_def
                .map(|def| def.name.as_str())
                .unwrap_or(node.type_id.as_str())
                .to_string();
            let inputs = node_def
                .map(|def| canvas_ports(&layout.owner_id, CanvasPortSide::Input, &def.inputs))
                .unwrap_or_default();
            let outputs = node_def
                .map(|def| canvas_ports(&layout.owner_id, CanvasPortSide::Output, &def.outputs))
                .unwrap_or_default();
            let category = node_def
                .map(|def| def.category.clone())
                .unwrap_or_else(|| "node".to_string());
            let params = node_def.map(canvas_node_params).unwrap_or_default();

            Some(CanvasNodeView {
                owner_id: layout.owner_id.clone(),
                title,
                subtitle: node.type_id.clone(),
                category,
                params,
                input_group: gui::canvas::CanvasPortGroupView::default(),
                output_group: gui::canvas::CanvasPortGroupView::default(),
                inputs,
                outputs,
                selected: false,
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

pub(crate) fn canvas_connection_views(engine: &Engine) -> Vec<CanvasConnectionView> {
    let graph = engine.query_graph_snapshot();
    graph
        .connections
        .iter()
        .filter_map(|connection| {
            let from_owner_id = engine_node_owner_id(connection.from.node);
            let to_owner_id = engine_node_owner_id(connection.to.node);
            Some(CanvasConnectionView {
                from_port_id: canvas_port_stable_id(
                    &from_owner_id,
                    CanvasPortSide::Output,
                    &connection.from.interface,
                ),
                to_port_id: canvas_port_stable_id(
                    &to_owner_id,
                    CanvasPortSide::Input,
                    &connection.to.interface,
                ),
            })
        })
        .collect()
}

fn canvas_ports(
    owner_id: &str,
    side: CanvasPortSide,
    pins: &[engine::node_manager::PinDef],
) -> Vec<CanvasPortView> {
    pins.iter()
        .enumerate()
        .map(|(index, pin)| CanvasPortView {
            name: pin.name.clone(),
            stable_id: canvas_port_stable_id(owner_id, side, &pin.name),
            side,
            index,
            count: pins.len(),
            connection_state: CanvasPortConnectionState::Idle,
        })
        .collect()
}

fn canvas_node_params(node_def: &engine::node_manager::NodeDef) -> Vec<CanvasNodeParamView> {
    node_def
        .params
        .iter()
        .filter(|param| {
            param
                .expose
                .iter()
                .any(|expose| matches!(expose, engine::node_manager::ParamExpose::Control))
        })
        .take(5)
        .map(|param| CanvasNodeParamView {
            name: param.name.clone(),
            kind: param.data_type.to_string(),
            value: compact_value(&param.default_value),
        })
        .collect()
}

fn compact_value(value: &types::Value) -> String {
    match value {
        types::Value::Float(value) => format!("{value:.2}"),
        types::Value::Int(value) => value.to_string(),
        types::Value::Bool(value) => value.to_string(),
        types::Value::Color([r, g, b, _]) => format!("#{r:.1}{g:.1}{b:.1}"),
        types::Value::String(value) => {
            if value.is_empty() {
                "text".to_string()
            } else {
                value.chars().take(16).collect()
            }
        }
        types::Value::Image(_) => "image".to_string(),
        types::Value::Handle(handle) => handle.data_type.to_string(),
    }
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

pub(crate) fn parse_engine_node_owner_id(owner_id: &str) -> Option<types::NodeId> {
    let id = owner_id.strip_prefix("engine_node::")?.parse().ok()?;
    Some(types::NodeId(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::graph::{Connection, PinRef};

    #[test]
    fn engine_node_owner_id_roundtrips() {
        let owner_id = engine_node_owner_id(types::NodeId(42));

        assert_eq!(
            parse_engine_node_owner_id(&owner_id),
            Some(types::NodeId(42))
        );
        assert_eq!(parse_engine_node_owner_id("panel::42"), None);
    }

    #[test]
    fn canvas_connection_views_use_port_stable_ids() {
        let mut engine = Engine::new(None);
        let generator = engine.add_node("image_gen").unwrap();
        let color_adjust = engine.add_node("color_adjust").unwrap();
        engine
            .connect(Connection {
                from: PinRef {
                    node: generator,
                    interface: "image".to_string(),
                },
                to: PinRef {
                    node: color_adjust,
                    interface: "image".to_string(),
                },
            })
            .unwrap();

        let connections = canvas_connection_views(&engine);

        assert_eq!(connections.len(), 1);
        assert_eq!(
            connections[0].from_port_id,
            format!(
                "canvas_node::engine_node::{}::port::output::image",
                generator.0
            )
        );
        assert_eq!(
            connections[0].to_port_id,
            format!(
                "canvas_node::engine_node::{}::port::input::image",
                color_adjust.0
            )
        );
    }
}
