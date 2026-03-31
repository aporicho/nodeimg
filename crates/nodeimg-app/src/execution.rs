use nodeimg_engine::transport::{ExecuteProgress, GraphRequest, ProcessingTransport};
use nodeimg_engine::NodeId;
use std::sync::mpsc;
use std::sync::Arc;
use std::collections::HashMap;

struct TaggedSender {
    gen: u64,
    trigger: NodeId,
    tx: mpsc::Sender<(NodeId, u64, ExecuteProgress)>,
}

impl TaggedSender {
    fn into_sender(self) -> mpsc::Sender<ExecuteProgress> {
        let (inner_tx, inner_rx) = mpsc::channel();
        let tagged_tx = self.tx;
        let gen = self.gen;
        let trigger = self.trigger;
        std::thread::spawn(move || {
            while let Ok(progress) = inner_rx.recv() {
                if tagged_tx.send((trigger, gen, progress)).is_err() {
                    break;
                }
            }
        });
        inner_tx
    }
}

struct TaskState {
    generation: u64,
}

pub struct ExecutionManager {
    transport: Arc<dyn ProcessingTransport>,
    progress_tx: mpsc::Sender<(NodeId, u64, ExecuteProgress)>,
    progress_rx: mpsc::Receiver<(NodeId, u64, ExecuteProgress)>,
    tasks: HashMap<NodeId, TaskState>,
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl ExecutionManager {
    pub fn new(transport: Arc<dyn ProcessingTransport>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            transport,
            progress_tx: tx,
            progress_rx: rx,
            tasks: HashMap::new(),
            repaint: None,
        }
    }

    pub fn set_repaint_callback(&mut self, cb: impl Fn() + Send + Sync + 'static) {
        self.repaint = Some(Arc::new(cb));
    }

    pub fn submit(&mut self, trigger_node: NodeId, request: GraphRequest) {
        let gen = self.tasks
            .get(&trigger_node)
            .map(|s| s.generation + 1)
            .unwrap_or(1);
        self.tasks.insert(trigger_node, TaskState { generation: gen });

        let transport = Arc::clone(&self.transport);
        let tagged_tx = TaggedSender {
            gen,
            trigger: trigger_node,
            tx: self.progress_tx.clone(),
        };

        let repaint = self.repaint.clone();
        std::thread::spawn(move || {
            let result = transport.execute(&request, tagged_tx.into_sender());
            if let Err(e) = result {
                eprintln!("[execution] Task error for trigger {}: {}", trigger_node, e);
            }
            if let Some(cb) = repaint {
                cb();
            }
        });
    }

    pub fn poll(&mut self) -> Vec<(NodeId, ExecuteProgress)> {
        let mut results = Vec::new();
        while let Ok((trigger, gen, progress)) = self.progress_rx.try_recv() {
            let current_gen = self.tasks.get(&trigger).map(|s| s.generation);
            if current_gen != Some(gen) {
                continue;
            }
            let is_terminal = matches!(
                progress,
                ExecuteProgress::Finished | ExecuteProgress::Error { .. }
            );
            results.push((trigger, progress));
            if is_terminal {
                self.tasks.remove(&trigger);
            }
        }
        results
    }

    pub fn cancel(&mut self, trigger_node: NodeId) {
        self.tasks.remove(&trigger_node);
    }

    pub fn cancel_all(&mut self) {
        self.tasks.clear();
    }

    pub fn is_running(&self, trigger_node: NodeId) -> bool {
        self.tasks.contains_key(&trigger_node)
    }

    pub fn transport(&self) -> &Arc<dyn ProcessingTransport> {
        &self.transport
    }
}
