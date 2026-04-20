use std::time::SystemTime;

use types::NodeId;

use super::GenerationId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheBucket {
    Result,
    Texture,
    Handle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressureLevel {
    Soft,
    Hard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheEventKind {
    CachePressure {
        bucket: CacheBucket,
        level: PressureLevel,
        bytes: usize,
    },
    HandleRelease {
        handle_id: String,
        outcome: &'static str,
        retry_count: u32,
    },
    WriteDroppedByGeneration {
        node_id: NodeId,
        generation: GenerationId,
    },
    PreviewMiss,
}

#[derive(Clone, Debug)]
pub struct CacheEvent {
    pub at: SystemTime,
    pub kind: CacheEventKind,
}

impl CacheEvent {
    pub fn new(kind: CacheEventKind) -> Self {
        Self {
            at: SystemTime::now(),
            kind,
        }
    }
}
