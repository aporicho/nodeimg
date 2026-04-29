use crate::workspace::controller::{WorkspaceActionResult, WorkspaceController};
use crate::workspace::scene_controller::{WorkspaceSceneController, WorkspaceSceneInput};
use gui::action::{node_library_add_type_id, GuiAction};
use gui::canvas::camera::Camera;
use gui::canvas::navigation::CanvasNavigationController;
use gui::canvas::node_template::CanvasNodeRenderView;
use gui::context::{Context, FrameworkOutput, GuiEvent, PlatformEffect, WidgetEvent};
use gui::diagnostics::render_trace::{self, RectSummary, RenderTraceStage, TARGET_RENDER};
use gui::gesture::Gesture;
use gui::renderer::{Rect, Renderer, TextureSize};
use gui::shell::{App, AppContext, AppEvent, CursorStyle, Key, MouseButton};
use gui::theme::{light_theme, Theme};
use gui::tree::layout::TextureHandle;
use gui::widget::resize_edge::ResizeEdge;
use std::time::{Duration, Instant};

const APP_THEME_SCALE: f32 = 1.2;
const SAMPLE_IMAGE_HANDLE: TextureHandle = TextureHandle(1);
const SAMPLE_IMAGE_SIZE: TextureSize = TextureSize::new(96, 96);
const DEVELOPER_MODE_ENABLED: bool = cfg!(debug_assertions);
const CANVAS_DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_millis(300);
const CANVAS_DOUBLE_CLICK_DISTANCE_SQ: f32 = 36.0;

