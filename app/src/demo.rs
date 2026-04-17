use crate::demo_gallery::{
    build_demo_popup, build_gallery_sections, GallerySection, GalleryState, POPUP_CLOSE_ID,
    POPUP_TRIGGER_ID, SLIDER_RADIUS_ID,
};
use gui::canvas::camera::Camera;
use gui::canvas::navigation::CanvasNavigationController;
use gui::context::{
    Context, FrameworkOutput, GuiEvent, OverlayPlacement, OverlayRequest, PanelEvent,
    PlatformEffect, WidgetEvent,
};
use gui::gesture::Gesture;
use gui::renderer::{Rect, Renderer};
use gui::shell::{App, AppContext, AppEvent, CursorStyle, MouseButton};
use gui::theme::{light_theme, Theme};
use gui::tree::layout::{BoxStyle, Decoration, LeafKind, Position, Size, TextureHandle, Transform};
use gui::tree::Desc;
use gui::widget::frameworks::panel::PanelProps;
use gui::widget::resize_edge::ResizeEdge;
use std::borrow::Cow;
use std::collections::HashMap;

const GALLERY_SCALE: f32 = 1.2;
const GRID_SPACING: f32 = 20.0;
const GRID_DOT_SIZE: f32 = 1.5;
const DEMO_IMAGE_HANDLE: TextureHandle = TextureHandle(1);
const GALLERY_PANEL_PREFIX: &str = "gallery_panel::";

fn align_grid_start(min_canvas: f32, spacing: f32) -> f32 {
    (min_canvas / spacing).floor() * spacing - spacing * 2.0
}

#[derive(Debug, Clone, Copy)]
struct PointerSession {
    last_x: f32,
    last_y: f32,
}

impl PointerSession {
    fn new(x: f32, y: f32) -> Self {
        Self {
            last_x: x,
            last_y: y,
        }
    }
}

#[derive(Debug)]
enum DemoMessage {
    WidgetClicked(String),
    WidgetDoubleClicked(String),
    TextChanged {
        id: String,
        value: String,
    },
    NumberChanged {
        id: String,
        value: f32,
    },
    SelectionChanged {
        id: String,
        selected: usize,
    },
    WidgetDragStart {
        id: String,
        x: f32,
        y: f32,
    },
    WidgetDragMove {
        id: String,
        x: f32,
    },
    WidgetDragEnd,
    LongPress(String),
    PanelDragStart {
        id: String,
        x: f32,
        y: f32,
    },
    PanelDragMove {
        id: String,
        x: f32,
        y: f32,
    },
    PanelDragEnd,
    PanelResizeMove {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
}

fn build_demo_tree(
    viewport: Rect,
    camera: &Camera,
    theme: &Theme,
    gallery: &GalleryState,
    panel_positions: &HashMap<String, (f32, f32)>,
) -> Desc {
    let (canvas_min_x, canvas_min_y) = camera.screen_to_canvas(0.0, 0.0);
    let (canvas_max_x, canvas_max_y) = camera.screen_to_canvas(viewport.w, viewport.h);
    let grid_x = align_grid_start(canvas_min_x, GRID_SPACING);
    let grid_y = align_grid_start(canvas_min_y, GRID_SPACING);
    let grid_w = (canvas_max_x - canvas_min_x).abs() + GRID_SPACING * 4.0;
    let grid_h = (canvas_max_y - canvas_min_y).abs() + GRID_SPACING * 4.0;
    let sections = build_gallery_sections(theme, gallery, DEMO_IMAGE_HANDLE);

    Desc::Container {
        id: Cow::Borrowed("root"),
        style: BoxStyle {
            width: Size::Fixed(viewport.w),
            height: Size::Fixed(viewport.h),
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(theme.colors.canvas_bg),
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }),
        children: vec![
            Desc::Container {
                id: Cow::Borrowed("canvas_root"),
                style: BoxStyle {
                    position: Position::Absolute { x: 0.0, y: 0.0 },
                    width: Size::Fixed(viewport.w),
                    height: Size::Fixed(viewport.h),
                    transform: Some(Transform {
                        translate: [camera.x, camera.y],
                        scale: camera.zoom,
                        rotate: 0.0,
                    }),
                    ..BoxStyle::default()
                },
                decoration: None,
                children: vec![Desc::Leaf {
                    id: Cow::Borrowed("canvas_grid"),
                    style: BoxStyle {
                        position: Position::Absolute {
                            x: grid_x,
                            y: grid_y,
                        },
                        width: Size::Fixed(grid_w.max(GRID_SPACING)),
                        height: Size::Fixed(grid_h.max(GRID_SPACING)),
                        ..BoxStyle::default()
                    },
                    kind: LeafKind::Grid {
                        spacing: GRID_SPACING,
                        dot_color: theme.colors.canvas_grid,
                        dot_size: GRID_DOT_SIZE,
                    },
                }],
            },
            Desc::Container {
                id: Cow::Borrowed("panel_root"),
                style: BoxStyle {
                    position: Position::Absolute { x: 0.0, y: 0.0 },
                    width: Size::Fixed(viewport.w),
                    height: Size::Fixed(viewport.h),
                    ..BoxStyle::default()
                },
                decoration: None,
                children: build_gallery_panels(viewport, &sections, panel_positions),
            },
        ],
    }
}

