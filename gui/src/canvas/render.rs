use super::background;
use super::canvas::Canvas;
use crate::renderer::Renderer;

impl Canvas {
    pub fn render(&self, renderer: &mut Renderer, viewport_w: f32, viewport_h: f32) {
        background::render(renderer, &self.camera, viewport_w, viewport_h);
    }
}
