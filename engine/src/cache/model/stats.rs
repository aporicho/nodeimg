#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub cache_hit_total: u64,
    pub cache_miss_total: u64,
    pub cache_bytes: usize,
    pub texture_bytes: usize,
    pub handle_bytes: usize,
    pub eviction_count: u64,
    pub dropped_write_count: u64,
    pub pending_release_count: usize,
}
