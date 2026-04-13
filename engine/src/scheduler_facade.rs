use crate::execution::CookingContextRange;
use crate::facade::{EngineError, ExecuteRequest, ExecutionTicket};
use crate::graph;
use crate::graph::model::subgraph::ExecuteTarget;
use crate::graph::query::downstream::downstream;
use crate::runtime::Runtime;
use std::collections::{HashMap, HashSet};
use types::{NodeId, Value};

#[derive(Default)]
struct DirtyStateStub {
    graph_dirty: bool,
    pending_dirty_roots: HashSet<NodeId>,
}

pub struct SchedulerFacade {
    runtime: Runtime,
    cooking_range: CookingContextRange,
    dirty: DirtyStateStub,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Connection, Graph, PinRef};

    #[test]
    fn collect_dirty_nodes_expands_downstream_for_graph_target() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let (g, c) = g.add_node("c", Default::default());
        let g = g.connect(Connection {
            from: PinRef {
                node: a,
                interface: "out".into(),
            },
            to: PinRef {
                node: b,
                interface: "in".into(),
            },
        });
        let g = g.connect(Connection {
            from: PinRef {
                node: b,
                interface: "out".into(),
            },
            to: PinRef {
                node: c,
                interface: "in".into(),
            },
        });

        let mut scheduler = SchedulerFacade {
            runtime: crate::runtime::Runtime::new(
                std::sync::Arc::new(crate::node_manager::NodeManager::new()),
                std::sync::Arc::new(crate::capability::CapabilityRegistry::new()),
                crate::executors::image::ImageExecutor::new(None),
                crate::cache::manager::CacheManager::new(),
            ),
            cooking_range: CookingContextRange::default(),
            dirty: DirtyStateStub::default(),
        };
        scheduler.mark_dirty(a);

        let dirty = scheduler.collect_dirty_nodes(&g, &ExecuteTarget::Graph);
        assert_eq!(dirty, [a, b, c].into_iter().collect());
    }

    #[test]
    fn collect_dirty_nodes_intersects_with_node_target_closure() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let (g, c) = g.add_node("c", Default::default());
        let (g, d) = g.add_node("d", Default::default());
        let g = g.connect(Connection {
            from: PinRef {
                node: a,
                interface: "out".into(),
            },
            to: PinRef {
                node: b,
                interface: "in".into(),
            },
        });
        let g = g.connect(Connection {
            from: PinRef {
                node: b,
                interface: "out".into(),
            },
            to: PinRef {
                node: c,
                interface: "in".into(),
            },
        });

        let mut scheduler = SchedulerFacade {
            runtime: crate::runtime::Runtime::new(
                std::sync::Arc::new(crate::node_manager::NodeManager::new()),
                std::sync::Arc::new(crate::capability::CapabilityRegistry::new()),
                crate::executors::image::ImageExecutor::new(None),
                crate::cache::manager::CacheManager::new(),
            ),
            cooking_range: CookingContextRange::default(),
            dirty: DirtyStateStub::default(),
        };
        scheduler.mark_dirty(a);
        scheduler.mark_dirty(d);

        let dirty = scheduler.collect_dirty_nodes(&g, &ExecuteTarget::Node(c));
        assert_eq!(dirty, [a, b, c].into_iter().collect());
        assert!(!dirty.contains(&d));
    }
}

impl SchedulerFacade {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            cooking_range: CookingContextRange::default(),
            dirty: DirtyStateStub::default(),
        }
    }

    pub fn graph_changed(&mut self) {
        self.dirty.graph_dirty = true;
    }

    pub fn mark_dirty(&mut self, node_id: NodeId) {
        self.dirty.pending_dirty_roots.insert(node_id);
    }

    pub fn set_cooking_range(&mut self, range: CookingContextRange) {
        self.cooking_range = range;
    }

    pub async fn execute_request(
        &mut self,
        graph: &graph::Graph,
        request: ExecuteRequest,
    ) -> Result<ExecutionTicket, EngineError> {
        let dirty_nodes = self.collect_dirty_nodes(graph, &request.target);
        self.runtime.invalidate_nodes(&dirty_nodes);
        self.dirty.graph_dirty = false;
        self.dirty.pending_dirty_roots.clear();
        self.runtime
            .execute_request(graph, &self.cooking_range, request)
            .await
    }

    pub fn get_execution_outputs(
        &self,
        execution_id: crate::execution::ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError> {
        self.runtime.get_execution_outputs(execution_id, node_id)
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    fn collect_dirty_nodes(
        &self,
        graph: &graph::Graph,
        target: &ExecuteTarget,
    ) -> HashSet<NodeId> {
        if self.dirty.graph_dirty {
            return graph.nodes.keys().copied().collect();
        }

        let mut dirty = HashSet::new();
        for root in &self.dirty.pending_dirty_roots {
            dirty.insert(*root);
            dirty.extend(downstream(graph, *root));
        }

        match target {
            ExecuteTarget::Graph => dirty,
            ExecuteTarget::Node(node_id) => {
                let target_closure = {
                    let mut set = HashSet::from([*node_id]);
                    set.extend(crate::graph::query::resolve_subgraph::resolve_subgraph(
                        graph,
                        ExecuteTarget::Node(*node_id),
                    )
                    .map(|info| info.nodes)
                    .unwrap_or_default());
                    set
                };
                dirty
                    .into_iter()
                    .filter(|node_id| target_closure.contains(node_id))
                    .collect()
            }
        }
    }
}
