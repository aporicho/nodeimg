use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use types::{GpuTexture, NodeId, Value};

use crate::cache::model::{
    CacheBucket, CacheBudget, CacheEvent, CacheEventKind, CacheKey, CachePolicy, CacheStats,
    ExecSignature, GenerationId, PressureLevel, PreviewRequest, TextureKey,
};
use crate::cache::release_queue::{ReleaseAttempt, ReleaseQueue, RetryPolicy};
use crate::cache::result_store::{ResultStore, WriteResult};
use crate::cache::texture_store::{
    select_preview_source, upload::upload_preview_value, TextureStore,
};

#[derive(Debug, Default)]
pub struct CacheManager {
    result_store: ResultStore,
    texture_store: TextureStore,
    release_queue: ReleaseQueue,
    current_generation: RwLock<GenerationId>,
    budget: CacheBudget,
    stats: RwLock<CacheStats>,
    retry_policy: RetryPolicy,
    events: RwLock<VecDeque<CacheEvent>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            result_store: ResultStore::new(),
            texture_store: TextureStore::new(),
            release_queue: ReleaseQueue::new(),
            current_generation: RwLock::new(GenerationId::initial()),
            budget: CacheBudget::default(),
            stats: RwLock::new(CacheStats::default()),
            retry_policy: RetryPolicy::default(),
            events: RwLock::new(VecDeque::new()),
        }
    }

    #[cfg(test)]
    fn with_budget(budget: CacheBudget) -> Self {
        Self {
            result_store: ResultStore::new(),
            texture_store: TextureStore::new(),
            release_queue: ReleaseQueue::new(),
            current_generation: RwLock::new(GenerationId::initial()),
            budget,
            stats: RwLock::new(CacheStats::default()),
            retry_policy: RetryPolicy::default(),
            events: RwLock::new(VecDeque::new()),
        }
    }

    #[cfg(test)]
    fn insert_stale_handle_for_test(
        &self,
        node_id: NodeId,
        output_pin: &str,
        exec_signature: ExecSignature,
        generation: GenerationId,
        handle: types::Handle,
    ) {
        let _ = self.result_store.put(
            node_id,
            output_pin,
            exec_signature,
            generation,
            generation,
            Value::Handle(handle),
        );
    }

    pub fn current_generation(&self) -> GenerationId {
        *self
            .current_generation
            .read()
            .expect("cache manager generation lock poisoned")
    }

    pub fn get_result(
        &self,
        node_id: NodeId,
        output_pin: &str,
        exec_signature: ExecSignature,
        generation: GenerationId,
    ) -> Option<Arc<Value>> {
        if generation != self.current_generation() {
            self.stats
                .write()
                .expect("cache stats lock poisoned")
                .cache_miss_total += 1;
            self.record_event(CacheEventKind::WriteDroppedByGeneration {
                node_id,
                generation,
            });
            return None;
        }

        let key = CacheKey::new(node_id, output_pin, exec_signature);
        let entry = self.result_store.get_entry(&key)?;
        if entry.generation != generation {
            self.stats
                .write()
                .expect("cache stats lock poisoned")
                .cache_miss_total += 1;
            return None;
        }
        self.stats
            .write()
            .expect("cache stats lock poisoned")
            .cache_hit_total += 1;
        Some(entry.value)
    }

    pub fn put_result(
        &self,
        node_id: NodeId,
        output_pin: &str,
        exec_signature: ExecSignature,
        write_generation: GenerationId,
        value: Value,
    ) -> Result<WriteResult, crate::cache::model::CacheError> {
        let policy = CachePolicy::from_value(&value);
        self.put_result_with_policy(
            node_id,
            output_pin,
            exec_signature,
            write_generation,
            value,
            policy,
        )
    }

    pub fn put_result_with_policy(
        &self,
        node_id: NodeId,
        output_pin: &str,
        exec_signature: ExecSignature,
        write_generation: GenerationId,
        value: Value,
        policy: CachePolicy,
    ) -> Result<WriteResult, crate::cache::model::CacheError> {
        let current_generation = self.current_generation();

        if !policy.should_cache() {
            self.stats
                .write()
                .expect("cache stats lock poisoned")
                .dropped_write_count += 1;
            self.record_event(CacheEventKind::WriteDroppedByGeneration {
                node_id,
                generation: write_generation,
            });
            return Ok(WriteResult::DroppedByGeneration);
        }

        if let Value::Handle(handle) = &value {
            for handle_id in self.result_store.remove_stale_handles(current_generation) {
                self.release_queue
                    .enqueue(crate::cache::release_queue::PendingRelease::new(handle_id));
            }

            let current_handle_bytes = self.result_store.total_handle_bytes();
            if current_handle_bytes + handle.bytes > self.budget.handle_budget_bytes.hard {
                let mut stats = self.stats.write().expect("cache stats lock poisoned");
                stats.hard_pressure_total += 1;
                stats.handle_hard_pressure_total += 1;
                drop(stats);
                self.record_pressure(
                    CacheBucket::Handle,
                    PressureLevel::Hard,
                    current_handle_bytes + handle.bytes,
                );
                return Err(crate::cache::model::CacheError::HandleMemoryPressure {
                    node_id,
                    output_pin: output_pin.to_string(),
                    generation: write_generation,
                });
            }
        }

        let result = self.result_store.put(
            node_id,
            output_pin,
            exec_signature,
            current_generation,
            write_generation,
            value,
        )?;

        match &result {
            WriteResult::Written { replaced_entry } => {
                let key = TextureKey::new(node_id, output_pin);
                let _ = self.texture_store.invalidate(&key);

                if let Some(entry) = replaced_entry {
                    self.enqueue_handle_if_needed(entry.value.as_ref());
                }

                let result_bytes_before_eviction = self.result_store.total_bytes();
                let texture_bytes_before_eviction = self.texture_store.total_bytes();
                let handle_bytes_before_eviction = self.result_store.total_handle_bytes();

                let evicted_entries = self
                    .result_store
                    .evict_to_budget(self.budget.result_budget_bytes);
                for entry in &evicted_entries {
                    self.enqueue_handle_if_needed(entry.value.as_ref());
                }

                self.stats
                    .write()
                    .expect("cache stats lock poisoned")
                    .eviction_count += evicted_entries.len() as u64;

                if result_bytes_before_eviction > self.budget.result_budget_bytes.soft
                    && result_bytes_before_eviction <= self.budget.result_budget_bytes.hard
                {
                    {
                        let mut stats = self.stats.write().expect("cache stats lock poisoned");
                        stats.soft_pressure_total += 1;
                        stats.result_soft_pressure_total += 1;
                    }
                    self.record_pressure(
                        CacheBucket::Result,
                        PressureLevel::Soft,
                        result_bytes_before_eviction,
                    );
                }
                if result_bytes_before_eviction > self.budget.result_budget_bytes.hard {
                    {
                        let mut stats = self.stats.write().expect("cache stats lock poisoned");
                        stats.hard_pressure_total += 1;
                        stats.result_hard_pressure_total += 1;
                    }
                    self.record_pressure(
                        CacheBucket::Result,
                        PressureLevel::Hard,
                        result_bytes_before_eviction,
                    );
                }
                if texture_bytes_before_eviction > self.budget.texture_budget_bytes.soft
                    && texture_bytes_before_eviction <= self.budget.texture_budget_bytes.hard
                {
                    {
                        let mut stats = self.stats.write().expect("cache stats lock poisoned");
                        stats.soft_pressure_total += 1;
                        stats.texture_soft_pressure_total += 1;
                    }
                    self.record_pressure(
                        CacheBucket::Texture,
                        PressureLevel::Soft,
                        texture_bytes_before_eviction,
                    );
                }
                if texture_bytes_before_eviction > self.budget.texture_budget_bytes.hard {
                    {
                        let mut stats = self.stats.write().expect("cache stats lock poisoned");
                        stats.hard_pressure_total += 1;
                        stats.texture_hard_pressure_total += 1;
                    }
                    self.record_pressure(
                        CacheBucket::Texture,
                        PressureLevel::Hard,
                        texture_bytes_before_eviction,
                    );
                }
                if handle_bytes_before_eviction > self.budget.handle_budget_bytes.soft
                    && handle_bytes_before_eviction <= self.budget.handle_budget_bytes.hard
                {
                    {
                        let mut stats = self.stats.write().expect("cache stats lock poisoned");
                        stats.soft_pressure_total += 1;
                        stats.handle_soft_pressure_total += 1;
                    }
                    self.record_pressure(
                        CacheBucket::Handle,
                        PressureLevel::Soft,
                        handle_bytes_before_eviction,
                    );
                }
            }
            WriteResult::DroppedByGeneration => {
                self.stats
                    .write()
                    .expect("cache stats lock poisoned")
                    .dropped_write_count += 1;
                self.record_event(CacheEventKind::WriteDroppedByGeneration {
                    node_id,
                    generation: write_generation,
                });
            }
        }

        Ok(result)
    }

    pub fn get_preview_texture(
        &self,
        request: PreviewRequest,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<Arc<GpuTexture>> {
        if let Some(texture) = self.texture_store.get(&request) {
            self.stats
                .write()
                .expect("cache stats lock poisoned")
                .preview_hit_total += 1;
            return Some(texture);
        }

        self.stats
            .write()
            .expect("cache stats lock poisoned")
            .preview_miss_total += 1;
        self.record_event(CacheEventKind::PreviewMiss);

        let Some(value) = select_preview_source(&self.result_store, &request) else {
            return None;
        };

        let upload_started_at = Instant::now();
        let Some(entry) = upload_preview_value(&value, request.generation, device, queue) else {
            return None;
        };
        let texture = Arc::clone(&entry.texture);
        let key = TextureKey::new(request.node_id, request.output_pin.clone());
        let _ = self.texture_store.insert(key, entry);
        let texture_bytes_before_eviction = self.texture_store.total_bytes();
        let evicted = self
            .texture_store
            .evict_to_budget(self.budget.texture_budget_bytes);
        {
            let mut stats = self.stats.write().expect("cache stats lock poisoned");
            stats.eviction_count += evicted as u64;
            stats.preview_upload_latency_ms += upload_started_at.elapsed().as_millis();
        }
        if texture_bytes_before_eviction > self.budget.texture_budget_bytes.soft
            && texture_bytes_before_eviction <= self.budget.texture_budget_bytes.hard
        {
            {
                let mut stats = self.stats.write().expect("cache stats lock poisoned");
                stats.soft_pressure_total += 1;
                stats.texture_soft_pressure_total += 1;
            }
            self.record_pressure(
                CacheBucket::Texture,
                PressureLevel::Soft,
                texture_bytes_before_eviction,
            );
        }
        if texture_bytes_before_eviction > self.budget.texture_budget_bytes.hard {
            {
                let mut stats = self.stats.write().expect("cache stats lock poisoned");
                stats.hard_pressure_total += 1;
                stats.texture_hard_pressure_total += 1;
            }
            self.record_pressure(
                CacheBucket::Texture,
                PressureLevel::Hard,
                texture_bytes_before_eviction,
            );
        }

        Some(texture)
    }

    pub fn invalidate_output(&self, node_id: NodeId, output_pin: &str) {
        let _ = self.result_store.invalidate_output(node_id, output_pin);
        let _ = self
            .texture_store
            .invalidate(&TextureKey::new(node_id, output_pin));
    }

    pub fn invalidate_node(&self, node_id: NodeId) {
        let _ = self.result_store.invalidate_node(node_id);
        let _ = self.texture_store.invalidate_node(node_id);
    }

    pub fn invalidate_subgraph(&self, node_ids: &[NodeId]) {
        let _ = self.result_store.invalidate_subgraph(node_ids);
        let _ = self.texture_store.invalidate_subgraph(node_ids);
    }

    pub fn clear_all(&self) -> GenerationId {
        self.result_store.clear();
        self.texture_store.clear();
        self.release_queue.clear();

        let mut generation = self
            .current_generation
            .write()
            .expect("cache manager generation lock poisoned");
        *generation = generation.next();
        *generation
    }

    pub fn clear_handles(&self) -> CacheStats {
        for handle_id in self.result_store.remove_handles() {
            self.release_queue
                .enqueue(crate::cache::release_queue::PendingRelease::new(handle_id));
        }

        self.release_queue.clear();
        CacheStats {
            pending_release_count: 0,
            handle_bytes: 0,
            ..CacheStats::default()
        }
    }

    pub fn stats_snapshot(&self) -> CacheStats {
        let mut snapshot = self
            .stats
            .read()
            .expect("cache stats lock poisoned")
            .clone();
        snapshot.cache_bytes = self.result_store.total_bytes();
        snapshot.texture_bytes = self.texture_store.total_bytes();
        snapshot.handle_bytes = self.result_store.total_handle_bytes();
        snapshot.pending_release_count = self.release_queue.len();
        snapshot
    }

    pub fn events_snapshot(&self) -> Vec<CacheEvent> {
        self.events
            .read()
            .expect("cache events lock poisoned")
            .iter()
            .cloned()
            .collect()
    }

    pub fn retry_pending_release<F>(&self, release_fn: F) -> Option<ReleaseAttempt>
    where
        F: FnOnce(&str) -> bool,
    {
        let report = self
            .release_queue
            .process_next(self.retry_policy, release_fn);

        if let Some(report) = &report {
            match report.attempt {
                ReleaseAttempt::Released => {
                    self.stats
                        .write()
                        .expect("cache stats lock poisoned")
                        .release_released_total += 1;
                    self.record_event(CacheEventKind::HandleRelease {
                        handle_id: report.handle_id.clone(),
                        outcome: "released",
                        retry_count: report.retry_count,
                    });
                }
                ReleaseAttempt::Requeued => {
                    self.stats
                        .write()
                        .expect("cache stats lock poisoned")
                        .release_requeued_total += 1;
                    self.record_event(CacheEventKind::HandleRelease {
                        handle_id: report.handle_id.clone(),
                        outcome: "requeued",
                        retry_count: report.retry_count,
                    });
                }
                ReleaseAttempt::Dropped => {
                    self.stats
                        .write()
                        .expect("cache stats lock poisoned")
                        .release_dropped_total += 1;
                    self.record_event(CacheEventKind::HandleRelease {
                        handle_id: report.handle_id.clone(),
                        outcome: "dropped",
                        retry_count: report.retry_count,
                    });
                }
            }
        }

        report.map(|report| report.attempt)
    }

    fn enqueue_handle_if_needed(&self, value: &Value) {
        if let Value::Handle(handle) = value {
            self.release_queue
                .enqueue(crate::cache::release_queue::PendingRelease::new(
                    handle.handle_id.clone(),
                ));
        }
    }

    fn record_pressure(&self, bucket: CacheBucket, level: PressureLevel, bytes: usize) {
        self.record_event(CacheEventKind::CachePressure {
            bucket,
            level,
            bytes,
        });
    }

    fn record_event(&self, kind: CacheEventKind) {
        const MAX_EVENTS: usize = 256;

        let mut events = self.events.write().expect("cache events lock poisoned");
        if events.len() >= MAX_EVENTS {
            events.pop_front();
        }
        events.push_back(CacheEvent::new(kind));
    }
}

