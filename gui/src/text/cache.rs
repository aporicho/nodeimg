use std::collections::HashMap;

use crate::renderer::{TextFamily, TextMeasurer, TextStyle, TextWeight};

use super::layout::{layout_text, TextLayoutPolicy, TextLayoutResult};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextLayoutCache {
    entries: HashMap<TextLayoutCacheKey, TextLayoutResult>,
}

impl TextLayoutCache {
    pub(crate) fn layout_text(
        &mut self,
        text: &str,
        policy: TextLayoutPolicy,
        max_width: f32,
        measurer: &mut TextMeasurer,
        style: &TextStyle,
    ) -> (TextLayoutResult, bool) {
        let key = TextLayoutCacheKey::new(text, policy, max_width, style);
        if let Some(result) = self.entries.get(&key) {
            return (result.clone(), true);
        }
        let result = layout_text(text, policy, max_width, measurer, style);
        self.entries.insert(key, result.clone());
        (result, false)
    }
}

impl Default for TextLayoutCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextLayoutCacheKey {
    text: String,
    policy: TextLayoutPolicy,
    max_width: i32,
    size_bits: u32,
    line_height_bits: u32,
    family: TextFamily,
    weight: TextWeight,
    italic: bool,
}

impl TextLayoutCacheKey {
    fn new(text: &str, policy: TextLayoutPolicy, max_width: f32, style: &TextStyle) -> Self {
        Self {
            text: text.to_string(),
            policy,
            max_width: quantize(max_width),
            size_bits: style.size.to_bits(),
            line_height_bits: style.line_height.to_bits(),
            family: style.family,
            weight: style.weight,
            italic: style.italic,
        }
    }
}

fn quantize(value: f32) -> i32 {
    if value.is_nan() {
        0
    } else if value.is_infinite() {
        i32::MAX
    } else {
        (value * 100.0)
            .round()
            .clamp(i32::MIN as f32, i32::MAX as f32) as i32
    }
}
