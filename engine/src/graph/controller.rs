use crate::graph::{validate, Connection, Graph, PinRef};
use crate::node_manager::NodeManager;
use std::collections::HashMap;
use std::sync::Arc;
use types::{NodeId, Value};

use super::model::batch::{ApplyBatchError, BatchNodeRef, BatchPinRef, BatchWorkResult, EditBatchRequest, EditBatchResult, GraphEdit};
use super::model::events::{
    GraphChange, GraphChangedEvent, GraphEvent, GraphEventKind, PreviewChangedEvent,
};
use super::model::state::GraphState;
use super::model::subgraph::{ExecuteTarget, SubgraphInfo};
use super::model::validation::ValidationReport;
use super::query::downstream::downstream;
use super::query::resolve_subgraph::resolve_subgraph;
use super::query::state_summary::state_summary;
use super::query::upstream::upstream;
use super::query::validate_graph::validate_graph;

pub struct GraphController {
    state: GraphState,
    node_manager: Arc<NodeManager>,
    events: Vec<GraphEvent>,
}

impl GraphController {
    pub fn new(node_manager: Arc<NodeManager>, max_undo: usize) -> Self {
        Self {
            state: GraphState::new(max_undo),
            node_manager,
            events: Vec::new(),
        }
    }

    pub fn current(&self) -> &Graph {
        self.state.current()
    }

    pub fn snapshot(&self) -> Arc<Graph> {
        self.state.snapshot()
    }

    pub fn graph_version(&self) -> u64 {
        self.state.graph_version()
    }

    pub fn is_dirty(&self) -> bool {
        self.state.is_dirty()
    }

    pub fn state_summary(&self) -> super::model::state::GraphStateSummary {
        state_summary(&self.state)
    }

    pub fn events_snapshot(&self) -> Vec<GraphEvent> {
        self.events.clone()
    }

    pub fn validate_graph(&self) -> ValidationReport {
        validate_graph(self.state.current(), &self.node_manager)
    }

    pub fn upstream(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut nodes: Vec<_> = upstream(self.state.current(), node_id).into_iter().collect();
        nodes.sort_by_key(|id| id.0);
        nodes
    }

    pub fn downstream(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut nodes: Vec<_> = downstream(self.state.current(), node_id).into_iter().collect();
        nodes.sort_by_key(|id| id.0);
        nodes
    }

    pub fn resolve_subgraph(&self, target: ExecuteTarget) -> Result<SubgraphInfo, String> {
        resolve_subgraph(self.state.current(), target).map_err(|err| err.to_string())
    }

    fn record_event(&mut self, kind: GraphEventKind) {
        self.events.push(GraphEvent::new(kind));
    }

