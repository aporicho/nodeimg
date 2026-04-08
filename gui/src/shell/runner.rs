use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

use super::app::App;
use super::context::AppContext;
use super::event::AppEvent;
use super::translator::EventTranslator;
use super::{gpu, surface, window};
use crate::renderer::Renderer;

struct Runner<A: App> {
    state: Option<RunnerState<A>>,
}

struct RunnerState<A: App> {
    app: A,
    ctx: AppContext,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    events: EventTranslator,
}

impl<A: App> Runner<A> {
    fn new() -> Self {
        Self { state: None }
    }
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let win = window::create_window(event_loop);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surf = surface::create_surface(&instance, Arc::clone(&win));
        let (adapter, device, queue) = gpu::init_gpu(&instance, &surf);

        let size = win.inner_size();
        let scale_factor = win.scale_factor();
        tracing::info!("Window: {}x{}, scale_factor: {}", size.width, size.height, scale_factor);
        let surface_config = surface::configure(&surf, &adapter, &device, size);

        let renderer = Renderer::new(&device, &queue, surface_config.format, size);

        let mut ctx = AppContext {
            device,
            queue,
            window: win,
            surface_config,
            size,
            scale_factor,
            cursor: super::cursor::CursorState::new(),
        };

        let app = A::init(&mut ctx);

        self.state = Some(RunnerState {
            app,
            ctx,
            surface: surf,
            renderer,
            events: EventTranslator::new(scale_factor),
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        // 处理 resize
        if let WindowEvent::Resized(new_size) = event {
            state.ctx.size = new_size;
            surface::resize(
                &state.surface,
                &state.ctx.device,
                &mut state.ctx.surface_config,
                new_size,
            );
            state.renderer.resize(&state.ctx.device, new_size);
        }

        // 翻译并分发事件
        if let Some(app_event) = state.events.translate(&event) {
            match app_event {
                AppEvent::CloseRequested => {
                    event_loop.exit();
                    return;
                }
                _ => state.app.event(app_event, &mut state.ctx),
            }
        }

        // 请求重绘
        if let WindowEvent::RedrawRequested = event {
            state.ctx.cursor.reset();
            state.app.update(&mut state.ctx);
            state.ctx.cursor.apply(&state.ctx.window);

            let output = match state.surface.get_current_texture() {
                Ok(tex) => tex,
                Err(_) => return,
            };
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            state.renderer.begin_frame(view, state.ctx.size, state.ctx.scale_factor);
            state.app.render(&mut state.renderer, &state.ctx);
            state.renderer.end_frame(&state.ctx.device, &state.ctx.queue);

            output.present();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.ctx.window.request_redraw();
        }
    }
}

pub fn run<A: App>() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut runner = Runner::<A>::new();
    event_loop.run_app(&mut runner).expect("event loop error");
}