#[derive(Debug)]
enum AppMessage {
    WidgetClicked(String),
    WidgetDragStart {
        id: String,
        x: f32,
        y: f32,
    },
    WidgetDragMove {
        id: String,
        x: f32,
        y: f32,
    },
    WidgetDragEnd {
        id: String,
        x: f32,
        y: f32,
    },
    WidgetResizeStart {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
    WidgetResizeMove {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
    WidgetResizeEnd {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
    WidgetTextChanged {
        id: String,
        value: String,
    },
    LongPress(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppMode {
    User,
    Developer,
}

pub(crate) fn toggle_app_mode(current: AppMode, developer_mode_enabled: bool) -> AppMode {
    if !developer_mode_enabled {
        return AppMode::User;
    }

    match current {
        AppMode::User => AppMode::Developer,
        AppMode::Developer => AppMode::User,
    }
}

pub(crate) fn initial_app_mode(developer_mode_enabled: bool) -> AppMode {
    if developer_mode_enabled {
        AppMode::Developer
    } else {
        AppMode::User
    }
}

pub struct AppShell {
    gui: Context,
    mode: AppMode,
    camera: Camera,
    navigation: CanvasNavigationController,
    workspace: WorkspaceController,
    scene_controller: WorkspaceSceneController,
    last_canvas_click: Option<CanvasClick>,
    mouse_x: f32,
    mouse_y: f32,
    theme: Theme,
}

impl App for AppShell {
    fn init(ctx: &mut AppContext) -> Self {
        let mut gui = Context::new();
        gui.register_texture(
            SAMPLE_IMAGE_HANDLE,
            create_sample_texture(&ctx.device, &ctx.queue),
            SAMPLE_IMAGE_SIZE,
        );

        let mode = initial_app_mode(DEVELOPER_MODE_ENABLED);
        tracing::info!("initial app mode: {:?}", mode);

        Self {
            gui,
            mode,
            camera: Camera::new(),
            navigation: CanvasNavigationController::new(),
            workspace: WorkspaceController::new(),
            scene_controller: WorkspaceSceneController::default(),
            last_canvas_click: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
            theme: light_theme().scaled(APP_THEME_SCALE),
        }
    }

    fn event(&mut self, event: AppEvent, ctx: &mut AppContext) {
        if !matches!(event, AppEvent::MouseMove { .. }) {
            tracing::info!("event: {:?}", event);
        }

        self.update_mouse_position_from_event(&event);
        if self.handle_global_shortcut(&event) {
            return;
        }

        let output = self.gui.handle_event(&event);
        let consumed = output.consumed;
        self.handle_framework_output(output, ctx);

        let navigation_consumed =
            !consumed && self.navigation.handle_event(&event, &mut self.camera);
        if navigation_consumed {
            if self.navigation.is_panning() {
                ctx.cursor.set(CursorStyle::Move);
            }
            return;
        }

        self.handle_user_event(event, consumed, ctx);
    }

    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext) {
        self.gui.tick_animations(Instant::now());
        let viewport = viewport_rect(ctx);
        let canvas_nodes = self.sync_workspace_scene(viewport, renderer, true);
        self.gui.sync_retained_canvas_text_boxes(
            &canvas_nodes,
            renderer.text_measurer(),
            &self.theme,
        );
        if !self.gui.text_box_dirty_intrinsics().is_empty() {
            let stabilized_nodes = self.sync_workspace_scene(viewport, renderer, false);
            self.gui.sync_retained_canvas_text_boxes(
                &stabilized_nodes,
                renderer.text_measurer(),
                &self.theme,
            );
        }
        ctx.apply_ime_request(self.gui.ime_request());
        self.update_hover_cursor(self.mouse_x, self.mouse_y, ctx);
        if self.gui.animations_active() {
            ctx.request_redraw();
        }
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        let viewport = viewport_rect(ctx);
        renderer.set_clear_color(self.theme.colors.canvas_bg);
        self.gui
            .render(renderer, viewport.w, viewport.h, &self.theme);
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct AppUpdateTraceSummary {
    viewport: RectSummary,
    canvas_nodes: usize,
    canvas_connections: usize,
    pending_connection: bool,
    animations_active: bool,
    mode: AppMode,
}

fn create_sample_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> std::sync::Arc<wgpu::TextureView> {
    const WIDTH: u32 = SAMPLE_IMAGE_SIZE.width;
    const HEIGHT: u32 = SAMPLE_IMAGE_SIZE.height;
    let mut data = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let idx = ((y * WIDTH + x) * 4) as usize;
            let checker = ((x / 12) + (y / 12)) % 2 == 0;
            let (r, g, b) = if checker {
                (59u8, 130u8, 246u8)
            } else {
                (244u8, 244u8, 245u8)
            };
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sample_image_texture"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * WIDTH),
            rows_per_image: Some(HEIGHT),
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    std::sync::Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

impl AppShell {
    fn sync_workspace_scene(
        &mut self,
        viewport: Rect,
        renderer: &mut Renderer,
        trace_app_update: bool,
    ) -> Vec<CanvasNodeRenderView> {
        let canvas_nodes = self
            .workspace
            .canvas_node_render_views(&mut self.gui, &self.theme);
        let canvas_connections = self.workspace.canvas_connection_views();
        let engine_panel = self.workspace.engine_panel_state();
        let pending_connection = self.gui.pending_canvas_connection();
        if trace_app_update {
            render_trace::debug_stage(
                RenderTraceStage::AppUpdate,
                AppUpdateTraceSummary {
                    viewport: RectSummary::from(viewport),
                    canvas_nodes: canvas_nodes.len(),
                    canvas_connections: canvas_connections.len(),
                    pending_connection: pending_connection.is_some(),
                    animations_active: self.gui.animations_active(),
                    mode: self.mode,
                },
            );
        }
        if let Err(error) = self.scene_controller.sync(
            &mut self.gui,
            WorkspaceSceneInput {
                viewport,
                camera: &self.camera,
                canvas_nodes: &canvas_nodes,
                canvas_connections: &canvas_connections,
                pending_connection: pending_connection.as_ref(),
                theme: &self.theme,
                preview_image: SAMPLE_IMAGE_HANDLE,
                engine_panel: &engine_panel,
            },
        ) {
            tracing::warn!(
                target: TARGET_RENDER,
                frame_id = render_trace::current_render_trace_frame().id,
                ?error,
                "failed to sync retained workspace scene"
            );
        }
        self.gui
            .flush_layout_dirty(viewport, renderer.text_measurer());
        canvas_nodes
    }

    fn handle_global_shortcut(&mut self, event: &AppEvent) -> bool {
        if !DEVELOPER_MODE_ENABLED || !is_developer_mode_shortcut(event) {
            return false;
        }

        let next_mode = toggle_app_mode(self.mode, DEVELOPER_MODE_ENABLED);
        if next_mode != self.mode {
            self.mode = next_mode;
            self.gui.close_overlay();
            tracing::info!("app mode switched to {:?}", self.mode);
        }
        true
    }

    fn handle_user_event(&mut self, event: AppEvent, consumed: bool, ctx: &mut AppContext) {
        match event {
            AppEvent::KeyPress {
                key: Key::Escape, ..
            } if !consumed => {
                if self.scene_controller.node_palette_open() {
                    self.scene_controller.close_overlay();
                    return;
                }
                let _ = self.workspace.cancel_canvas_port_connection(&mut self.gui);
            }
            AppEvent::MouseMove { x, y } => {
                self.workspace.update_canvas_hover(&mut self.gui, x, y);
                self.update_hover_cursor(x, y, ctx);
            }
            AppEvent::MouseRelease {
                x,
                y,
                button: MouseButton::Left,
            } => {
                if !consumed {
                    self.handle_canvas_release(x, y);
                }
            }
            _ => {}
        }
    }

    fn update_mouse_position_from_event(&mut self, event: &AppEvent) {
        match *event {
            AppEvent::MouseMove { x, y }
            | AppEvent::MousePress { x, y, .. }
            | AppEvent::MouseRelease { x, y, .. }
            | AppEvent::ScrollLine { x, y, .. }
            | AppEvent::ScrollPixel { x, y, .. }
            | AppEvent::PinchZoom { x, y, .. } => {
                self.mouse_x = x;
                self.mouse_y = y;
            }
            _ => {}
        }
    }

    fn handle_framework_output(&mut self, output: FrameworkOutput, ctx: &mut AppContext) {
        let mut handled_node_adds = Vec::new();
        for action in output.actions {
            let result = self.handle_gui_action(action);
            if result.close_overlay {
                self.scene_controller.close_overlay();
                self.gui.close_overlay();
            }
            if let Some(type_id) = result.handled_node_add {
                handled_node_adds.push(type_id);
            }
        }

        for event in output.events {
            if self.is_duplicate_action_event(&event, &handled_node_adds) {
                continue;
            }
            if let GuiEvent::Panel(panel_event) = &event {
                if self.gui.handle_panel_event(panel_event) {
                    continue;
                }
            }
            self.handle_gui_event(event);
        }

        for effect in output.effects {
            match effect {
                PlatformEffect::WriteClipboard(text) => {
                    let _ = ctx.clipboard_write_text(&text);
                }
                PlatformEffect::RequestClipboardPaste => {
                    if let Some(text) = ctx.clipboard_read_text() {
                        let paste_output = self.gui.paste_focused_text(&text);
                        self.handle_framework_output(paste_output, ctx);
                    }
                }
            }
        }
    }

    fn handle_gui_action(&mut self, action: GuiAction) -> WorkspaceActionResult {
        tracing::info!("GuiAction: {:?}", action);
        self.workspace.handle_gui_action(action)
    }

    fn is_duplicate_action_event(&self, event: &GuiEvent, handled_node_adds: &[String]) -> bool {
        let GuiEvent::Widget(WidgetEvent::Click { id }) = event else {
            return false;
        };
        let Some(type_id) = node_library_add_type_id(id) else {
            return false;
        };
        handled_node_adds.iter().any(|handled| handled == type_id)
    }

    fn handle_gui_event(&mut self, event: GuiEvent) {
        tracing::info!("GuiEvent: {:?}", event);
        if let Some(message) = self.map_gui_event(event) {
            self.handle_message(message);
        }
    }

    fn map_gui_event(&self, event: GuiEvent) -> Option<AppMessage> {
        match event {
            GuiEvent::Widget(event) => self.map_widget_event(event),
            GuiEvent::Panel(_) => None,
            GuiEvent::Overlay(_) => None,
        }
    }

    fn map_widget_event(&self, event: WidgetEvent) -> Option<AppMessage> {
        Some(match event {
            WidgetEvent::Click { id } => AppMessage::WidgetClicked(id),
            WidgetEvent::DragStart { id, x, y } => AppMessage::WidgetDragStart { id, x, y },
            WidgetEvent::DragMove { id, x, y } => AppMessage::WidgetDragMove { id, x, y },
            WidgetEvent::DragEnd { id, x, y } => AppMessage::WidgetDragEnd { id, x, y },
            WidgetEvent::ResizeStart { id, edge, x, y } => {
                AppMessage::WidgetResizeStart { id, edge, x, y }
            }
            WidgetEvent::ResizeMove { id, edge, x, y } => {
                AppMessage::WidgetResizeMove { id, edge, x, y }
            }
            WidgetEvent::ResizeEnd { id, edge, x, y } => {
                AppMessage::WidgetResizeEnd { id, edge, x, y }
            }
            WidgetEvent::TextChanged { id, value } => AppMessage::WidgetTextChanged { id, value },
            WidgetEvent::LongPress { id } => AppMessage::LongPress(id),
            WidgetEvent::DoubleClick { .. }
            | WidgetEvent::NumberChanged { .. }
            | WidgetEvent::SelectionChanged { .. } => return None,
        })
    }

    fn handle_message(&mut self, message: AppMessage) {
        self.handle_user_message(message);
    }

    fn handle_user_message(&mut self, message: AppMessage) {
        match message {
            AppMessage::WidgetClicked(id) => {
                if self.workspace.toggle_canvas_port_group(&mut self.gui, &id) {
                    return;
                }
                if self.workspace.select_canvas_node(&mut self.gui, &id) {
                    return;
                }
                if let Some(type_id) = node_library_add_type_id(&id) {
                    let result = self.workspace.add_node_from_library(type_id);
                    if result.close_overlay {
                        self.scene_controller.close_overlay();
                        self.gui.close_overlay();
                    }
                    return;
                }
                let _ = self.workspace.handle_image_demo_button(&id);
            }
            AppMessage::WidgetDragStart { id, x, y } => {
                if self.workspace.begin_canvas_port_connection(
                    &mut self.gui,
                    &self.camera,
                    &id,
                    x,
                    y,
                ) {
                    return;
                }
                let _ =
                    self.workspace
                        .start_canvas_node_drag(&mut self.gui, &self.camera, &id, x, y);
            }
            AppMessage::WidgetDragMove { id, x, y } => {
                if self
                    .workspace
                    .update_canvas_port_connection(&mut self.gui, &self.camera, x, y)
                {
                    return;
                }
                let _ = self
                    .workspace
                    .drag_canvas_node(&mut self.gui, &self.camera, &id, x, y);
            }
            AppMessage::WidgetDragEnd { id, x, y } => {
                if self
                    .workspace
                    .end_canvas_port_connection(&mut self.gui, x, y)
                {
                    return;
                }
                let _ = self
                    .workspace
                    .end_canvas_node_drag(&mut self.gui, &self.camera, &id, x, y);
            }
            AppMessage::WidgetResizeStart { id, edge, x, y } => {
                let _ = self.workspace.start_canvas_node_resize(
                    &mut self.gui,
                    &self.camera,
                    &id,
                    edge,
                    x,
                    y,
                );
            }
            AppMessage::WidgetResizeMove { id, edge, x, y } => {
                let _ =
                    self.workspace
                        .resize_canvas_node(&mut self.gui, &self.camera, &id, edge, x, y);
            }
            AppMessage::WidgetResizeEnd { id, edge, x, y } => {
                let _ = self.workspace.end_canvas_node_resize(
                    &mut self.gui,
                    &self.camera,
                    &id,
                    edge,
                    x,
                    y,
                );
            }
            AppMessage::WidgetTextChanged { id, value } => {
                let _ = self.workspace.update_text_area_showcase_value(&id, value);
            }
            AppMessage::LongPress(id) => {
                tracing::info!("LongPress: {}", id);
            }
        }
    }

    fn update_hover_cursor(&self, x: f32, y: f32, ctx: &mut AppContext) {
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            x,
            y,
            panning = self.navigation.is_panning(),
            "update hover cursor"
        );
        if self.navigation.is_panning() {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                x,
                y,
                cursor = ?CursorStyle::Move,
                "hover cursor set while canvas is panning"
            );
            ctx.cursor.set(CursorStyle::Move);
            return;
        }

        if let Some((node_id, edge)) = self.gui.resize_hit_at_screen_point(x, y) {
            let cursor = cursor_for_resize_edge(edge);
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                x,
                y,
                node_id = ?self.gui.node_name(node_id),
                edge = ?edge,
                cursor = ?cursor,
                "hover cursor set from resize hit"
            );
            ctx.cursor.set(cursor);
            return;
        }

        let chain = self.gui.hit_test(x, y);
        if chain.is_empty() {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                x,
                y,
                "hover cursor no-op: empty hit chain and no resize hit"
            );
            return;
        }
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            x,
            y,
            chain_len = chain.len(),
            leaf = ?chain.leaf().and_then(|node_id| self.gui.node_name(node_id)),
            root = ?chain.root().and_then(|node_id| self.gui.node_name(node_id)),
            "hover cursor falling back to ordinary hit chain"
        );

        for node_id in chain.iter() {
            if self.gui.node_name(node_id).is_none() {
                continue;
            }

            if self.gui.node_is_text_input_field(node_id)
                || self.gui.node_is_text_area_field(node_id)
            {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    x,
                    y,
                    node_id = ?self.gui.node_name(node_id),
                    cursor = ?CursorStyle::Text,
                    "hover cursor set from text field hit"
                );
                ctx.cursor.set(CursorStyle::Text);
                return;
            }

            if self.gui.node_is_draggable(node_id) {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    x,
                    y,
                    node_id = ?self.gui.node_name(node_id),
                    cursor = ?CursorStyle::Move,
                    "hover cursor set from draggable hit"
                );
                ctx.cursor.set(CursorStyle::Move);
                return;
            }

            if self.gui.node_has_gesture(node_id, Gesture::Drag) {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    x,
                    y,
                    node_id = ?self.gui.node_name(node_id),
                    cursor = ?CursorStyle::Pointer,
                    "hover cursor set from drag gesture hit"
                );
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }

            if self.gui.node_has_gesture(node_id, Gesture::Tap)
                || self.gui.node_has_gesture(node_id, Gesture::DoubleTap)
            {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    x,
                    y,
                    node_id = ?self.gui.node_name(node_id),
                    cursor = ?CursorStyle::Pointer,
                    "hover cursor set from tap gesture hit"
                );
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }
        }
    }

    fn handle_canvas_release(&mut self, x: f32, y: f32) {
        if !self.is_blank_canvas_position(x, y) {
            self.last_canvas_click = None;
            return;
        }
        self.workspace.clear_canvas_selection(&mut self.gui);

        let now = Instant::now();
        let double_click = self.last_canvas_click.is_some_and(|last| {
            now.duration_since(last.at) <= CANVAS_DOUBLE_CLICK_TIMEOUT
                && distance_sq([x, y], [last.x, last.y]) <= CANVAS_DOUBLE_CLICK_DISTANCE_SQ
        });

        if double_click {
            self.open_node_library(x, y);
            self.last_canvas_click = None;
        } else {
            self.last_canvas_click = Some(CanvasClick { at: now, x, y });
        }
    }

    fn is_blank_canvas_position(&self, x: f32, y: f32) -> bool {
        let chain = self.gui.hit_test(x, y);
        if chain.is_empty() {
            return true;
        }

        let blank = chain.iter().all(|node_id| {
            self.gui
                .node_name(node_id)
                .map(|id| matches!(id, "root" | "canvas_root" | "canvas_grid" | "panel_root"))
                .unwrap_or(true)
        });
        blank
    }

    fn open_node_library(&mut self, x: f32, y: f32) {
        let state = self.node_palette_state();
        self.scene_controller.open_node_palette(x, y, state);
        self.workspace.note_node_library_opened();
    }

    fn node_palette_state(&self) -> crate::workspace::node_palette::NodePaletteState {
        self.workspace.node_palette_state()
    }
}

