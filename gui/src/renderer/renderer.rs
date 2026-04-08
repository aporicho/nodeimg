use std::sync::Arc;
use winit::dpi::PhysicalSize;

use super::buffer::SharedViewport;
use super::command::DrawCommand;
use super::dispatch;
use super::pipeline::blit::{self, BlitPipeline};
use super::pipeline::circle::{CirclePipeline, CircleRequest};
use super::pipeline::curve::{CurvePipeline, CurveRequest};
use super::pipeline::image::ImagePipeline;
use super::pipeline::quad::{QuadPipeline, QuadRequest};
use super::pipeline::shadow::{ShadowPipeline, ShadowRequest};
use super::pipeline::stencil::StencilState;
use super::pipeline::text::{TextPipeline, TextRequest};
use super::style::{RectStyle, TextStyle};
use super::types::{Color, Point, Rect};

pub const MSAA_SAMPLE_COUNT: u32 = 4;
const DEFAULT_RENDER_SCALE: f32 = 2.0;

pub struct Renderer {
    shared_viewport: SharedViewport,
    quad_pipeline: QuadPipeline,
    text_pipeline: TextPipeline,
    image_pipeline: ImagePipeline,
    circle_pipeline: CirclePipeline,
    curve_pipeline: CurvePipeline,
    shadow_pipeline: ShadowPipeline,
    stencil: StencilState,
    msaa_view: wgpu::TextureView,
    resolve_view: wgpu::TextureView,
    blit: BlitPipeline,
    format: wgpu::TextureFormat,
    render_scale: f32,
    clear_color: Color,
    commands: Vec<DrawCommand>,
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

        Self {
            shared_viewport: SharedViewport::new(device),
            quad_pipeline: QuadPipeline::new(device, format, ms),
            text_pipeline: TextPipeline::new(device, queue, format, ms),
            image_pipeline: ImagePipeline::new(device, format, ms),
            circle_pipeline: CirclePipeline::new(device, format, ms),
            curve_pipeline: CurvePipeline::new(device, format, ms),
            shadow_pipeline: ShadowPipeline::new(device, format, ms),
            stencil: StencilState::new(device, internal, format, ms),
            msaa_view: create_msaa_texture(device, format, internal),
            resolve_view: blit::create_resolve_texture(device, format, internal),
            blit: BlitPipeline::new(device, format),
            format,
            render_scale,
            clear_color: Color::BLACK,
            commands: Vec::new(),
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

    pub fn begin_frame(&mut self, view: wgpu::TextureView, size: PhysicalSize<u32>, scale_factor: f64) {
        self.commands.clear();
        self.stencil.reset();
        self.frame = Some(FrameState { view, size, scale_factor });
    }

    pub fn measure_text(&mut self, text: &str, size: f32) -> (f32, f32) {
        self.text_pipeline.measure(text, size)
    }

    pub fn draw_rect(&mut self, rect: Rect, style: &RectStyle) {
        if let Some(ref shadow) = style.shadow {
            self.commands.push(DrawCommand::Shadow(ShadowRequest {
                rect,
                radius: style.radius,
                shadow: shadow.clone(),
            }));
        }
        self.commands
            .push(DrawCommand::Rect(QuadRequest::from_style(rect, style)));
    }

    pub fn draw_text(&mut self, pos: Point, text: &str, style: &TextStyle) {
        self.commands.push(DrawCommand::Text(TextRequest {
            pos,
            text: text.to_string(),
            style: TextStyle {
                color: style.color,
                size: style.size,
            },
        }));
    }

    pub fn draw_image(&mut self, rect: Rect, view: Arc<wgpu::TextureView>) {
        self.commands.push(DrawCommand::Image { rect, view });
    }

    pub fn draw_circle(&mut self, center: Point, radius: f32, color: Color) {
        self.commands.push(DrawCommand::Circle(CircleRequest {
            center,
            radius,
            color,
        }));
    }

    pub fn draw_curve(&mut self, points: [Point; 4], width: f32, color: Color) {
        self.commands.push(DrawCommand::Curve(CurveRequest {
            points,
            width,
            color,
        }));
    }

    pub fn push_clip(&mut self, rect: Rect, radius: f32) {
        self.commands.push(DrawCommand::PushClip { rect, radius });
    }

    pub fn pop_clip(&mut self) {
        self.commands.push(DrawCommand::PopClip);
    }

    pub fn end_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(frame) = self.frame.take() else {
            return;
        };

        let internal = scale_size(frame.size, self.render_scale);

        dispatch::dispatch(
            &self.commands,
            &frame.view,
            &self.msaa_view,
            &self.resolve_view,
            internal,
            frame.scale_factor,
            self.render_scale,
            self.clear_color,
            device,
            queue,
            &self.blit,
            &mut self.shared_viewport,
            &mut self.quad_pipeline,
            &mut self.text_pipeline,
            &mut self.image_pipeline,
            &mut self.circle_pipeline,
            &mut self.curve_pipeline,
            &mut self.shadow_pipeline,
            &mut self.stencil,
        );
        self.shadow_pipeline.evict_cache();
    }
}