fn build_gallery_panels(
    viewport: Rect,
    sections: &[GallerySection],
    panel_positions: &HashMap<String, (f32, f32)>,
) -> Vec<Desc> {
    let columns = if viewport.w >= 1200.0 { 4 } else { 3 };
    let margin = 28.0;
    let gap = 22.0;
    let panel_w = ((viewport.w - margin * 2.0) - gap * (columns as f32 - 1.0)) / columns as f32;
    let mut column_heights = vec![margin; columns];

    sections
        .iter()
        .map(|section| {
            let column = section.column.min(columns.saturating_sub(1));
            let panel_id = format!("{GALLERY_PANEL_PREFIX}{}", section.id);
            let default_x = margin + column as f32 * (panel_w + gap);
            let default_y = column_heights[column];
            let (x, y) = panel_positions
                .get(&panel_id)
                .copied()
                .unwrap_or((default_x, default_y));
            column_heights[column] += section.estimated_height + gap;

            Desc::Widget {
                id: Cow::Owned(panel_id.clone()),
                props: Box::new(PanelProps {
                    id: Cow::Owned(panel_id),
                    title: Cow::Borrowed(section.title),
                    x,
                    y,
                    w: panel_w,
                    h: 0.0,
                    content: section.clone().content,
                }),
            }
        })
        .collect()
}

