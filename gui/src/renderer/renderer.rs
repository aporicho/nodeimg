use std::sync::Arc;
use winit::dpi::PhysicalSize;

use super::buffer::SharedViewport;
use super::command::DrawCommand;
use super::curve::{CurvePipeline, CurveRequest};
use super::dispatch;
use super::image::ImagePipeline;
use super::quad::{QuadPipeline, QuadRequest};
use super::shadow::{ShadowPipeline, ShadowRequest};
use super::stencil::StencilState;
use super::style::{RectStyle, TextStyle};
use super::text::{TextPipeline, TextRequest};
use super::types::{Color, Point, Rect};

pub const MSAA_SAMPLE_COUNT: u32 = 4;

pub struct Renderer {
    shared_viewport: SharedViewport,
    quad_pipeline: QuadPipeline,
    text_pipeline: TextPipeline,
    image_pipeline: ImagePipeline,
    curve_pipeline: CurvePipeline,
    shadow_pipeline: ShadowPipeline,
    stencil: StencilState,
    msaa_view: wgpu::TextureView,
    format: wgpu::TextureFormat,
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
        Self {
            shared_viewport: SharedViewport::new(device),
            quad_pipeline: QuadPipeline::new(device, format, ms),
            text_pipeline: TextPipeline::new(device, queue, format, ms),
            image_pipeline: ImagePipeline::new(device, format, ms),
            curve_pipeline: CurvePipeline::new(device, format, ms),
            shadow_pipeline: ShadowPipeline::new(device, format, ms),
            stencil: StencilState::new(device, size, format, ms),
            msaa_view: create_msaa_texture(device, format, size),
            format,
            commands: Vec::new(),
            frame: None,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
        self.stencil.resize(device, size);
        self.msaa_view = create_msaa_texture(device, self.format, size);
    }

    pub fn begin_frame(&mut self, view: wgpu::TextureView, size: PhysicalSize<u32>, scale_factor: f64) {
        self.commands.clear();
        self.stencil.reset();
        self.frame = Some(FrameState { view, size, scale_factor });
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

        dispatch::dispatch(
            &self.commands,
            &frame.view,
            &self.msaa_view,
            frame.size,
            frame.scale_factor,
            device,
            queue,
            &mut self.shared_viewport,
            &mut self.quad_pipeline,
            &mut self.text_pipeline,
            &mut self.image_pipeline,
            &mut self.curve_pipeline,
            &mut self.shadow_pipeline,
            &mut self.stencil,
        );
        self.shadow_pipeline.evict_cache();
    }
}