    pub fn mark_saved(&mut self) {
        self.state.mark_saved();
        self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
            graph_version: self.state.graph_version(),
            dirty: self.state.is_dirty(),
            changes: Vec::new(),
        }));
    }

    pub fn discard_preview(&mut self) -> bool {
        let cleared = self.state.discard_preview();
        if let Some(target) = cleared {
            self.record_event(GraphEventKind::PreviewChanged(PreviewChangedEvent {
                node_id: target.node_id,
                param: target.param,
                has_preview: false,
            }));
            true
        } else {
            false
        }
    }

    pub fn add_node(&mut self, type_id: &str) -> Result<NodeId, String> {
        if self.node_manager.get_node_def(type_id).is_none() {
            return Err(format!("Unknown node type: {}", type_id));
        }
        let defaults = self
            .node_manager
            .default_params(type_id)
            .unwrap_or_default();
        let (graph, id) = self.state.current().add_node(type_id, defaults);
        self.state.commit(graph);
        self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
            graph_version: self.state.graph_version(),
            dirty: self.state.is_dirty(),
            changes: vec![GraphChange::NodeAdded { node_id: id }],
        }));
        Ok(id)
    }

    pub fn remove_node(&mut self, id: NodeId) -> Result<(), String> {
        if !self.state.current().nodes.contains_key(&id) {
            return Err(format!("Node {:?} not found", id));
        }
        let graph = self.state.current().remove_node(id);
        self.state.commit(graph);
        self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
            graph_version: self.state.graph_version(),
            dirty: self.state.is_dirty(),
            changes: vec![GraphChange::NodeRemoved { node_id: id }],
        }));
        Ok(())
    }

    pub fn connect(&mut self, conn: Connection) -> Result<(), validate::ConnectionError> {
        validate::validate_connection_basic(self.state.current(), &conn)?;
        validate::validate_formal_connection(&self.node_manager, self.state.current(), &conn)?;

        let replaced = self
            .state
            .current()
            .connections
            .iter()
            .find(|existing| existing.to == conn.to)
            .cloned();

        let added = GraphChange::ConnectionAdded {
            from: conn.from.clone(),
            to: conn.to.clone(),
        };
        let graph = self.state.current().connect(conn);
        self.state.commit(graph);
        let mut changes = Vec::new();
        if let Some(old) = replaced {
            changes.push(GraphChange::ConnectionRemoved {
                from: old.from,
                to: old.to,
            });
        }
        changes.push(added);
        self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
            graph_version: self.state.graph_version(),
            dirty: self.state.is_dirty(),
            changes,
        }));
        Ok(())
    }

    pub fn disconnect(&mut self, from: PinRef, to: PinRef) -> Result<(), validate::ConnectionError> {
        let conn = Connection {
            from: from.clone(),
            to: to.clone(),
        };
        validate::validate_disconnect_request(&self.node_manager, self.state.current(), &conn)?;

        let graph = self.state.current().disconnect(&from, &to);
        self.state.commit(graph);
        self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
            graph_version: self.state.graph_version(),
            dirty: self.state.is_dirty(),
            changes: vec![GraphChange::ConnectionRemoved { from, to }],
        }));
        Ok(())
    }

    pub fn set_param(&mut self, id: NodeId, key: &str, value: Value, preview: bool) {
        if preview {
            self.state.preview(id, key, value);
            self.record_event(GraphEventKind::PreviewChanged(PreviewChangedEvent {
                node_id: id,
                param: key.to_string(),
                has_preview: true,
            }));
        } else {
            let graph = self.state.current().set_param(id, key, value);
            let cleared_preview = self.state.preview_target().cloned();
            self.state.commit(graph);
            if let Some(target) = cleared_preview {
                self.record_event(GraphEventKind::PreviewChanged(PreviewChangedEvent {
                    node_id: target.node_id,
                    param: target.param,
                    has_preview: false,
                }));
            }
            self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
                graph_version: self.state.graph_version(),
                dirty: self.state.is_dirty(),
                changes: vec![GraphChange::ParamCommitted {
                    node_id: id,
                    param: key.to_string(),
                }],
            }));
        }
    }

    pub fn undo(&mut self) -> bool {
        let cleared_preview = self.state.preview_target().cloned();
        let undone = self.state.undo();
        if undone {
            if let Some(target) = cleared_preview {
                self.record_event(GraphEventKind::PreviewChanged(PreviewChangedEvent {
                    node_id: target.node_id,
                    param: target.param,
                    has_preview: false,
                }));
            }
            self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
                graph_version: self.state.graph_version(),
                dirty: self.state.is_dirty(),
                changes: vec![GraphChange::Undone],
            }));
        }
        undone
    }

    pub fn redo(&mut self) -> bool {
        let cleared_preview = self.state.preview_target().cloned();
        let redone = self.state.redo();
        if redone {
            if let Some(target) = cleared_preview {
                self.record_event(GraphEventKind::PreviewChanged(PreviewChangedEvent {
                    node_id: target.node_id,
                    param: target.param,
                    has_preview: false,
                }));
            }
            self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
                graph_version: self.state.graph_version(),
                dirty: self.state.is_dirty(),
                changes: vec![GraphChange::Redone],
            }));
        }
        redone
    }

    pub fn replace_graph(&mut self, graph: Graph) -> Result<(), ValidationReport> {
        let report = validate_graph(&graph, &self.node_manager);
        if !report.is_valid {
            return Err(report);
        }

        let cleared_preview = self.state.preview_target().cloned();
        self.state.replace(graph);
        if let Some(target) = cleared_preview {
            self.record_event(GraphEventKind::PreviewChanged(PreviewChangedEvent {
                node_id: target.node_id,
                param: target.param,
                has_preview: false,
            }));
        }
        self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
            graph_version: self.state.graph_version(),
            dirty: self.state.is_dirty(),
            changes: vec![GraphChange::Replaced],
        }));
        Ok(())
    }

    pub fn replace(&mut self, graph: Graph) {
        let _ = self.replace_graph(graph);
    }

    pub fn apply_batch(&mut self, req: EditBatchRequest) -> Result<EditBatchResult, ApplyBatchError> {
        let work = self.apply_batch_edits(req)?;
        let cleared_preview = self.state.preview_target().cloned();
        self.state.commit(work.graph);
        if let Some(target) = cleared_preview {
            self.record_event(GraphEventKind::PreviewChanged(PreviewChangedEvent {
                node_id: target.node_id,
                param: target.param,
                has_preview: false,
            }));
        }
        self.record_event(GraphEventKind::GraphChanged(GraphChangedEvent {
            graph_version: self.state.graph_version(),
            dirty: self.state.is_dirty(),
            changes: work.changes,
        }));

        Ok(EditBatchResult {
            created_nodes: work.created_nodes,
            graph_version: self.state.graph_version(),
        })
    }

    fn apply_batch_edits(&self, req: EditBatchRequest) -> Result<BatchWorkResult, ApplyBatchError> {
        let mut graph = self.state.current().clone();
        let mut created_nodes = HashMap::new();
        let mut changes = Vec::new();

        for edit in req.edits {
            match edit {
                GraphEdit::AddNode {
                    client_key,
                    type_id,
                    param_overrides,
                } => {
                    if self.node_manager.get_node_def(&type_id).is_none() {
                        return Err(ApplyBatchError::UnknownNodeType { type_id });
                    }
                    let mut defaults = self.node_manager.default_params(&type_id).unwrap_or_default();
                    for (key, value) in param_overrides {
                        defaults.insert(key, value);
                    }
                    let (next_graph, id) = graph.add_node(&type_id, defaults);
                    graph = next_graph;
                    created_nodes.insert(client_key, id);
                    changes.push(GraphChange::NodeAdded { node_id: id });
                }
                GraphEdit::RemoveNode { node_id } => {
                    if !graph.nodes.contains_key(&node_id) {
                        return Err(ApplyBatchError::NodeNotFound { node_id });
                    }
                    graph = graph.remove_node(node_id);
                    changes.push(GraphChange::NodeRemoved { node_id });
                }
                GraphEdit::Connect { from, to } => {
                    let conn = self.resolve_batch_connection(&created_nodes, from, to)?;
                    validate::validate_connection_basic(&graph, &conn)
                        .map_err(ApplyBatchError::ConnectionError)?;
                    validate::validate_formal_connection(&self.node_manager, &graph, &conn)
                        .map_err(ApplyBatchError::ConnectionError)?;
                    if let Some(old) = graph.connections.iter().find(|existing| existing.to == conn.to).cloned() {
                        changes.push(GraphChange::ConnectionRemoved {
                            from: old.from,
                            to: old.to,
                        });
                    }
                    graph = graph.connect(conn.clone());
                    changes.push(GraphChange::ConnectionAdded {
                        from: conn.from,
                        to: conn.to,
                    });
                }
                GraphEdit::Disconnect { from, to } => {
                    let conn = self.resolve_batch_connection(&created_nodes, from, to)?;
                    validate::validate_disconnect_request(&self.node_manager, &graph, &conn)
                        .map_err(ApplyBatchError::ConnectionError)?;
                    graph = graph.disconnect(&conn.from, &conn.to);
                    changes.push(GraphChange::ConnectionRemoved {
                        from: conn.from,
                        to: conn.to,
                    });
                }
                GraphEdit::SetParam {
                    node,
                    param,
                    value,
                    preview,
                } => {
                    if preview {
                        return Err(ApplyBatchError::PreviewNotAllowed);
                    }
                    let node_id = self.resolve_batch_node(&created_nodes, node)?;
                    if !graph.nodes.contains_key(&node_id) {
                        return Err(ApplyBatchError::NodeNotFound { node_id });
                    }
                    graph = graph.set_param(node_id, &param, value);
                    changes.push(GraphChange::ParamCommitted { node_id, param });
                }
            }
        }

        Ok(BatchWorkResult {
            graph,
            changes,
            created_nodes,
        })
    }

    fn resolve_batch_node(
        &self,
        created_nodes: &HashMap<String, NodeId>,
        node: BatchNodeRef,
    ) -> Result<NodeId, ApplyBatchError> {
        match node {
            BatchNodeRef::Existing(id) => Ok(id),
            BatchNodeRef::Created(key) => created_nodes
                .get(&key)
                .copied()
                .ok_or(ApplyBatchError::UnknownBatchNodeKey { key }),
        }
    }

    fn resolve_batch_connection(
        &self,
        created_nodes: &HashMap<String, NodeId>,
        from: BatchPinRef,
        to: BatchPinRef,
    ) -> Result<Connection, ApplyBatchError> {
        Ok(Connection {
            from: PinRef {
                node: self.resolve_batch_node(created_nodes, from.node)?,
                interface: from.interface,
            },
            to: PinRef {
                node: self.resolve_batch_node(created_nodes, to.node)?,
                interface: to.interface,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::{ExecutorType, NodeDef, NodeManager, ParamDef, ParamExpose, PinDef};
    use types::{Constraint, DataType, Value};

    fn make_manager() -> Arc<NodeManager> {
        let mut nm = NodeManager::new();
        nm.register(NodeDef {
            type_id: "src".into(),
            name: "src".into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            inputs: vec![],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "strength".into(),
                data_type: DataType::float(),
                constraint: Some(Constraint::range(0.0, 1.0)),
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Output],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        nm.register(NodeDef {
            type_id: "mid".into(),
            name: "mid".into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "strength".into(),
                data_type: DataType::float(),
                constraint: Some(Constraint::range(0.0, 1.0)),
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Input, ParamExpose::Output],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        nm.register(NodeDef {
            type_id: "dst".into(),
            name: "dst".into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            inputs: vec![PinDef {
                name: "image".into(),
                data_type: DataType::image(),
                optional: false,
            }],
            outputs: vec![],
            params: vec![ParamDef {
                name: "strength".into(),
                data_type: DataType::float(),
                constraint: Some(Constraint::range(0.0, 1.0)),
                default_value: Value::Float(0.0),
                expose: vec![ParamExpose::Input],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });
        Arc::new(nm)
    }

    #[test]
    fn validate_graph_reports_current_graph_state() {
        let mut gc = GraphController::new(make_manager(), 10);
        let a = gc.add_node("src").unwrap();
        let b = gc.add_node("dst").unwrap();
        gc.connect(Connection {
            from: PinRef {
                node: a,
                interface: "strength".into(),
            },
            to: PinRef {
                node: b,
                interface: "strength".into(),
            },
        })
        .unwrap();

        let report = gc.validate_graph();
        assert!(report.is_valid);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn validate_graph_reports_param_constraint_violation() {
        let mut gc = GraphController::new(make_manager(), 10);
        let a = gc.add_node("src").unwrap();
        gc.set_param(a, "strength", Value::Float(2.0), false);

        let report = gc.validate_graph();
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.code == super::super::model::validation::ValidationIssueCode::InvalidParamValue));
    }

    #[test]
    fn upstream_and_downstream_are_exposed_from_controller() {
        let mut gc = GraphController::new(make_manager(), 10);
        let a = gc.add_node("src").unwrap();
        let b = gc.add_node("mid").unwrap();
        let c = gc.add_node("dst").unwrap();

        gc.connect(Connection {
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

        gc.connect(Connection {
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

        assert_eq!(gc.upstream(c), vec![a, b]);
        assert_eq!(gc.downstream(a), vec![b, c]);
    }

    #[test]
    fn resolve_subgraph_is_exposed_from_controller() {
        let mut gc = GraphController::new(make_manager(), 10);
        let a = gc.add_node("src").unwrap();
        let b = gc.add_node("mid").unwrap();
        let c = gc.add_node("dst").unwrap();

        gc.connect(Connection {
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

        gc.connect(Connection {
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

        let info = gc.resolve_subgraph(ExecuteTarget::Node(c)).unwrap();
        assert_eq!(info.nodes, vec![a, b, c]);
        assert_eq!(info.order, vec![a, b, c]);
    }

    #[test]
    fn replace_graph_validates_before_committing() {
        let mut gc = GraphController::new(make_manager(), 10);
        let mut graph = Graph::new();
        let (g, a) = graph.add_node("src", Default::default());
        graph = g;
        let (g, b) = graph.add_node("dst", Default::default());
        graph = g.connect(Connection {
            from: PinRef {
                node: b,
                interface: "image".into(),
            },
            to: PinRef {
                node: a,
                interface: "image".into(),
            },
        });

        let result = gc.replace_graph(graph);
        assert!(result.is_err());
        assert_eq!(gc.current().nodes.len(), 0);
    }

    #[test]
    fn replace_graph_commits_valid_graph() {
        let mut gc = GraphController::new(make_manager(), 10);
        let g = Graph::new();
        let (g, a) = g.add_node("src", Default::default());
        let (g, b) = g.add_node("dst", Default::default());
        let graph = g.connect(Connection {
            from: PinRef {
                node: a,
                interface: "strength".into(),
            },
            to: PinRef {
                node: b,
                interface: "strength".into(),
            },
        });

        gc.replace_graph(graph).unwrap();
        assert_eq!(gc.current().nodes.len(), 2);
        assert_eq!(gc.current().connections.len(), 1);
    }

    #[test]
    fn apply_batch_commits_atomically() {
        let mut gc = GraphController::new(make_manager(), 10);
        let result = gc
            .apply_batch(EditBatchRequest {
                edits: vec![
                    GraphEdit::AddNode {
                        client_key: "a".into(),
                        type_id: "src".into(),
                        param_overrides: HashMap::new(),
                    },
                    GraphEdit::AddNode {
                        client_key: "b".into(),
                        type_id: "dst".into(),
                        param_overrides: HashMap::new(),
                    },
                    GraphEdit::Connect {
                        from: BatchPinRef {
                            node: BatchNodeRef::Created("a".into()),
                            interface: "strength".into(),
                        },
                        to: BatchPinRef {
                            node: BatchNodeRef::Created("b".into()),
                            interface: "strength".into(),
                        },
                    },
                ],
                label: None,
            })
            .unwrap();

        assert_eq!(gc.current().nodes.len(), 2);
        assert_eq!(gc.current().connections.len(), 1);
        assert_eq!(result.created_nodes.len(), 2);
    }

    #[test]
    fn apply_batch_rejects_preview_edit() {
        let mut gc = GraphController::new(make_manager(), 10);
        let node = gc.add_node("src").unwrap();

        let result = gc.apply_batch(EditBatchRequest {
            edits: vec![GraphEdit::SetParam {
                node: BatchNodeRef::Existing(node),
                param: "strength".into(),
                value: Value::Float(1.0),
                preview: true,
            }],
            label: None,
        });

        assert!(matches!(result, Err(ApplyBatchError::PreviewNotAllowed)));
    }

    #[test]
    fn disconnect_requires_existing_formal_connection() {
        let mut gc = GraphController::new(make_manager(), 10);
        let a = gc.add_node("src").unwrap();
        let b = gc.add_node("dst").unwrap();

        let result = gc.disconnect(
            PinRef {
                node: a,
                interface: "strength".into(),
            },
            PinRef {
                node: b,
                interface: "strength".into(),
            },
        );

        assert!(matches!(result, Err(validate::ConnectionError::ConnectionNotFound)));
    }
}
