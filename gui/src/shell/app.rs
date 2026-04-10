use super::context::AppContext;
use super::event::AppEvent;
use crate::renderer::Renderer;

pub trait App: Sized + 'static {
    fn init(ctx: &mut AppContext) -> Self;
    fn event(&mut self, event: AppEvent, ctx: &mut AppContext);
    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext);
    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext);
}