pub struct DemoApp {
    gui: Context,
    pointer_session: Option<PointerSession>,
    camera: Camera,
    navigation: CanvasNavigationController,
    active_button: Option<String>,
    gallery: GalleryState,
    panel_positions: HashMap<String, (f32, f32)>,
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
            pointer_session: None,
            camera: Camera::new(),
            navigation: CanvasNavigationController::new(),
            active_button: None,
            gallery: GalleryState::default(),
            panel_positions: HashMap::new(),
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
            AppEvent::MouseRelease { button, .. } if button == MouseButton::Left => {}
            _ => {}
        }
    }

    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext) {
        let viewport = viewport_rect(ctx);
        let desc = build_demo_tree(
            viewport,
            &self.camera,
            &self.theme,
            &self.gallery,
            &self.panel_positions,
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
        for event in output.events {
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

    fn handle_gui_event(&mut self, event: GuiEvent) {
        tracing::info!("GuiEvent: {:?}", event);
        if let Some(message) = self.map_gui_event(event) {
            self.handle_message(message);
        }
    }

    fn map_gui_event(&self, event: GuiEvent) -> Option<DemoMessage> {
        match event {
            GuiEvent::Widget(event) => self.map_widget_event(event),
            GuiEvent::Panel(event) => self.map_panel_event(event),
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
            WidgetEvent::DragMove { id, x, .. } => DemoMessage::WidgetDragMove { id, x },
            WidgetEvent::DragEnd { .. } => DemoMessage::WidgetDragEnd,
            WidgetEvent::LongPress { id } => DemoMessage::LongPress(id),
        })
    }

    fn map_panel_event(&self, event: PanelEvent) -> Option<DemoMessage> {
        Some(match event {
            PanelEvent::DragStart { id, x, y } => DemoMessage::PanelDragStart { id, x, y },
            PanelEvent::DragMove { id, x, y } => DemoMessage::PanelDragMove { id, x, y },
            PanelEvent::DragEnd { .. } => DemoMessage::PanelDragEnd,
            PanelEvent::ResizeStart { .. } => return None,
            PanelEvent::ResizeMove { id, edge, x, y } => {
                DemoMessage::PanelResizeMove { id, edge, x, y }
            }
            PanelEvent::ResizeEnd { .. } => return None,
        })
    }

    fn handle_message(&mut self, message: DemoMessage) {
        match message {
            DemoMessage::WidgetClicked(id) => {
                let _ = self.gallery.apply_click(&id);
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
                if is_slider_target(&id) {
                    self.pointer_session = Some(PointerSession::new(x, y));
                }
            }
            DemoMessage::WidgetDragMove { id, x } => {
                if is_slider_target(&id) {
                    self.update_slider_from_pointer(x);
                    tracing::info!("slider_value(drag) -> {}", self.gallery.slider_value);
                }
            }
            DemoMessage::WidgetDragEnd => {
                self.pointer_session = None;
            }
            DemoMessage::LongPress(id) => {
                tracing::info!("LongPress: {}", id);
            }
            DemoMessage::PanelDragStart { id, x, y } => {
                if is_gallery_panel_node(&id) {
                    self.pointer_session = Some(PointerSession::new(x, y));
                }
            }
            DemoMessage::PanelDragMove { id, x, y } => {
                if !is_gallery_panel_node(&id) {
                    return;
                }
                if let Some(session) = &mut self.pointer_session {
                    let entry = self.panel_positions.entry(id.clone()).or_insert_with(|| {
                        self.gui
                            .node_rect(&id)
                            .map(|rect| (rect.x, rect.y))
                            .unwrap_or((0.0, 0.0))
                    });
                    entry.0 += x - session.last_x;
                    entry.1 += y - session.last_y;
                    session.last_x = x;
                    session.last_y = y;
                }
            }
            DemoMessage::PanelDragEnd => {
                self.pointer_session = None;
            }
            DemoMessage::PanelResizeMove { id, edge, x, y } => {
                tracing::info!(
                    "Ignoring gallery panel resize: {} {:?} {} {}",
                    id,
                    edge,
                    x,
                    y
                );
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

            if self.gui.node_has_gesture(node_id, Gesture::Resize) && !is_gallery_panel_node(id) {
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

            if is_gallery_panel_node(id) {
                continue;
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
}

fn is_toggle_target(id: &str) -> bool {
    id == "toggle_grid" || id.starts_with("toggle_grid::")
}

fn is_slider_target(id: &str) -> bool {
    id == SLIDER_RADIUS_ID || id.starts_with(&format!("{SLIDER_RADIUS_ID}::"))
}

fn is_gallery_panel_node(id: &str) -> bool {
    id.starts_with(GALLERY_PANEL_PREFIX)
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

#[cfg(test)]
mod tests {
    use super::align_grid_start;

    #[test]
    fn grid_alignment_snaps_to_spacing_not_single_units() {
        assert_eq!(align_grid_start(3.0, 20.0), -40.0);
        assert_eq!(align_grid_start(21.0, 20.0), -20.0);
        assert_eq!(align_grid_start(-1.0, 20.0), -60.0);
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