#[cfg(test)]
mod tests {
    use types::{DataType, Handle, Image, Value};

    use crate::cache::model::{
        BudgetWatermark, CacheBudget, CacheEventKind, CachePolicy, ExecSignature, GenerationId,
        PreviewRequest,
    };

    use super::CacheManager;
    use crate::cache::release_queue::ReleaseAttempt;

    fn gpu_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("wgpu adapter");
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("wgpu device")
    }

    fn image_value(width: u32, height: u32) -> Value {
        Value::Image(Image::from_cpu(image::DynamicImage::new_rgba8(
            width, height,
        )))
    }

    #[test]
    fn put_result_invalidates_existing_texture_slot_without_error() {
        let cache = CacheManager::new();
        let result = cache.put_result(
            types::NodeId(1),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
            Value::Image(Image::from_cpu(image::DynamicImage::new_rgba8(1, 1))),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn clear_all_advances_generation() {
        let cache = CacheManager::new();

        let next = cache.clear_all();

        assert_eq!(next, GenerationId(1));
        assert_eq!(cache.current_generation(), GenerationId(1));
    }

    #[test]
    fn clear_handles_returns_empty_queue_stats() {
        let cache = CacheManager::new();

        let stats = cache.clear_handles();

        assert_eq!(stats.pending_release_count, 0);
    }

    #[test]
    fn get_result_records_hit_stats() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "image",
                sig,
                GenerationId::initial(),
                Value::Int(7),
            )
            .expect("write should succeed");

        let value = cache.get_result(types::NodeId(1), "image", sig, GenerationId::initial());
        let stats = cache.stats_snapshot();

        assert!(matches!(value.as_deref(), Some(Value::Int(7))));
        assert_eq!(stats.cache_hit_total, 1);
    }

    #[test]
    fn stale_generation_reads_as_miss() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "image",
                sig,
                GenerationId::initial(),
                Value::Int(7),
            )
            .expect("write should succeed");
        let _ = cache.clear_all();

        let value = cache.get_result(types::NodeId(1), "image", sig, GenerationId::initial());
        let stats = cache.stats_snapshot();

        assert!(value.is_none());
        assert_eq!(stats.cache_miss_total, 1);
    }

    #[test]
    fn dropped_write_records_generation_event() {
        let cache = CacheManager::new();

        let _ = cache.put_result(
            types::NodeId(1),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId(99),
            Value::Int(7),
        );

        assert!(cache
            .events_snapshot()
            .iter()
            .any(|event| matches!(event.kind, CacheEventKind::WriteDroppedByGeneration { .. })));
    }

    #[test]
    fn result_store_evicts_when_budget_exceeded() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(4, 4),
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(1024, 1024),
        );
        let cache = CacheManager::with_budget(budget);

        cache
            .put_result(
                types::NodeId(1),
                "image",
                ExecSignature::new(1, 1, 1, 1),
                GenerationId::initial(),
                Value::String("1234".into()),
            )
            .expect("first write should succeed");
        cache
            .put_result(
                types::NodeId(2),
                "image",
                ExecSignature::new(1, 1, 2, 1),
                GenerationId::initial(),
                Value::String("1234".into()),
            )
            .expect("second write should succeed");

        let first = cache.get_result(
            types::NodeId(1),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
        );
        let second = cache.get_result(
            types::NodeId(2),
            "image",
            ExecSignature::new(1, 1, 2, 1),
            GenerationId::initial(),
        );

        assert!(first.is_none());
        assert!(second.is_some());
        assert!(cache.stats_snapshot().eviction_count >= 1);
    }

    #[test]
    fn result_soft_pressure_is_counted() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(2, 1024),
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(1024, 1024),
        );
        let cache = CacheManager::with_budget(budget);

        cache
            .put_result(
                types::NodeId(1),
                "text",
                ExecSignature::new(1, 1, 1, 1),
                GenerationId::initial(),
                Value::String("1234".into()),
            )
            .expect("write should succeed");

        let stats = cache.stats_snapshot();
        assert!(stats.result_soft_pressure_total >= 1);
    }

    #[test]
    fn oversized_handle_is_rejected() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(16, 16),
        );
        let cache = CacheManager::with_budget(budget);

        let result = cache.put_result(
            types::NodeId(1),
            "model",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
            Value::Handle(Handle::new("h1", DataType::handle(), "python", 32)),
        );

        assert!(matches!(
            result,
            Err(crate::cache::model::CacheError::HandleMemoryPressure { .. })
        ));
    }

    #[test]
    fn cumulative_handle_budget_is_enforced() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(16, 16),
        );
        let cache = CacheManager::with_budget(budget);

        cache
            .put_result(
                types::NodeId(1),
                "model_a",
                ExecSignature::new(1, 1, 1, 1),
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("first handle write should succeed");

        let result = cache.put_result(
            types::NodeId(2),
            "model_b",
            ExecSignature::new(1, 1, 2, 1),
            GenerationId::initial(),
            Value::Handle(Handle::new("h2", DataType::handle(), "python", 9)),
        );

        assert!(matches!(
            result,
            Err(crate::cache::model::CacheError::HandleMemoryPressure { .. })
        ));
        let stats = cache.stats_snapshot();
        assert_eq!(stats.handle_bytes, 8);
        assert_eq!(stats.hard_pressure_total, 1);
    }

    #[test]
    fn clear_handles_removes_cached_handles() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("handle write should succeed");

        let before = cache.get_result(types::NodeId(1), "model", sig, GenerationId::initial());
        assert!(matches!(before.as_deref(), Some(Value::Handle(_))));

        let _ = cache.clear_handles();
        let after = cache.get_result(types::NodeId(1), "model", sig, GenerationId::initial());

        assert!(after.is_none());
    }

    #[test]
    fn replacing_handle_enqueues_old_handle_release() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("first handle write should succeed");
        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h2", DataType::handle(), "python", 8)),
            )
            .expect("second handle write should succeed");

        assert_eq!(cache.stats_snapshot().pending_release_count, 1);
    }

    #[test]
    fn handle_is_not_evicted_by_result_budget_lru() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(8, 8),
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(1024, 1024),
        );
        let cache = CacheManager::with_budget(budget);

        cache
            .put_result(
                types::NodeId(1),
                "model_a",
                ExecSignature::new(1, 1, 1, 1),
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("first handle write should succeed");
        cache
            .put_result(
                types::NodeId(2),
                "model_b",
                ExecSignature::new(1, 1, 2, 1),
                GenerationId::initial(),
                Value::Handle(Handle::new("h2", DataType::handle(), "python", 8)),
            )
            .expect("second handle write should succeed");

        let stats = cache.stats_snapshot();
        assert_eq!(stats.pending_release_count, 0);
        assert_eq!(stats.handle_bytes, 16);
    }

    #[test]
    fn retry_pending_release_requeues_failed_item() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("handle write should succeed");
        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h2", DataType::handle(), "python", 8)),
            )
            .expect("replacement handle write should succeed");

        let attempt = cache.retry_pending_release(|_| false);

        assert_eq!(attempt, Some(ReleaseAttempt::Requeued));
        assert_eq!(cache.stats_snapshot().pending_release_count, 1);
    }

    #[test]
    fn retry_pending_release_drops_on_success() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("handle write should succeed");
        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h2", DataType::handle(), "python", 8)),
            )
            .expect("replacement handle write should succeed");

        let attempt = cache.retry_pending_release(|_| true);

        assert_eq!(attempt, Some(ReleaseAttempt::Released));
        assert_eq!(cache.stats_snapshot().pending_release_count, 0);
    }

    #[test]
    fn retry_pending_release_updates_release_stats() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("handle write should succeed");
        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h2", DataType::handle(), "python", 8)),
            )
            .expect("replacement handle write should succeed");

        let _ = cache.retry_pending_release(|_| false);
        let _ = cache.retry_pending_release(|_| true);
        let stats = cache.stats_snapshot();

        assert_eq!(stats.release_requeued_total, 1);
        assert_eq!(stats.release_released_total, 1);
    }

    #[test]
    fn retry_pending_release_records_release_event() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h1", DataType::handle(), "python", 8)),
            )
            .expect("handle write should succeed");
        cache
            .put_result(
                types::NodeId(1),
                "model",
                sig,
                GenerationId::initial(),
                Value::Handle(Handle::new("h2", DataType::handle(), "python", 8)),
            )
            .expect("replacement handle write should succeed");

        let _ = cache.retry_pending_release(|_| false);

        assert!(cache.events_snapshot().iter().any(|event| matches!(
            event.kind,
            CacheEventKind::HandleRelease {
                outcome: "requeued",
                ..
            }
        )));
    }

    #[test]
    fn preview_miss_updates_preview_stats() {
        let cache = CacheManager::new();
        let request = PreviewRequest::new(
            types::NodeId(99),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
        );

        let (device, queue) = gpu_device();

        let texture = cache.get_preview_texture(request, &device, &queue);
        let stats = cache.stats_snapshot();

        assert!(texture.is_none());
        assert_eq!(stats.preview_miss_total, 1);
    }

    #[test]
    fn texture_soft_pressure_is_counted() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(2, 1024),
            BudgetWatermark::new(1024, 1024),
        );
        let cache = CacheManager::with_budget(budget);
        let (device, queue) = gpu_device();

        cache
            .put_result(
                types::NodeId(1),
                "image",
                ExecSignature::new(1, 1, 1, 1),
                GenerationId::initial(),
                image_value(1, 1),
            )
            .expect("result write should succeed");

        let request = PreviewRequest::new(
            types::NodeId(1),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
        );
        let _ = cache.get_preview_texture(request, &device, &queue);

        let stats = cache.stats_snapshot();
        assert!(stats.texture_soft_pressure_total >= 1);
    }

    #[test]
    fn texture_budget_eviction_reuploads_evicted_preview() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(4, 4),
            BudgetWatermark::new(1024, 1024),
        );
        let cache = CacheManager::with_budget(budget);
        let (device, queue) = gpu_device();

        cache
            .put_result(
                types::NodeId(1),
                "image",
                ExecSignature::new(1, 1, 1, 1),
                GenerationId::initial(),
                image_value(1, 1),
            )
            .expect("first result write should succeed");
        cache
            .put_result(
                types::NodeId(2),
                "image",
                ExecSignature::new(1, 1, 2, 1),
                GenerationId::initial(),
                image_value(1, 1),
            )
            .expect("second result write should succeed");

        let request1 = PreviewRequest::new(
            types::NodeId(1),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
        );
        let request2 = PreviewRequest::new(
            types::NodeId(2),
            "image",
            ExecSignature::new(1, 1, 2, 1),
            GenerationId::initial(),
        );

        assert!(cache
            .get_preview_texture(request1.clone(), &device, &queue)
            .is_some());
        assert!(cache
            .get_preview_texture(request2, &device, &queue)
            .is_some());

        let stats_after_two = cache.stats_snapshot();
        assert!(stats_after_two.eviction_count >= 1);
        assert_eq!(stats_after_two.texture_bytes, 4);

        assert!(cache
            .get_preview_texture(request1, &device, &queue)
            .is_some());
        let stats_after_reupload = cache.stats_snapshot();
        assert_eq!(stats_after_reupload.preview_miss_total, 3);
    }

    #[test]
    fn texture_lru_touch_keeps_recent_preview() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(8, 8),
            BudgetWatermark::new(1024, 1024),
        );
        let cache = CacheManager::with_budget(budget);
        let (device, queue) = gpu_device();

        for (node_id, sig_hash) in [(1, 1_u64), (2, 2_u64), (3, 3_u64)] {
            cache
                .put_result(
                    types::NodeId(node_id),
                    "image",
                    ExecSignature::new(1, 1, sig_hash, 1),
                    GenerationId::initial(),
                    image_value(1, 1),
                )
                .expect("result write should succeed");
        }

        let req1 = PreviewRequest::new(
            types::NodeId(1),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
        );
        let req2 = PreviewRequest::new(
            types::NodeId(2),
            "image",
            ExecSignature::new(1, 1, 2, 1),
            GenerationId::initial(),
        );
        let req3 = PreviewRequest::new(
            types::NodeId(3),
            "image",
            ExecSignature::new(1, 1, 3, 1),
            GenerationId::initial(),
        );

        assert!(cache
            .get_preview_texture(req1.clone(), &device, &queue)
            .is_some());
        assert!(cache
            .get_preview_texture(req2.clone(), &device, &queue)
            .is_some());
        assert!(cache
            .get_preview_texture(req1.clone(), &device, &queue)
            .is_some());
        assert!(cache.get_preview_texture(req3, &device, &queue).is_some());

        let stats = cache.stats_snapshot();
        assert!(stats.preview_hit_total >= 1);
        assert_eq!(stats.texture_bytes, 8);

        assert!(cache.get_preview_texture(req2, &device, &queue).is_some());
        let stats_after = cache.stats_snapshot();
        assert!(stats_after.preview_miss_total >= 4);
    }

    #[test]
    fn explicit_skip_policy_bypasses_cache_write() {
        let cache = CacheManager::new();
        let sig = ExecSignature::new(1, 1, 1, 1);

        let result = cache.put_result_with_policy(
            types::NodeId(1),
            "image",
            sig,
            GenerationId::initial(),
            Value::Int(7),
            CachePolicy::skip(),
        );

        assert!(result.is_ok());
        assert!(cache
            .get_result(types::NodeId(1), "image", sig, GenerationId::initial())
            .is_none());
    }

    #[test]
    fn stale_handles_are_released_before_budget_rejection() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(1024, 1024),
            BudgetWatermark::new(8, 8),
        );
        let cache = CacheManager::with_budget(budget);
        let old_sig = ExecSignature::new(1, 1, 1, 1);
        let new_sig = ExecSignature::new(1, 1, 2, 1);

        cache.insert_stale_handle_for_test(
            types::NodeId(1),
            "model_old",
            old_sig,
            GenerationId(0),
            Handle::new("h_old", DataType::handle(), "python", 8),
        );
        *cache
            .current_generation
            .write()
            .expect("cache manager generation lock poisoned") = GenerationId(1);

        let result = cache.put_result(
            types::NodeId(2),
            "model_new",
            new_sig,
            GenerationId(1),
            Value::Handle(Handle::new("h_new", DataType::handle(), "python", 8)),
        );

        assert!(result.is_ok());
        assert_eq!(cache.stats_snapshot().pending_release_count, 1);
        assert_eq!(cache.stats_snapshot().handle_bytes, 8);
    }
}
