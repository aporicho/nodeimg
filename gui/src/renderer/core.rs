use winit::dpi::PhysicalSize;

use crate::paint::DisplayList;

use super::buffer::SharedViewport;
use super::command::BackendCommand;
use super::dispatch;
use super::display_backend::{lower_display_list, DisplayRenderReport};
use super::display_resources::DisplayResourceResolver;
use super::pipeline::blit::{self, BlitPipeline};
use super::pipeline::circle::CirclePipeline;
use super::pipeline::image::ImagePipeline;
use super::pipeline::quad::QuadPipeline;
use super::pipeline::shadow::ShadowPipeline;
use super::pipeline::stencil::StencilState;
use super::pipeline::text::TextPipeline;
use super::pipeline::vector::VectorPipeline;
use super::svg::{SvgRasterCache, SvgVectorCache};
use super::text_measurer::TextMeasurer;
use super::types::Color;
use super::vector_tessellator::VectorTessellator;

pub const MSAA_SAMPLE_COUNT: u32 = 4;
const DEFAULT_RENDER_SCALE: f32 = 2.0;

pub struct Renderer {
    shared_viewport: SharedViewport,
    quad_pipeline: QuadPipeline,
    text_measurer: TextMeasurer,
    text_pipeline: TextPipeline,
    image_pipeline: ImagePipeline,
    circle_pipeline: CirclePipeline,
    vector_pipeline: VectorPipeline,
    vector_tessellator: VectorTessellator,
    svg_vector_cache: SvgVectorCache,
    svg_raster_cache: SvgRasterCache,
    shadow_pipeline: ShadowPipeline,
    stencil: StencilState,
    msaa_view: wgpu::TextureView,
    resolve_view: wgpu::TextureView,
    blit: BlitPipeline,
    format: wgpu::TextureFormat,
    render_scale: f32,
    clear_color: Color,
    backend_commands: Vec<BackendCommand>,
    frame: Option<FrameState>,
}

struct FrameState {
    view: wgpu::TextureView,
    size: PhysicalSize<u32>,
    scale_factor: f64,
}

fn msaa_multisample_state() -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count: MSAA_SAMPLE_COUNT,
        mask: !0,
        alpha_to_coverage_enabled: false,
    }
}

pub(super) fn scale_size(size: PhysicalSize<u32>, scale: f32) -> PhysicalSize<u32> {
    PhysicalSize::new(
        ((size.width as f32 * scale) as u32).max(1),
        ((size.height as f32 * scale) as u32).max(1),
    )
}

