use std::collections::HashMap;

use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use winit::dpi::PhysicalSize;

use super::style::TextStyle;
use super::types::{Color, Point};

pub struct TextRequest {
    pub pos: Point,
    pub text: String,
    pub style: TextStyle,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct TextCacheKey {
    text: String,
    size_bits: u32,
}

impl TextCacheKey {
    fn new(text: &str, size: f32) -> Self {
        Self {
            text: text.to_string(),
            size_bits: size.to_bits(),
        }
    }
}

struct CachedBuffer {
    buffer: Buffer,
    used: bool,
}

pub struct TextPipeline {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    #[allow(dead_code)]
    cache: Cache,
    buffer_cache: HashMap<TextCacheKey, CachedBuffer>,
}

impl TextPipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            multisample,
            Some(super::stencil::content_depth_stencil_state()),
        );
        let viewport = Viewport::new(device, &cache);

        Self {
            font_system,
            swash_cache,
            atlas,
            text_renderer,
            viewport,
            cache,
            buffer_cache: HashMap::new(),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texts: &[TextRequest],
        size: PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        self.viewport.update(
            queue,
            Resolution {
                width: size.width,
                height: size.height,
            },
        );

        // 标记所有缓存为未使用
        for entry in self.buffer_cache.values_mut() {
            entry.used = false;
        }

        // 确保每个文本都有缓存的 Buffer
        for req in texts {
            let key = TextCacheKey::new(&req.text, req.style.size);
            if let Some(entry) = self.buffer_cache.get_mut(&key) {
                entry.used = true;
            } else {
                let mut buffer = Buffer::new(
                    &mut self.font_system,
                    Metrics::new(req.style.size, req.style.size * 1.2),
                );
                buffer.set_size(
                    &mut self.font_system,
                    Some(f32::MAX),
                    Some(req.style.size * 2.0),
                );
                buffer.set_text(
                    &mut self.font_system,
                    &req.text,
                    &Attrs::new().family(Family::SansSerif),
                    Shaping::Advanced,
                    None,
                );
                buffer.shape_until_scroll(&mut self.font_system, false);
                self.buffer_cache.insert(key, CachedBuffer { buffer, used: true });
            }
        }

        // 清理未使用的缓存
        self.buffer_cache.retain(|_, v| v.used);

        let sf = scale_factor as f32;

        let text_areas: Vec<TextArea<'_>> = texts
            .iter()
            .map(|req| {
                let key = TextCacheKey::new(&req.text, req.style.size);
                let buffer = &self.buffer_cache[&key].buffer;
                TextArea {
                    buffer,
                    left: req.pos.x * sf,
                    top: req.pos.y * sf,
                    scale: sf,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: size.width as i32,
                        bottom: size.height as i32,
                    },
                    default_color: to_glyphon_color(req.style.color),
                    custom_glyphs: &[],
                }
            })
            .collect();

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("failed to prepare text");
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.text_renderer
            .render(&self.atlas, &self.viewport, pass)
            .expect("failed to render text");
    }
}

fn to_glyphon_color(c: Color) -> GlyphonColor {
    GlyphonColor::rgba(
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
        (c.a * 255.0) as u8,
    )
}
