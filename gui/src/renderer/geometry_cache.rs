#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeometryCacheStats {
    pub hits: usize,
    pub misses: usize,
    pub tessellated_vertices: usize,
}

impl GeometryCacheStats {
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub fn record_miss(&mut self, tessellated_vertices: usize) {
        self.misses += 1;
        self.tessellated_vertices += tessellated_vertices;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
