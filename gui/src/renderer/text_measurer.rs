use std::collections::HashMap;

use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};

#[derive(Hash, PartialEq, Eq, Clone)]
pub(crate) struct TextCacheKey {
    text: String,
    size_bits: u32,
}

impl TextCacheKey {
    pub fn new(text: &str, size: f32) -> Self {
        Self {
            text: text.to_string(),
            size_bits: size.to_bits(),
        }
    }
}

pub(crate) struct CachedBuffer {
    pub buffer: Buffer,
    pub used: bool,
}

pub struct TextMeasurer {
    pub(crate) font_system: FontSystem,
    pub(crate) buffer_cache: HashMap<TextCacheKey, CachedBuffer>,
}

impl TextMeasurer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            buffer_cache: HashMap::new(),
        }
    }

    /// Ensure a buffer exists in the cache for the given text/size.
    /// Used by TextPipeline::prepare() phase 1.
    pub(crate) fn ensure_buffer(&mut self, text: &str, size: f32) {
        let key = TextCacheKey::new(text, size);
        if !self.buffer_cache.contains_key(&key) {
            let mut buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(size, size * 1.2),
            );
            buffer.set_size(
                &mut self.font_system,
                Some(f32::MAX),
                Some(size * 2.0),
            );
            buffer.set_text(
                &mut self.font_system,
                text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffer_cache.insert(key, CachedBuffer { buffer, used: true });
        } else {
            self.buffer_cache.get_mut(&key).unwrap().used = true;
        }
    }

    /// Mark all cached buffers as unused (start of frame).
    pub(crate) fn mark_all_unused(&mut self) {
        for entry in self.buffer_cache.values_mut() {
            entry.used = false;
        }
    }

    /// Evict unused buffers (end of prepare).
    pub(crate) fn evict_unused(&mut self) {
        self.buffer_cache.retain(|_, v| v.used);
    }

    /// Measure text dimensions in logical pixels. Reuses buffer_cache.
    pub fn measure(&mut self, text: &str, size: f32) -> (f32, f32) {
        self.ensure_buffer(text, size);

        let key = TextCacheKey::new(text, size);
        let buffer = &self.buffer_cache[&key].buffer;
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height += run.line_height;
        }
        (width, height)
    }
}
