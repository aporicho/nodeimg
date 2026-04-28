#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RendererPrepareStats {
    pub backend_commands: usize,
    pub render_passes: usize,
    pub text_batches: usize,
    pub affine_text_fallbacks: usize,
    pub grid_commands: usize,
    pub grid_dot_expansions: usize,
    pub geometry_cache_hits: usize,
    pub geometry_cache_misses: usize,
    pub tessellated_vertices: usize,
    pub upload_bytes: usize,
    pub upload_buffer_grows: usize,
}
