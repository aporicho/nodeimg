use crate::demo_gallery::{
    build_demo_popup, GalleryState, POPUP_CLOSE_ID, POPUP_TRIGGER_ID, SLIDER_RADIUS_ID,
};
use crate::workspace::controller::{WorkspaceActionResult, WorkspaceController};
use crate::workspace::node_palette;
use crate::workspace::view::build_workspace_tree;
use gui::action::{node_library_add_type_id, GuiAction};
use gui::canvas::camera::Camera;
use gui::canvas::navigation::CanvasNavigationController;
use gui::context::{
    Context, FrameworkOutput, GuiEvent, OverlayPlacement, OverlayRequest, PlatformEffect,
    WidgetEvent,
};
use gui::gesture::Gesture;
use gui::renderer::{Rect, Renderer};
use gui::shell::{App, AppContext, AppEvent, CursorStyle, MouseButton};
use gui::theme::{light_theme, Theme};
use gui::tree::layout::TextureHandle;
use gui::widget::resize_edge::ResizeEdge;
use std::time::{Duration, Instant};

const GALLERY_SCALE: f32 = 1.2;
const DEMO_IMAGE_HANDLE: TextureHandle = TextureHandle(1);
const CANVAS_DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_millis(300);
const CANVAS_DOUBLE_CLICK_DISTANCE_SQ: f32 = 36.0;

#[derive(Debug)]
enum DemoMessage {
    WidgetClicked(String),
    WidgetDoubleClicked(String),
    TextChanged { id: String, value: String },
    NumberChanged { id: String, value: f32 },
    SelectionChanged { id: String, selected: usize },
    WidgetDragStart { id: String, x: f32, y: f32 },
    WidgetDragMove { id: String, x: f32, y: f32 },
    WidgetDragEnd { id: String, x: f32, y: f32 },
    LongPress(String),
}

pub struct DemoApp {
    gui: Context,
    camera: Camera,
    navigation: CanvasNavigationController,
    active_button: Option<String>,
    gallery: GalleryState,
    workspace: WorkspaceController,
    last_canvas_click: Option<CanvasClick>,
    mouse_x: f32,
    mouse_y: f32,
    theme: Theme,
}

impl App for DemoApp {
    fn init(ctx: &mut AppContext) -> Self {
        let mut gui = Context::new();
        gui.register_texture(
            DEMO_IMAGE_HANDLE,
            create_demo_texture(&ctx.device, &ctx.queue),
        );

        Self {
            gui,
            camera: Camera::new(),
            navigation: CanvasNavigationController::new(),
            active_button: None,
            gallery: GalleryState::default(),
            workspace: WorkspaceController::new(),
            last_canvas_click: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
            theme: light_theme().scaled(GALLERY_SCALE),
        }
    }

