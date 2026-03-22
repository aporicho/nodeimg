//! Asynchronous AI backend execution via background threads.
//!
//! `AiExecutor` spawns HTTP requests on background threads and delivers
//! results back to the UI thread through an `mpsc` channel, keeping the
//! GUI responsive during long-running AI operations (2+ minutes).

use crate::backend::BackendClient;
use crate::cache::{Cache, NodeId};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

/// Result delivered from a background AI execution thread.
pub struct AiResult {
    /// The preview/trigger node that requested this execution.
    pub trigger_node: NodeId,
    /// The AI output node whose subgraph was executed.
    pub ai_node_id: NodeId,
    /// All AI node IDs in the subgraph (extracted from graph_json at spawn time).
    /// Used to mark all intermediate nodes as cached so they're not re-submitted.
    pub all_ai_node_ids: HashSet<NodeId>,
    /// Generation counter — used to discard stale results after param changes.
    pub generation: u64,
    /// The backend response (or error).
    pub result: Result<serde_json::Value, String>,
}

/// Manages background AI execution threads.
pub struct AiExecutor {
    tx: mpsc::Sender<AiResult>,
    rx: mpsc::Receiver<AiResult>,
    /// Maps trigger_node -> current generation. Bumped on cancel.
    running: HashMap<NodeId, u64>,
}

impl Default for AiExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl AiExecutor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            running: HashMap::new(),
        }
    }

    /// Spawn a background thread to execute an AI subgraph.
    ///
    /// `on_complete` is called when the thread finishes so the UI layer can
    /// trigger a repaint (e.g. `ctx.request_repaint()`).
    pub fn spawn(
        &mut self,
        trigger_node: NodeId,
        ai_node_id: NodeId,
        graph_json: serde_json::Value,
        client: BackendClient,
        on_complete: Box<dyn Fn() + Send + Sync>,
    ) {
        let generation = self.running.get(&trigger_node).copied().unwrap_or(0);
        self.running.insert(trigger_node, generation);

        // Extract all AI node IDs from graph_json so we can mark them
        // all as cached when the result arrives (prevents re-submission).
        let all_ai_node_ids: HashSet<NodeId> = graph_json
            .get("nodes")
            .and_then(|n| n.as_object())
            .map(|map| {
                map.keys()
                    .filter_map(|k| k.parse::<NodeId>().ok())
                    .collect()
            })
            .unwrap_or_default();

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            eprintln!("[backend] AI execution started (output node {}, {} nodes in subgraph)", ai_node_id, all_ai_node_ids.len());
            let start = std::time::Instant::now();
            let result = client.execute_graph(&graph_json);
            let elapsed = start.elapsed();
            match &result {
                Ok(_) => eprintln!("[backend] AI execution completed in {:.1}s", elapsed.as_secs_f64()),
                Err(e) => eprintln!("[backend] AI execution failed after {:.1}s: {}", elapsed.as_secs_f64(), e),
            }
            let _ = tx.send(AiResult {
                trigger_node,
                ai_node_id,
                all_ai_node_ids,
                generation,
                result,
            });
            on_complete();
        });
    }

    /// Non-blocking poll: drain all completed results from the channel.
    ///
    /// For each valid (non-stale) result, parses the backend response,
    /// invalidates stale downstream caches, and inserts the fresh output.
    ///
    /// Returns `(completed_triggers, errors)`:
    /// - `completed_triggers`: trigger node IDs that received successful results
    ///   (caller should clear their textures so new images get rendered)
    /// - `errors`: `(trigger_node, message)` pairs for failed executions
    pub fn poll_results(
        &mut self,
        cache: &mut Cache,
    ) -> (Vec<NodeId>, Vec<(NodeId, String)>) {
        let mut completed = Vec::new();
        let mut errors = Vec::new();
        while let Ok(ai_result) = self.rx.try_recv() {
            let current_gen = self.running.get(&ai_result.trigger_node).copied();

            // Discard stale results (generation mismatch = params changed).
            // Still remove from running so a new spawn can happen.
            if current_gen != Some(ai_result.generation) {
                self.running.remove(&ai_result.trigger_node);
                continue;
            }

            // Task completed — remove from running map
            self.running.remove(&ai_result.trigger_node);

            match ai_result.result {
                Ok(response) => {
                    // Check for error response from backend
                    if let Some(error) = response.get("error").and_then(|v| v.as_str()) {
                        let failed_node = response
                            .get("failed_node")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let msg = format!("Backend error at node {}: {}", failed_node, error);
                        eprintln!("[backend] {}", msg);
                        errors.push((ai_result.trigger_node, msg));
                        continue;
                    }

                    match BackendClient::parse_backend_response(
                        &response,
                        ai_result.ai_node_id,
                    ) {
                        Ok(outputs) => {
                            eprintln!(
                                "[backend] Received {} output(s) for node {}",
                                outputs.len(),
                                ai_result.ai_node_id
                            );
                            // Invalidate the output AI node and all downstream
                            // (e.g. Preview) so they re-evaluate with fresh output.
                            cache.invalidate(ai_result.ai_node_id);
                            cache.insert(ai_result.ai_node_id, outputs);

                            // Mark ALL intermediate AI nodes in the subgraph as
                            // cached (with empty placeholder). This prevents
                            // pending_ai_execution from re-submitting them.
                            for &nid in &ai_result.all_ai_node_ids {
                                if nid != ai_result.ai_node_id && cache.get(nid).is_none() {
                                    cache.insert(nid, HashMap::new());
                                }
                            }

                            completed.push(ai_result.trigger_node);
                        }
                        Err(e) => {
                            eprintln!("[backend] Failed to parse response: {}", e);
                            errors.push((ai_result.trigger_node, e));
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[backend] Execution failed: {}", e);
                    errors.push((ai_result.trigger_node, e));
                }
            }
        }
        (completed, errors)
    }

    /// Check if a background task is running for the given trigger node.
    pub fn is_running(&self, trigger_node: NodeId) -> bool {
        self.running.contains_key(&trigger_node)
    }

    /// Cancel a running task by bumping its generation counter.
    ///
    /// The background thread is not killed — its result will simply be
    /// discarded when it arrives because the generation won't match.
    pub fn cancel(&mut self, trigger_node: NodeId) {
        if let Some(gen) = self.running.get_mut(&trigger_node) {
            *gen += 1;
        }
    }

    /// Cancel ALL running tasks. Used when any node's parameters change,
    /// since we don't track which trigger depends on which AI node.
    pub fn cancel_all(&mut self) {
        for gen in self.running.values_mut() {
            *gen += 1;
        }
    }
}