fn create_msaa_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: PhysicalSize<u32>,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_texture"),
        size: wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: MSAA_SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        size: PhysicalSize<u32>,
    ) -> Self {
        let ms = msaa_multisample_state();
        let render_scale = DEFAULT_RENDER_SCALE;
        let internal = scale_size(size, render_scale);
        let mut text_measurer = TextMeasurer::new();

        Self {
            shared_viewport: SharedViewport::new(device),
            quad_pipeline: QuadPipeline::new(device, format, ms),
            text_pipeline: TextPipeline::new(
                device,
                queue,
                format,
                ms,
                &mut text_measurer.font_system,
            ),
            text_measurer,
            image_pipeline: ImagePipeline::new(device, format, ms),
            circle_pipeline: CirclePipeline::new(device, format, ms),
            vector_pipeline: VectorPipeline::new(device, format, ms),
            vector_tessellator: VectorTessellator::new(),
            svg_vector_cache: SvgVectorCache::new(),
            svg_raster_cache: SvgRasterCache::new(),
            shadow_pipeline: ShadowPipeline::new(device, format, ms),
            stencil: StencilState::new(device, internal, format, ms),
            msaa_view: create_msaa_texture(device, format, internal),
            resolve_view: blit::create_resolve_texture(device, format, internal),
            blit: BlitPipeline::new(device, format),
            format,
            render_scale,
            clear_color: Color::BLACK,
            backend_commands: Vec::new(),
            frame: None,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
        let internal = scale_size(size, self.render_scale);
        self.stencil.resize(device, internal);
        self.msaa_view = create_msaa_texture(device, self.format, internal);
        self.resolve_view = blit::create_resolve_texture(device, self.format, internal);
    }

    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    pub fn begin_frame(
        &mut self,
        view: wgpu::TextureView,
        size: PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        self.backend_commands.clear();
        self.stencil.reset();
        self.frame = Some(FrameState {
            view,
            size,
            scale_factor,
        });
    }

    pub fn measure_text(&mut self, text: &str, size: f32) -> (f32, f32) {
        self.text_measurer.measure(text, size)
    }

    pub fn text_measurer(&mut self) -> &mut TextMeasurer {
        &mut self.text_measurer
    }

    pub(crate) fn draw_display_list<R: DisplayResourceResolver>(
        &mut self,
        list: &DisplayList,
        resources: &R,
    ) -> DisplayRenderReport {
        let output = lower_display_list(list, resources, &mut self.svg_vector_cache);
        self.backend_commands.extend(output.commands);
        output.report
    }

    pub fn end_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(frame) = self.frame.take() else {
            return;
        };

        let internal = scale_size(frame.size, self.render_scale);

        dispatch::dispatch(
            &self.backend_commands,
            dispatch::DispatchFrame {
                frame_view: &frame.view,
                msaa_view: &self.msaa_view,
                resolve_view: &self.resolve_view,
                internal_size: internal,
                scale_factor: frame.scale_factor,
                render_scale: self.render_scale,
                format: self.format,
                clear_color: self.clear_color,
                device,
                queue,
            },
            dispatch::DispatchPipelines {
                blit: &self.blit,
                shared_viewport: &mut self.shared_viewport,
                quad_pipeline: &mut self.quad_pipeline,
                text_pipeline: &mut self.text_pipeline,
                image_pipeline: &mut self.image_pipeline,
                circle_pipeline: &mut self.circle_pipeline,
                vector_pipeline: &mut self.vector_pipeline,
                vector_tessellator: &mut self.vector_tessellator,
                svg_raster_cache: &mut self.svg_raster_cache,
                shadow_pipeline: &mut self.shadow_pipeline,
                stencil: &mut self.stencil,
                text_measurer: &mut self.text_measurer,
            },
        );
        self.shadow_pipeline.evict_cache();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::geometry::{Affine2D, Point, Rect};
    use crate::paint::{
        ClipShape, Color, DisplayList, DisplayListBuilder, ImagePaint, PaintCommand, RectPaint,
        RectStyle, Shadow, ShadowPaint, SvgFit, SvgPaint, SvgSourceKey, SvgStyle, TextPaint,
        TextStyle, TextureHandle,
    };
    use crate::renderer::display_resources::DisplayResourceResolver;
    use crate::renderer::svg::SvgSource;
    use crate::renderer::test_support::try_test_device;
    use crate::renderer::{ImageStyle, TextureResource, TextureSize};

    use super::*;

    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    #[derive(Default)]
    struct SmokeResources {
        textures: HashMap<TextureHandle, TextureResource>,
        svg_sources: HashMap<String, SvgSource>,
        _texture_storage: Vec<wgpu::Texture>,
    }

    impl SmokeResources {
        fn insert_texture(
            &mut self,
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            handle: TextureHandle,
        ) {
            let size = TextureSize::new(2, 2);
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("renderer_smoke_texture"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &[
                    255, 255, 255, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255,
                ],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(8),
                    rows_per_image: Some(2),
                },
                wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            );
            let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
            self.textures
                .insert(handle, TextureResource::new(view, size));
            self._texture_storage.push(texture);
        }

        fn insert_svg(&mut self, id: &str, svg: &'static [u8]) {
            self.svg_sources
                .insert(id.to_string(), SvgSource::new(id, Arc::from(svg)));
        }
    }

    impl DisplayResourceResolver for SmokeResources {
        fn texture(&self, handle: TextureHandle) -> Option<TextureResource> {
            self.textures.get(&handle).cloned()
        }

        fn svg_source(&self, key: &SvgSourceKey) -> Option<SvgSource> {
            self.svg_sources.get(&key.id).cloned()
        }
    }

    #[test]
    fn gpu_smoke_covers_affine_image_clip_text_shadow_and_svg_raster() {
        let Some((device, queue)) = try_test_device("affine-renderer-smoke") else {
            return;
        };
        let size = PhysicalSize::new(96, 96);
        let mut renderer = Renderer::new(&device, &queue, FORMAT, size);
        let mut resources = SmokeResources::default();
        resources.insert_texture(&device, &queue, TextureHandle(1));
        resources.insert_svg(
            "gradient-fallback",
            br##"<svg width="20" height="10" viewBox="0 0 20 10" xmlns="http://www.w3.org/2000/svg">
<defs><linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient></defs>
<rect width="20" height="10" fill="url(#g)"/>
</svg>"##,
        );

        let list = smoke_display_list();
        let (frame_texture, frame_view) = frame_target(&device, size);
        renderer.begin_frame(frame_view, size, 1.0);
        let report = renderer.draw_display_list(&list, &resources);
        assert!(report.unsupported.is_empty(), "{:?}", report.unsupported);
        renderer.end_frame(&device, &queue);

        drop(frame_texture);
    }

    fn smoke_display_list() -> DisplayList {
        let mut builder = DisplayListBuilder::new();

        builder.push_transform(Affine2D::compose(
            Affine2D::translation(20.0, 8.0),
            Affine2D::rotation_radians(0.2),
        ));
        builder.draw(PaintCommand::Image(ImagePaint {
            rect: rect(0.0, 0.0, 18.0, 16.0),
            texture: TextureHandle(1),
            style: ImageStyle::default(),
        }));
        builder.pop_transform();

        builder.push_clip(ClipShape::Rect(rect(4.0, 4.0, 68.0, 68.0)));
        builder.push_transform(Affine2D::compose(
            Affine2D::translation(40.0, 14.0),
            Affine2D::rotation_radians(0.3),
        ));
        builder.push_clip(ClipShape::RoundedRect {
            rect: rect(0.0, 0.0, 32.0, 24.0),
            radius: [4.0; 4],
        });
        builder.pop_transform();
        builder.draw(PaintCommand::Rect(RectPaint {
            rect: rect(12.0, 18.0, 42.0, 28.0),
            style: rect_style(Color::WHITE),
        }));
        builder.pop_clip();
        builder.pop_clip();

        builder.push_transform(Affine2D::compose(
            Affine2D::translation(8.0, 60.0),
            Affine2D::rotation_radians(-0.25),
        ));
        builder.draw(PaintCommand::Text(TextPaint {
            pos: Point { x: 0.0, y: 0.0 },
            text: "Affine".to_string(),
            style: TextStyle::new(Color::WHITE, 14.0),
            bounds: Some(rect(0.0, 0.0, 54.0, 18.0)),
        }));
        builder.pop_transform();

        builder.push_transform(Affine2D::compose(
            Affine2D::translation(52.0, 48.0),
            Affine2D::rotation_radians(0.15),
        ));
        builder.draw(PaintCommand::Shadow(ShadowPaint {
            rect: rect(0.0, 0.0, 24.0, 18.0),
            radius: [3.0; 4],
            shadow: Shadow {
                color: Color {
                    a: 0.5,
                    ..Color::BLACK
                },
                offset: [2.0, 3.0],
                blur: 4.0,
                spread: 1.0,
            },
        }));
        builder.pop_transform();

        builder.push_transform(Affine2D::compose(
            Affine2D::translation(58.0, 18.0),
            Affine2D::rotation_radians(0.35),
        ));
        builder.draw(PaintCommand::Svg(SvgPaint {
            rect: rect(0.0, 0.0, 30.0, 16.0),
            source: SvgSourceKey::new("gradient-fallback"),
            style: SvgStyle::new(Color::WHITE).with_fit(SvgFit::Stretch),
        }));
        builder.pop_transform();

        builder.finish().expect("smoke list should balance stacks")
    }

    fn frame_target(
        device: &wgpu::Device,
        size: PhysicalSize<u32>,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("renderer_smoke_frame"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn rect_style(color: Color) -> RectStyle {
        RectStyle {
            color,
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }
    }
}
