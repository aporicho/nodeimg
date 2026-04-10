use glyphon::{
    Cache, Color as GlyphonColor, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use winit::dpi::PhysicalSize;

use super::super::style::TextStyle;
use super::super::text_measurer::{TextCacheKey, TextMeasurer};
use super::super::types::{Color, Point};

pub struct TextRequest {
    pub pos: Point,
    pub text: String,
    pub style: TextStyle,
}

pub struct TextPipeline {
    swash_cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    #[allow(dead_code)]
    cache: Cache,
}

impl TextPipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
        _font_system: &mut glyphon::FontSystem,
    ) -> Self {
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
            swash_cache,
            atlas,
            text_renderer,
            viewport,
            cache,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texts: &[TextRequest],
        size: PhysicalSize<u32>,
        scale_factor: f64,
        text_measurer: &mut TextMeasurer,
    ) {
        self.viewport.update(
            queue,
            Resolution {
                width: size.width,
                height: size.height,
            },
        );

        // Phase 1 (mutable): mark all unused, ensure buffers exist for each request
        text_measurer.mark_all_unused();

        for req in texts {
            text_measurer.ensure_buffer(&req.text, req.style.size);
        }

        // Phase 2 (immutable borrow of buffer_cache): build TextArea references
        let sf = scale_factor as f32;

        let text_areas: Vec<TextArea<'_>> = texts
            .iter()
            .map(|req| {
                let key = TextCacheKey::new(&req.text, req.style.size);
                let buffer = &text_measurer.buffer_cache[&key].buffer;
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
                &mut text_measurer.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("failed to prepare text");

        // Evict unused buffers after prepare
        text_measurer.evict_unused();
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
