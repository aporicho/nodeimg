use glyphon::{
    Cache, Color as GlyphonColor, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use winit::dpi::PhysicalSize;

use super::super::style::TextStyle;
use super::super::text_measurer::{TextCacheKey, TextMeasurer};
use super::super::types::{Color, Point, Rect};

#[derive(Clone)]
pub struct TextRequest {
    pub pos: Point,
    pub text: String,
    pub style: TextStyle,
    pub bounds: Option<Rect>,
}

struct PreparedTextBatch {
    renderer: TextRenderer,
    #[cfg(test)]
    request_count: usize,
}

pub struct TextPipeline {
    swash_cache: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    prepared_batches: Vec<PreparedTextBatch>,
    multisample: wgpu::MultisampleState,
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
        let atlas = TextAtlas::new(device, queue, &cache, format);
        let viewport = Viewport::new(device, &cache);

        Self {
            swash_cache,
            atlas,
            viewport,
            prepared_batches: Vec::new(),
            multisample,
            cache,
        }
    }

    pub fn begin_frame(&mut self) {
        self.prepared_batches.clear();
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texts: &[TextRequest],
        size: PhysicalSize<u32>,
        scale_factor: f64,
        text_measurer: &mut TextMeasurer,
    ) -> usize {
        self.viewport.update(
            queue,
            Resolution {
                width: size.width,
                height: size.height,
            },
        );

        for req in texts {
            text_measurer.ensure_buffer(&req.text, &req.style);
        }

        let sf = scale_factor as f32;

        let text_areas: Vec<TextArea<'_>> = texts
            .iter()
            .map(|req| {
                let key = TextCacheKey::new(&req.text, &req.style);
                let buffer = &text_measurer.buffer_cache[&key].buffer;
                TextArea {
                    buffer,
                    left: req.pos.x * sf,
                    top: req.pos.y * sf,
                    scale: sf,
                    bounds: text_bounds(req.bounds, size, sf),
                    default_color: to_glyphon_color(req.style.color),
                    custom_glyphs: &[],
                }
            })
            .collect();

        let mut renderer = TextRenderer::new(
            &mut self.atlas,
            device,
            self.multisample,
            Some(super::stencil::content_depth_stencil_state()),
        );

        renderer
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

        let batch_index = self.prepared_batches.len();
        self.prepared_batches.push(PreparedTextBatch {
            renderer,
            #[cfg(test)]
            request_count: texts.len(),
        });
        batch_index
    }

    pub fn render_batch<'a>(&'a self, batch_index: usize, pass: &mut wgpu::RenderPass<'a>) {
        self.prepared_batches[batch_index]
            .renderer
            .render(&self.atlas, &self.viewport, pass)
            .expect("failed to render text");
    }

    #[cfg(test)]
    fn prepared_batch_count_for_test(&self) -> usize {
        self.prepared_batches.len()
    }

    #[cfg(test)]
    fn prepared_request_count_for_test(&self, index: usize) -> Option<usize> {
        self.prepared_batches
            .get(index)
            .map(|batch| batch.request_count)
    }
}

fn text_bounds(bounds: Option<Rect>, size: PhysicalSize<u32>, scale: f32) -> TextBounds {
    match bounds {
        Some(rect) => TextBounds {
            left: (rect.x * scale).floor() as i32,
            top: (rect.y * scale).floor() as i32,
            right: ((rect.x + rect.w) * scale).ceil() as i32,
            bottom: ((rect.y + rect.h) * scale).ceil() as i32,
        },
        None => TextBounds {
            left: 0,
            top: 0,
            right: size.width as i32,
            bottom: size.height as i32,
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::test_support::try_test_device;

    fn test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        try_test_device("text-pipeline-test-device")
    }

    #[test]
    fn pipeline_keeps_multiple_prepared_batches() {
        let Some((device, queue)) = test_device() else {
            return;
        };
        let mut font_system = glyphon::FontSystem::new();
        let mut pipeline = TextPipeline::new(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::MultisampleState::default(),
            &mut font_system,
        );
        let mut measurer = TextMeasurer::new();

        pipeline.begin_frame();
        let first = pipeline.prepare(
            &device,
            &queue,
            &[TextRequest {
                pos: Point { x: 0.0, y: 0.0 },
                text: "first".to_string(),
                style: TextStyle::new(Color::WHITE, 12.0),
                bounds: None,
            }],
            PhysicalSize::new(800, 600),
            1.0,
            &mut measurer,
        );
        let second = pipeline.prepare(
            &device,
            &queue,
            &[TextRequest {
                pos: Point { x: 10.0, y: 10.0 },
                text: "second".to_string(),
                style: TextStyle::new(Color::WHITE, 12.0),
                bounds: None,
            }],
            PhysicalSize::new(800, 600),
            1.0,
            &mut measurer,
        );

        assert_eq!(first, 0);
        assert_eq!(second, 1);
        assert_eq!(pipeline.prepared_batch_count_for_test(), 2);
        assert_eq!(pipeline.prepared_request_count_for_test(0), Some(1));
        assert_eq!(pipeline.prepared_request_count_for_test(1), Some(1));
    }
}