#[derive(Clone, Copy)]
struct CanvasClick {
    at: Instant,
    x: f32,
    y: f32,
}

fn distance_sq(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

fn is_developer_mode_shortcut(event: &AppEvent) -> bool {
    match event {
        AppEvent::KeyPress {
            key: Key::Function(12),
            ..
        } => true,
        AppEvent::KeyPress { key, modifiers } => {
            *key == Key::Char('D')
                && modifiers.shift
                && (modifiers.ctrl || modifiers.meta)
                && !modifiers.alt
        }
        _ => false,
    }
}

fn viewport_rect(ctx: &AppContext) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: ctx.size.width as f32 / ctx.scale_factor as f32,
        h: ctx.size.height as f32 / ctx.scale_factor as f32,
    }
}

fn cursor_for_resize_edge(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top => CursorStyle::ResizeN,
        ResizeEdge::Bottom => CursorStyle::ResizeS,
        ResizeEdge::Left => CursorStyle::ResizeW,
        ResizeEdge::Right => CursorStyle::ResizeE,
        ResizeEdge::TopLeft => CursorStyle::ResizeNW,
        ResizeEdge::TopRight => CursorStyle::ResizeNE,
        ResizeEdge::BottomLeft => CursorStyle::ResizeSW,
        ResizeEdge::BottomRight => CursorStyle::ResizeSE,
    }
}