    fn event(&mut self, event: AppEvent, ctx: &mut AppContext) {
        if !matches!(event, AppEvent::MouseMove { .. }) {
            tracing::info!("event: {:?}", event);
        }

        self.update_mouse_position_from_event(&event);

        let output = self.gui.handle_event(&event);
        let consumed = output.consumed;
        self.handle_framework_output(output, ctx);

        if !consumed && self.navigation.handle_event(&event, &mut self.camera) {
            if self.navigation.is_panning() {
                ctx.cursor.set(CursorStyle::Move);
            }
            return;
        }

        match event {
            AppEvent::MouseMove { x, y } => {
                self.update_hover_cursor(x, y, ctx);
            }
            AppEvent::MouseRelease { x, y, button } if button == MouseButton::Left => {
                if !consumed {
                    self.handle_canvas_release(x, y);
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext) {
        let viewport = viewport_rect(ctx);
        let panels = crate::panels::collect_panels(&crate::panels::PanelBuildContext {
            theme: &self.theme,
            gallery: &self.gallery,
            image: DEMO_IMAGE_HANDLE,
            engine: &self.workspace.engine_panel_state(),
        });
        let panel_root = self.gui.panel_root(viewport, panels);
        let canvas_nodes = self.workspace.canvas_node_views(&mut self.gui);
        let desc = build_workspace_tree(
            viewport,
            &self.camera,
            &self.theme,
            &canvas_nodes,
            panel_root,
        );
        self.gui
            .update(desc, viewport, renderer.text_measurer(), &self.theme);
        ctx.apply_ime_request(self.gui.ime_request());
        self.update_hover_cursor(self.mouse_x, self.mouse_y, ctx);
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        let viewport = viewport_rect(ctx);
        self.gui
            .render(renderer, viewport.w, viewport.h, &self.theme);
    }
}

fn create_demo_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> std::sync::Arc<wgpu::TextureView> {
    const SIZE: u32 = 96;
    let mut data = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let idx = ((y * SIZE + x) * 4) as usize;
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
        label: Some("demo_image_texture"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
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
            bytes_per_row: Some(4 * SIZE),
            rows_per_image: Some(SIZE),
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );

    std::sync::Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

impl DemoApp {
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

    fn map_gui_event(&self, event: GuiEvent) -> Option<DemoMessage> {
        match event {
            GuiEvent::Widget(event) => self.map_widget_event(event),
            GuiEvent::Panel(_) => None,
            GuiEvent::Overlay(_) => None,
        }
    }

    fn map_widget_event(&self, event: WidgetEvent) -> Option<DemoMessage> {
        Some(match event {
            WidgetEvent::Click { id } => DemoMessage::WidgetClicked(id),
            WidgetEvent::DoubleClick { id } => DemoMessage::WidgetDoubleClicked(id),
            WidgetEvent::TextChanged { id, value } => DemoMessage::TextChanged { id, value },
            WidgetEvent::NumberChanged { id, value } => DemoMessage::NumberChanged { id, value },
            WidgetEvent::SelectionChanged { id, selected } => {
                DemoMessage::SelectionChanged { id, selected }
            }
            WidgetEvent::DragStart { id, x, y } => DemoMessage::WidgetDragStart { id, x, y },
            WidgetEvent::DragMove { id, x, y } => DemoMessage::WidgetDragMove { id, x, y },
            WidgetEvent::DragEnd { id, x, y } => DemoMessage::WidgetDragEnd { id, x, y },
            WidgetEvent::LongPress { id } => DemoMessage::LongPress(id),
        })
    }

    fn handle_message(&mut self, message: DemoMessage) {
        match message {
            DemoMessage::WidgetClicked(id) => {
                let _ = self.gallery.apply_click(&id);
                if let Some(type_id) = node_library_add_type_id(&id) {
                    let result = self.workspace.add_node_from_library(type_id);
                    if result.close_overlay {
                        self.gui.close_overlay();
                    }
                    self.active_button = Some(id);
                    return;
                }
                let _ = self.workspace.handle_image_demo_button(&id);
                if id == POPUP_TRIGGER_ID {
                    if self.gui.overlay_open() {
                        self.gui.close_overlay();
                    } else {
                        self.gui.open_overlay(OverlayRequest {
                            id: "demo_popup".to_string(),
                            anchor_id: POPUP_TRIGGER_ID.to_string(),
                            restore_focus_id: Some(POPUP_TRIGGER_ID.to_string()),
                            placement: OverlayPlacement::BelowStart,
                            content: build_demo_popup(),
                            offset_x: 0.0,
                            offset_y: 8.0,
                            match_anchor_width: false,
                            dismiss_on_escape: true,
                            dismiss_on_outside_click: true,
                            restore_focus_to_anchor: true,
                        });
                    }
                }
                if id == POPUP_CLOSE_ID {
                    self.gui.close_overlay();
                }
                if is_slider_target(&id) {
                    self.update_slider_from_pointer(self.mouse_x);
                    tracing::info!("slider_value(click) -> {}", self.gallery.slider_value);
                }
                self.active_button = Some(id);
            }
            DemoMessage::WidgetDoubleClicked(id) => {
                if id == SLIDER_RADIUS_ID {
                    self.gallery.slider_value = 5.0;
                }
            }
            DemoMessage::TextChanged { id, value } => {
                let _ = self.gallery.apply_text_change(&id, value);
            }
            DemoMessage::NumberChanged { id, value } => {
                let _ = self.gallery.apply_number_change(&id, value);
            }
            DemoMessage::SelectionChanged { id, selected } => {
                let _ = self.gallery.apply_select_change(&id, selected);
            }
            DemoMessage::WidgetDragStart { id, x, y } => {
                if self
                    .workspace
                    .start_canvas_node_drag(&mut self.gui, &self.camera, &id, x, y)
                {
                    return;
                }
                if is_slider_target(&id) {
                    self.update_slider_from_pointer(x);
                    tracing::info!("slider_value(drag_start) -> {}", self.gallery.slider_value);
                }
                let _ = y;
            }
            DemoMessage::WidgetDragMove { id, x, y } => {
                if self
                    .workspace
                    .drag_canvas_node(&mut self.gui, &self.camera, &id, x, y)
                {
                    return;
                }
                if is_slider_target(&id) {
                    self.update_slider_from_pointer(x);
                    tracing::info!("slider_value(drag) -> {}", self.gallery.slider_value);
                }
            }
            DemoMessage::WidgetDragEnd { id, x, y } => {
                let _ = self
                    .workspace
                    .end_canvas_node_drag(&mut self.gui, &self.camera, &id, x, y);
            }
            DemoMessage::LongPress(id) => {
                tracing::info!("LongPress: {}", id);
            }
        }
    }

    fn update_hover_cursor(&self, x: f32, y: f32, ctx: &mut AppContext) {
        if self.navigation.is_panning() {
            ctx.cursor.set(CursorStyle::Move);
            return;
        }

        let chain = self.gui.hit_test(x, y);
        if chain.is_empty() {
            return;
        }

        for node_id in chain.iter() {
            let Some(id) = self.gui.node_name(node_id) else {
                continue;
            };

            if self.gui.node_has_gesture(node_id, Gesture::Resize) {
                if let Some(edge) = self
                    .gui
                    .node_rect_by_node(node_id)
                    .and_then(|rect| detect_resize_edge(rect, x, y))
                {
                    ctx.cursor.set(cursor_for_resize_edge(edge));
                    return;
                }
            }

            if is_slider_target(id) && self.gui.node_has_gesture(node_id, Gesture::Drag) {
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }

            if self.gui.node_is_text_input_field(node_id) {
                ctx.cursor.set(CursorStyle::Text);
                return;
            }

            if is_toggle_target(id)
                && (self.gui.node_has_gesture(node_id, Gesture::Tap)
                    || self.gui.node_has_gesture(node_id, Gesture::DoubleTap))
            {
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }

            if id.ends_with("::titlebar") && self.gui.node_has_gesture(node_id, Gesture::Drag) {
                ctx.cursor.set(CursorStyle::Move);
                return;
            }

            if id.starts_with("canvas_node::") && self.gui.node_has_gesture(node_id, Gesture::Drag)
            {
                ctx.cursor.set(CursorStyle::Move);
                return;
            }

            if self.gui.node_has_gesture(node_id, Gesture::Tap)
                || self.gui.node_has_gesture(node_id, Gesture::DoubleTap)
            {
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }
        }
    }

    fn update_slider_from_pointer(&mut self, x: f32) {
        if let Some(value) = self
            .gui
            .node_rect(&format!("{SLIDER_RADIUS_ID}::track"))
            .and_then(|track_rect| slider_value_from_x(track_rect, x, 0.0, 10.0, 0.1))
        {
            self.gallery.slider_value = value;
        }
    }

    fn handle_canvas_release(&mut self, x: f32, y: f32) {
        if !self.is_blank_canvas_position(x, y) {
            self.last_canvas_click = None;
            return;
        }

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
        self.gui.open_overlay(OverlayRequest {
            id: "node_palette".to_string(),
            anchor_id: "canvas_root".to_string(),
            restore_focus_id: None,
            placement: OverlayPlacement::AtPoint { x, y },
            content: node_palette::overlay_content(&state, &self.theme),
            offset_x: 0.0,
            offset_y: 0.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: false,
        });
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

fn is_toggle_target(id: &str) -> bool {
    id == "toggle_grid" || id.starts_with("toggle_grid::")
}

fn is_slider_target(id: &str) -> bool {
    id == SLIDER_RADIUS_ID || id.starts_with(&format!("{SLIDER_RADIUS_ID}::"))
}

fn slider_value_from_x(track_rect: Rect, x: f32, min: f32, max: f32, step: f32) -> Option<f32> {
    let width = track_rect.w.max(1.0);
    let ratio = ((x - track_rect.x) / width).clamp(0.0, 1.0);
    let raw = min + (max - min) * ratio;
    let stepped = if step > 0.0 {
        ((raw - min) / step).round() * step + min
    } else {
        raw
    };
    Some(stepped.clamp(min, max))
}

fn viewport_rect(ctx: &AppContext) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: ctx.size.width as f32 / ctx.scale_factor as f32,
        h: ctx.size.height as f32 / ctx.scale_factor as f32,
    }
}

fn detect_resize_edge(rect: Rect, x: f32, y: f32) -> Option<ResizeEdge> {
    const EDGE_THRESHOLD: f32 = 6.0;
    let near_left = (x - rect.x).abs() < EDGE_THRESHOLD;
    let near_right = (x - (rect.x + rect.w)).abs() < EDGE_THRESHOLD;
    let near_top = (y - rect.y).abs() < EDGE_THRESHOLD;
    let near_bottom = (y - (rect.y + rect.h)).abs() < EDGE_THRESHOLD;

    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (true, _, _, true) => Some(ResizeEdge::BottomLeft),
        (_, true, true, _) => Some(ResizeEdge::TopRight),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, _, _, _) => Some(ResizeEdge::Left),
        (_, true, _, _) => Some(ResizeEdge::Right),
        (_, _, true, _) => Some(ResizeEdge::Top),
        (_, _, _, true) => Some(ResizeEdge::Bottom),
        _ => None,
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
