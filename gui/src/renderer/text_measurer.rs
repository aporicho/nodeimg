use std::collections::HashMap;

use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Style, Weight};

use super::style::{TextFamily, TextStyle, TextWeight};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct TextCacheKey {
    text: String,
    size_bits: u32,
    line_height_bits: u32,
    family: TextFamily,
    weight: TextWeight,
    italic: bool,
}

impl TextCacheKey {
    pub fn new(text: &str, style: &TextStyle) -> Self {
        Self {
            text: text.to_string(),
            size_bits: style.size.to_bits(),
            line_height_bits: style.line_height.to_bits(),
            family: style.family,
            weight: style.weight,
            italic: style.italic,
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
    pub(crate) fn ensure_buffer(&mut self, text: &str, style: &TextStyle) {
        let key = TextCacheKey::new(text, style);
        if !self.buffer_cache.contains_key(&key) {
            let mut buffer = Buffer::new(&mut self.font_system, glyphon_metrics(style));
            let line_height = style.size * style.line_height;
            buffer.set_size(
                &mut self.font_system,
                Some(f32::MAX),
                Some(line_height * 2.0),
            );
            buffer.set_text(
                &mut self.font_system,
                text,
                &glyphon_attrs(style),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffer_cache
                .insert(key, CachedBuffer { buffer, used: true });
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
        let style = TextStyle::new(super::types::Color::WHITE, size);
        self.measure_with_style(text, &style)
    }

    pub fn measure_with_style(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        self.ensure_buffer(text, style);

        let key = TextCacheKey::new(text, style);
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

impl Default for TextMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn glyphon_metrics(style: &TextStyle) -> Metrics {
    Metrics::new(style.size, style.size * style.line_height)
}

pub(crate) fn glyphon_attrs(style: &TextStyle) -> Attrs<'_> {
    Attrs::new()
        .family(glyphon_family(style.family))
        .weight(glyphon_weight(style.weight))
        .style(if style.italic {
            Style::Italic
        } else {
            Style::Normal
        })
}

pub(crate) fn glyphon_family(family: TextFamily) -> Family<'static> {
    match family {
        TextFamily::Sans => Family::SansSerif,
        TextFamily::Monospace => Family::Monospace,
    }
}

pub(crate) fn glyphon_weight(weight: TextWeight) -> Weight {
    match weight {
        TextWeight::Normal => Weight::NORMAL,
        TextWeight::Medium => Weight::MEDIUM,
        TextWeight::Semibold => Weight::SEMIBOLD,
        TextWeight::Bold => Weight::BOLD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Color;

    #[test]
    fn cache_key_changes_with_family_and_line_height() {
        let a = TextStyle::new(Color::WHITE, 12.0);
        let b = TextStyle::new(Color::WHITE, 12.0).with_family(TextFamily::Monospace);
        let c = TextStyle::new(Color::WHITE, 12.0).with_line_height(1.6);
        let d = TextStyle::new(Color::WHITE, 12.0).with_weight(TextWeight::Bold);
        let e = TextStyle::new(Color::WHITE, 12.0).with_italic(true);

        assert_ne!(
            TextCacheKey::new("hello", &a),
            TextCacheKey::new("hello", &b)
        );
        assert_ne!(
            TextCacheKey::new("hello", &a),
            TextCacheKey::new("hello", &c)
        );
        assert_ne!(
            TextCacheKey::new("hello", &a),
            TextCacheKey::new("hello", &d)
        );
        assert_ne!(
            TextCacheKey::new("hello", &a),
            TextCacheKey::new("hello", &e)
        );
    }

    #[test]
    fn line_height_affects_measured_height() {
        let mut measurer = TextMeasurer::new();
        let base = TextStyle::new(Color::WHITE, 14.0);
        let taller = base.with_line_height(1.8);

        let (_, h1) = measurer.measure_with_style("Hello", &base);
        let (_, h2) = measurer.measure_with_style("Hello", &taller);

        assert!(h2 > h1);
    }
}
