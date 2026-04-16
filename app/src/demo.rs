use crate::demo_gallery::{
    build_demo_popup, build_gallery_sections, GallerySection, GalleryState, POPUP_CLOSE_ID,
    POPUP_TRIGGER_ID, SLIDER_RADIUS_ID,
};
use gui::canvas::camera::Camera;
use gui::canvas::navigation::CanvasNavigationController;
use gui::context::{Context, FrameworkOutput, OverlayPlacement, OverlayRequest, PlatformEffect};
use gui::gesture::Gesture;
use gui::renderer::{Rect, Renderer};
use gui::shell::{App, AppContext, AppEvent, CursorStyle, MouseButton};
use gui::theme::{light_theme, Theme};
use gui::tree::layout::{BoxStyle, Decoration, LeafKind, Position, Size, TextureHandle, Transform};
use gui::tree::Desc;
use gui::widget::action::Action;
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

        if self.navigation.handle_event(&event, &mut self.camera) {
            if self.navigation.is_panning() {
                ctx.cursor.set(CursorStyle::Move);
            }
            return;
        }

        let output = self.gui.handle_event(&event);
        self.handle_framework_output(output, ctx);

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
        for action in output.actions {
            self.handle_action(action);
        }

        for effect in output.effects {
            match effect {
                PlatformEffect::WriteClipboard(text) => {
                    let _ = ctx.clipboard_write_text(&text);
                }
                PlatformEffect::RequestClipboardPaste => {
                    if let Some(text) = ctx.clipboard_read_text() {
                        for action in self.gui.paste_focused_text(&text) {
                            self.handle_action(action);
                        }
                    }
                }
            }
        }
    }

    fn handle_action(&mut self, action: Action) {
        tracing::info!("Action: {:?}", action);
        match action {
            Action::Click(id) => {
                tracing::info!("handle click: {}", id);
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
            Action::DoubleClick(id) => {
                if id.contains("slider") {
                    self.gallery.slider_value = 5.0;
                }
            }
            Action::TextChange { id, value } => {
                let _ = self.gallery.apply_text_change(&id, value);
            }
            Action::NumberChange { id, value } => {
                let _ = self.gallery.apply_number_change(&id, value);
            }
            Action::SelectChange { id, selected } => {
                let _ = self.gallery.apply_select_change(&id, selected);
            }
            Action::DragMove { id, x, y } => {
                let mut should_update_slider = false;
                if let Some(session) = &mut self.pointer_session {
                    if is_slider_target(&id) {
                        should_update_slider = true;
                    } else if is_gallery_panel_node(&id) {
                        let entry = self.panel_positions.entry(id.clone()).or_insert_with(|| {
                            self.gui
                                .tree()
                                .iter()
                                .find_map(|(_, node)| {
                                    (node.id.as_ref() == id).then_some((node.rect.x, node.rect.y))
                                })
                                .unwrap_or((0.0, 0.0))
                        });
                        entry.0 += x - session.last_x;
                        entry.1 += y - session.last_y;
                    } else {
                        tracing::info!("Unhandled drag target: {} at ({}, {})", id, x, y);
                    }
                    session.last_x = x;
                    session.last_y = y;
                }
                if should_update_slider {
                    self.update_slider_from_pointer(x);
                    tracing::info!("slider_value(drag) -> {}", self.gallery.slider_value);
                }
            }
            Action::ResizeMove { id, edge, x, y } => {
                tracing::info!(
                    "Ignoring gallery panel resize: {} {:?} {} {}",
                    id,
                    edge,
                    x,
                    y
                );
            }
            Action::DragStart { id, x, y } => {
                if is_slider_target(&id) || is_gallery_panel_node(&id) {
                    self.pointer_session = Some(PointerSession::new(x, y));
                }
            }
            Action::ResizeStart { .. } => {}
            Action::DragEnd { .. } | Action::ResizeEnd { .. } => {
                self.pointer_session = None;
            }
            Action::LongPress(id) => {
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
            let Some(node) = self.gui.tree().get(node_id) else {
                continue;
            };

            if node.style.gestures.contains(&Gesture::Resize)
                && !is_gallery_panel_node(node.id.as_ref())
            {
                if let Some(edge) = detect_resize_edge(node.rect, x, y) {
                    ctx.cursor.set(cursor_for_resize_edge(edge));
                    return;
                }
            }

            let id = node.id.as_ref();
            if is_slider_target(id) && node.style.gestures.contains(&Gesture::Drag) {
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }

            if is_text_input_field(self.gui.tree(), node_id) {
                ctx.cursor.set(CursorStyle::Text);
                return;
            }

            if is_toggle_target(id)
                && (node.style.gestures.contains(&Gesture::Tap)
                    || node.style.gestures.contains(&Gesture::DoubleTap))
            {
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }

            if id.ends_with("::titlebar") && node.style.gestures.contains(&Gesture::Drag) {
                ctx.cursor.set(CursorStyle::Move);
                return;
            }

            if is_gallery_panel_node(node.id.as_ref()) {
                continue;
            }

            if node.style.gestures.contains(&Gesture::Tap)
                || node.style.gestures.contains(&Gesture::DoubleTap)
            {
                ctx.cursor.set(CursorStyle::Pointer);
                return;
            }
        }
    }

    fn update_slider_from_pointer(&mut self, x: f32) {
        if let Some(value) =
            slider_value_from_x(self.gui.tree(), "slider_radius", x, 0.0, 10.0, 0.1)
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

fn is_text_input_field(tree: &gui::tree::Tree, node_id: usize) -> bool {
    let Some(node) = tree.get(node_id) else {
        return false;
    };
    let node_id_str = node.id.as_ref();
    if !node_id_str.ends_with("::field") {
        return false;
    }

    let Some(root_id) = node_id_str.split("::").next() else {
        return false;
    };
    let Some((_, root_node)) = tree
        .iter()
        .find(|(_, candidate)| candidate.id.as_ref() == root_id)
    else {
        return false;
    };
    let gui::tree::NodeKind::Widget(props) = &root_node.kind else {
        return false;
    };
    props.widget_type() == "TextInput"
}

fn slider_value_from_x(
    tree: &gui::tree::Tree,
    root_id: &str,
    x: f32,
    min: f32,
    max: f32,
    step: f32,
) -> Option<f32> {
    let track_id = format!("{root_id}::track");
    let (_, track) = tree.iter().find(|(_, node)| node.id.as_ref() == track_id)?;
    let width = track.rect.w.max(1.0);
    let ratio = ((x - track.rect.x) / width).clamp(0.0, 1.0);
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
