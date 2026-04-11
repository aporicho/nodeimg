use std::sync::{Arc, RwLock};

use types::{GpuTexture, NodeId, Value};

use crate::cache::model::{
    CacheBudget, CacheKey, CacheStats, ExecSignature, GenerationId, PreviewRequest, TextureKey,
};
use crate::cache::release_queue::ReleaseQueue;
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
        }
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
        let current_generation = self.current_generation();
        let result = self.result_store.put(
            node_id,
            output_pin,
            exec_signature,
            current_generation,
            write_generation,
            value,
        )?;

        if result == WriteResult::Written {
            let key = TextureKey::new(node_id, output_pin);
            let _ = self.texture_store.invalidate(&key);
            let evicted = self
                .result_store
                .evict_to_budget(self.budget.result_budget_bytes);
            self.stats
                .write()
                .expect("cache stats lock poisoned")
                .eviction_count += evicted as u64;
        } else {
            self.stats
                .write()
                .expect("cache stats lock poisoned")
                .dropped_write_count += 1;
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
            return Some(texture);
        }

        let value = select_preview_source(&self.result_store, &request)?;
        let entry = upload_preview_value(&value, request.generation, device, queue)?;
        let texture = Arc::clone(&entry.texture);
        let key = TextureKey::new(request.node_id, request.output_pin.clone());
        let _ = self.texture_store.insert(key, entry);
        let evicted = self
            .texture_store
            .evict_to_budget(self.budget.texture_budget_bytes);
        self.stats
            .write()
            .expect("cache stats lock poisoned")
            .eviction_count += evicted as u64;

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
        self.release_queue.clear();
        CacheStats {
            pending_release_count: 0,
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
        snapshot.pending_release_count = self.release_queue.len();
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use types::{Image, Value};

    use crate::cache::model::{BudgetWatermark, CacheBudget, ExecSignature, GenerationId};

    use super::CacheManager;

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
}