#[cfg(test)]
mod tests {
    use super::{initial_app_mode, is_developer_mode_shortcut, toggle_app_mode, AppMode};
    use gui::shell::{AppEvent, Key, Modifiers};

    #[test]
    fn initial_app_mode_respects_developer_mode_gate() {
        assert_eq!(initial_app_mode(true), AppMode::Developer);
        assert_eq!(initial_app_mode(false), AppMode::User);
    }

    #[test]
    fn app_mode_toggle_respects_developer_mode_gate() {
        assert_eq!(toggle_app_mode(AppMode::User, true), AppMode::Developer);
        assert_eq!(toggle_app_mode(AppMode::Developer, true), AppMode::User);
        assert_eq!(toggle_app_mode(AppMode::User, false), AppMode::User);
        assert_eq!(toggle_app_mode(AppMode::Developer, false), AppMode::User);
    }

    #[test]
    fn developer_mode_shortcut_is_ctrl_shift_d() {
        assert!(is_developer_mode_shortcut(&AppEvent::KeyPress {
            key: Key::Function(12),
            modifiers: Modifiers::default(),
        }));

        assert!(is_developer_mode_shortcut(&AppEvent::KeyPress {
            key: Key::Char('D'),
            modifiers: Modifiers {
                ctrl: true,
                shift: true,
                ..Modifiers::default()
            },
        }));

        assert!(!is_developer_mode_shortcut(&AppEvent::KeyPress {
            key: Key::Char('D'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        }));
    }
}
