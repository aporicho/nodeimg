use eframe::egui;
use nodeimg_engine::transport::{ExecuteProgress, GraphRequest, ProcessingTransport};
use std::sync::mpsc;
use std::sync::Arc;

/// Wraps a Sender to tag each ExecuteProgress with a generation number.
/// Uses a forwarding thread so progress is delivered in real-time.
struct TaggedSender {
    gen: u64,
    tx: mpsc::Sender<(u64, ExecuteProgress)>,
}

impl TaggedSender {
    fn into_sender(self) -> mpsc::Sender<ExecuteProgress> {
        let (inner_tx, inner_rx) = mpsc::channel();
        let tagged_tx = self.tx;
        let gen = self.gen;
        std::thread::spawn(move || {
            while let Ok(progress) = inner_rx.recv() {
                if tagged_tx.send((gen, progress)).is_err() {
                    break;
                }
            }
        });
        inner_tx
    }
}

/// App-side execution dispatcher. Spawns background threads and polls results each frame.
pub struct ExecutionManager {
    transport: Arc<dyn ProcessingTransport>,
    progress_rx: Option<mpsc::Receiver<(u64, ExecuteProgress)>>,
    generation: u64,
    running: bool,
    repaint: Option<egui::Context>,
}

impl ExecutionManager {
    pub fn new(transport: Arc<dyn ProcessingTransport>) -> Self {
        Self {
            transport,
            progress_rx: None,
            generation: 0,
            running: false,
            repaint: None,
        }
    }

    pub fn set_repaint_ctx(&mut self, ctx: egui::Context) {
        self.repaint = Some(ctx);
    }

    pub fn submit(&mut self, request: GraphRequest) {
        self.generation += 1;
        let gen = self.generation;
        let transport = Arc::clone(&self.transport);
        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.running = true;

        let repaint = self.repaint.clone();
        std::thread::spawn(move || {
            let tagged_tx = TaggedSender { gen, tx: tx.clone() };
            let result = transport.execute(request, tagged_tx.into_sender());

            if let Err(e) = result {
                let _ = tx.send((gen, ExecuteProgress::Error {
                    node_id: None,
                    message: e,
                }));
            }

            if let Some(ctx) = repaint {
                ctx.request_repaint();
            }
        });
    }

    pub fn poll(&mut self) -> Vec<ExecuteProgress> {
        let mut results = Vec::new();
        if let Some(ref rx) = self.progress_rx {
            while let Ok((gen, progress)) = rx.try_recv() {
                if gen != self.generation {
                    continue;
                }
                if matches!(progress, ExecuteProgress::Finished | ExecuteProgress::Error { .. }) {
                    self.running = false;
                }
                results.push(progress);
            }
        }
        results
    }

    pub fn cancel(&mut self) {
        self.generation += 1;
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn transport(&self) -> &Arc<dyn ProcessingTransport> {
        &self.transport
    }
}
