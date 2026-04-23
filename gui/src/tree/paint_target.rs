use std::collections::HashMap;
use std::sync::Arc;

use super::layout::TextureHandle;
use crate::renderer::{Color, PathData, PathStyle, Point, Rect, RectStyle, Renderer, TextStyle};

pub trait PaintTarget {
    fn draw_rect(&mut self, rect: Rect, style: RectStyle);
    fn draw_text(&mut self, pos: Point, text: &str, style: TextStyle);
    fn draw_text_clipped(&mut self, pos: Point, text: &str, style: TextStyle, bounds: Rect);
    fn draw_image(&mut self, rect: Rect, texture: TextureHandle);
    fn draw_circle(&mut self, center: Point, radius: f32, color: Color);
    fn draw_path(&mut self, data: PathData, style: PathStyle);
    fn push_clip(&mut self, rect: Rect, radius: f32);
    fn pop_clip(&mut self);
    fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32);
}

pub struct RendererPaintTarget<'a> {
    renderer: &'a mut Renderer,
    textures: Option<&'a HashMap<TextureHandle, Arc<wgpu::TextureView>>>,
}

impl<'a> RendererPaintTarget<'a> {
    pub fn new(
        renderer: &'a mut Renderer,
        textures: Option<&'a HashMap<TextureHandle, Arc<wgpu::TextureView>>>,
    ) -> Self {
        Self { renderer, textures }
    }
}

impl PaintTarget for RendererPaintTarget<'_> {
    fn draw_rect(&mut self, rect: Rect, style: RectStyle) {
        self.renderer.draw_rect(rect, &style);
    }

    fn draw_text(&mut self, pos: Point, text: &str, style: TextStyle) {
        self.renderer.draw_text(pos, text, &style);
    }

    fn draw_text_clipped(&mut self, pos: Point, text: &str, style: TextStyle, bounds: Rect) {
        self.renderer.draw_text_clipped(pos, text, &style, bounds);
    }

    fn draw_image(&mut self, rect: Rect, texture: TextureHandle) {
        if let Some(view) = self
            .textures
            .and_then(|registry| registry.get(&texture))
            .cloned()
        {
            self.renderer.draw_image(rect, view);
        }
    }

    fn draw_circle(&mut self, center: Point, radius: f32, color: Color) {
        self.renderer.draw_circle(center, radius, color);
    }

    fn draw_path(&mut self, data: PathData, style: PathStyle) {
        self.renderer.draw_path(data, style);
    }

    fn push_clip(&mut self, rect: Rect, radius: f32) {
        self.renderer.push_clip(rect, radius);
    }

    fn pop_clip(&mut self) {
        self.renderer.pop_clip();
    }

    fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        self.renderer
            .text_measurer()
            .measure_with_style(text, style)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomPaintCx {
    pub rect: Rect,
}
