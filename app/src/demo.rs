use std::borrow::Cow;
use std::time::Instant;

use gui::canvas::camera::Camera;
use gui::canvas::navigation::CanvasNavigationController;
use gui::context::{ClipboardRequest, Context};
use gui::gesture::{arena_from_hit_chain, Gesture, GestureArena};
use gui::renderer::{Color, Rect, Renderer};
use gui::shell::{App, AppContext, AppEvent, CursorStyle, MouseButton};
use gui::tree::layout::{BoxStyle, Decoration, LeafKind, Position, Size, Transform};
use gui::tree::Desc;
use gui::widget::action::Action;
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::slider::SliderProps;
use gui::widget::atoms::text_input::TextInputProps;
use gui::widget::atoms::toggle::ToggleProps;
use gui::widget::frameworks::panel::PanelProps;
use gui::widget::resize_edge::ResizeEdge;

const GRID_SPACING: f32 = 20.0;
const GRID_DOT_SIZE: f32 = 1.5;
const PANEL_ID: &str = "demo_panel";

#[derive(Debug, Clone, Copy)]
struct PanelState {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    min_w: f32,
    min_h: f32,
}

impl PanelState {
    fn new() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            w: 300.0,
            h: 200.0,
            min_w: 120.0,
            min_h: 80.0,
        }
    }

    fn apply_drag(&mut self, x: f32, y: f32, last_x: f32, last_y: f32) {
        self.x += x - last_x;
        self.y += y - last_y;
    }

    fn apply_resize(&mut self, edge: ResizeEdge, x: f32, y: f32, last_x: f32, last_y: f32) {
        let dx = x - last_x;
        let dy = y - last_y;

        match edge {
            ResizeEdge::Right => {
                self.w = (self.w + dx).max(self.min_w);
            }
            ResizeEdge::Bottom => {
                self.h = (self.h + dy).max(self.min_h);
            }
            ResizeEdge::Left => {
                let new_w = (self.w - dx).max(self.min_w);
                self.x += self.w - new_w;
                self.w = new_w;
            }
            ResizeEdge::Top => {
                let new_h = (self.h - dy).max(self.min_h);
                self.y += self.h - new_h;
                self.h = new_h;
            }
            ResizeEdge::TopLeft => {
                let new_w = (self.w - dx).max(self.min_w);
                let new_h = (self.h - dy).max(self.min_h);
                self.x += self.w - new_w;
                self.y += self.h - new_h;
                self.w = new_w;
                self.h = new_h;
            }
            ResizeEdge::TopRight => {
                self.w = (self.w + dx).max(self.min_w);
                let new_h = (self.h - dy).max(self.min_h);
                self.y += self.h - new_h;
                self.h = new_h;
            }
            ResizeEdge::BottomLeft => {
                let new_w = (self.w - dx).max(self.min_w);
                self.x += self.w - new_w;
                self.w = new_w;
                self.h = (self.h + dy).max(self.min_h);
            }
            ResizeEdge::BottomRight => {
                self.w = (self.w + dx).max(self.min_w);
                self.h = (self.h + dy).max(self.min_h);
            }
        }
    }
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

fn build_panel_content(text_value: &str, slider_value: f32, toggle_value: bool) -> Vec<Desc> {
    vec![
        Desc::Widget {
            id: Cow::Borrowed("btn_a"),
            props: Box::new(ButtonProps {
                label: "Button A".into(),
                icon: None,
                disabled: false,
            }),
        },
        Desc::Widget {
            id: Cow::Borrowed("btn_b"),
            props: Box::new(ButtonProps {
                label: "Button B".into(),
                icon: None,
                disabled: false,
            }),
        },
        Desc::Widget {
            id: Cow::Borrowed("text_prompt"),
            props: Box::new(TextInputProps {
                label: "Prompt".into(),
                value: Cow::Owned(text_value.to_string()),
                disabled: false,
            }),
        },
        Desc::Widget {
            id: Cow::Borrowed("slider_radius"),
            props: Box::new(SliderProps {
                label: "Radius".into(),
                min: 0.0,
                max: 10.0,
                step: 0.1,
                value: slider_value,
                disabled: false,
            }),
        },
        Desc::Widget {
            id: Cow::Borrowed("toggle_grid"),
            props: Box::new(ToggleProps {
                label: "Show Grid".into(),
                value: toggle_value,
                disabled: false,
            }),
        },
    ]
}

fn build_demo_tree(
    viewport: Rect,
    camera: &Camera,
    panel: PanelState,
    text_value: &str,
    slider_value: f32,
    toggle_value: bool,
) -> Desc {
    let (canvas_min_x, canvas_min_y) = camera.screen_to_canvas(0.0, 0.0);
    let (canvas_max_x, canvas_max_y) = camera.screen_to_canvas(viewport.w, viewport.h);
    let grid_x = canvas_min_x.floor() - GRID_SPACING * 2.0;
    let grid_y = canvas_min_y.floor() - GRID_SPACING * 2.0;
    let grid_w = (canvas_max_x - canvas_min_x).abs() + GRID_SPACING * 4.0;
    let grid_h = (canvas_max_y - canvas_min_y).abs() + GRID_SPACING * 4.0;

    Desc::Container {
        id: Cow::Borrowed("root"),
        style: BoxStyle {
            width: Size::Fixed(viewport.w),
            height: Size::Fixed(viewport.h),
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(Color {
                r: 0.071,
                g: 0.078,
                b: 0.098,
                a: 1.0,
            }),
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
                        dot_color: Color {
                            r: 0.224,
                            g: 0.235,
                            b: 0.278,
                            a: 1.0,
                        },
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
                children: vec![Desc::Widget {
                    id: Cow::Borrowed(PANEL_ID),
                    props: Box::new(PanelProps {
                        id: Cow::Borrowed(PANEL_ID),
                        title: Cow::Borrowed("Demo Panel"),
                        x: panel.x,
                        y: panel.y,
                        w: panel.w,
                        h: panel.h,
                        content: build_panel_content(text_value, slider_value, toggle_value),
                    }),
                }],
            },
        ],
    }
}

pub struct DemoApp {
    gui: Context,
    arena: Option<GestureArena>,
    pointer_session: Option<PointerSession>,
    panel: PanelState,
    camera: Camera,
    navigation: CanvasNavigationController,
    active_button: Option<String>,
    text_value: String,
    toggle_value: bool,
    slider_value: f32,
    last_tap_time: Option<Instant>,
    mouse_x: f32,
    mouse_y: f32,
}

impl App for DemoApp {
    fn init(_ctx: &mut AppContext) -> Self {
        Self {
            gui: Context::new(),
            arena: None,
            pointer_session: None,
            panel: PanelState::new(),
            camera: Camera::new(),
            navigation: CanvasNavigationController::new(),
            active_button: None,
            text_value: "Hello nodeimg".to_string(),
            toggle_value: true,
            slider_value: 5.0,
            last_tap_time: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    fn event(&mut self, event: AppEvent, ctx: &mut AppContext) {
        if !matches!(event, AppEvent::MouseMove { .. }) {
            tracing::info!("event: {:?}", event);
        }

        let outcome = self.gui.handle_event(&event);
        self.handle_context_outcome(outcome, ctx);

        if self.navigation.handle_event(&event, &mut self.camera) {
            if self.navigation.is_panning() {
                ctx.cursor.set(CursorStyle::Move);
            }
            match event {
                AppEvent::MouseMove { x, y }
                | AppEvent::MousePress { x, y, .. }
                | AppEvent::MouseRelease { x, y, .. } => {
                    self.mouse_x = x;
                    self.mouse_y = y;
                }
                _ => {}
            }
            return;
        }

        match event {
            AppEvent::MousePress { x, y, button } if button == MouseButton::Left => {
                self.mouse_x = x;
                self.mouse_y = y;

                if self.arena.is_none() {
                    let chain = self.gui.hit_test(x, y);
                    tracing::info!("hit chain on press: {:?}", chain);
                    if let Some(arena) =
                        arena_from_hit_chain(self.gui.tree(), &chain, x, y, self.last_tap_time)
                    {
                        tracing::info!("arena target: {}", arena.target_id());
                        self.pointer_session = Some(PointerSession::new(x, y));
                        self.arena = Some(arena);
                        return;
                    }
                }
            }
            AppEvent::MouseMove { x, y } => {
                self.mouse_x = x;
                self.mouse_y = y;

                if let Some(arena) = &mut self.arena {
                    if let Some(action) = arena.pointer_move(x, y) {
                        self.handle_action(action);
                    }
                    return;
                }

                self.update_hover_cursor(x, y, ctx);
            }
            AppEvent::MouseRelease { x, y, button } if button == MouseButton::Left => {
                self.mouse_x = x;
                self.mouse_y = y;
                if let Some(mut arena) = self.arena.take() {
                    if let Some(action) = arena.pointer_up(x, y) {
                        tracing::info!("pointer_up action: {:?}", action);
                        self.handle_action(action);
                    } else {
                        tracing::info!("pointer_up produced no action");
                    }
                }
                self.pointer_session = None;
            }
            _ => {}
        }
    }

    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext) {
        let viewport = viewport_rect(ctx);
        let desc = build_demo_tree(
            viewport,
            &self.camera,
            self.panel,
            &self.text_value,
            self.slider_value,
            self.toggle_value,
        );
        self.gui.update(desc, viewport, renderer.text_measurer());
        ctx.apply_ime_request(self.gui.ime_request());
        self.update_hover_cursor(self.mouse_x, self.mouse_y, ctx);
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        let viewport = viewport_rect(ctx);
        self.gui.render(renderer, viewport.w, viewport.h);
    }
}

impl DemoApp {
    fn handle_context_outcome(
        &mut self,
        outcome: gui::context::EventOutcome,
        ctx: &mut AppContext,
    ) {
        for action in outcome.actions {
            self.handle_action(action);
        }

        match outcome.clipboard {
            Some(ClipboardRequest::Copy(text)) | Some(ClipboardRequest::Cut(text)) => {
                let _ = ctx.clipboard_write_text(&text);
            }
            Some(ClipboardRequest::Paste) => {
                if let Some(text) = ctx.clipboard_read_text() {
                    for action in self.gui.paste_focused_text(&text) {
                        self.handle_action(action);
                    }
                }
            }
            None => {}
        }
    }

    fn handle_action(&mut self, action: Action) {
        tracing::info!("Action: {:?}", action);
        match action {
            Action::Click(id) => {
                tracing::info!("handle click: {}", id);
                self.last_tap_time = Some(Instant::now());
                if is_toggle_target(&id) {
                    self.toggle_value = !self.toggle_value;
                    tracing::info!("toggle_value -> {}", self.toggle_value);
                }
                if is_slider_target(&id) {
                    self.update_slider_from_pointer(self.mouse_x);
                    tracing::info!("slider_value(click) -> {}", self.slider_value);
                }
                self.active_button = Some(id);
            }
            Action::DoubleClick(id) => {
                self.last_tap_time = None;
                if id.contains("slider") {
                    self.slider_value = 5.0;
                }
            }
            Action::TextChange { id, value } => {
                if id == "text_prompt" {
                    self.text_value = value;
                }
            }
            Action::DragMove { id, x, y } => {
                let mut should_update_slider = false;
                if let Some(session) = &mut self.pointer_session {
                    if id == PANEL_ID {
                        self.panel.apply_drag(x, y, session.last_x, session.last_y);
                    } else if is_slider_target(&id) {
                        should_update_slider = true;
                    } else {
                        tracing::info!("Unhandled drag target: {} at ({}, {})", id, x, y);
                    }
                    session.last_x = x;
                    session.last_y = y;
                }
                if should_update_slider {
                    self.update_slider_from_pointer(x);
                    tracing::info!("slider_value(drag) -> {}", self.slider_value);
                }
            }
            Action::ResizeMove { id, edge, x, y } => {
                if id == PANEL_ID {
                    if let Some(session) = &mut self.pointer_session {
                        self.panel
                            .apply_resize(edge, x, y, session.last_x, session.last_y);
                        session.last_x = x;
                        session.last_y = y;
                    }
                }
            }
            Action::LongPress(id) => {
                tracing::info!("LongPress: {}", id);
            }
            Action::DragStart { .. }
            | Action::DragEnd { .. }
            | Action::ResizeStart { .. }
            | Action::ResizeEnd { .. } => {}
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

            if node.style.gestures.contains(&Gesture::Resize) {
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
            self.slider_value = value;
        }
    }
}

fn is_toggle_target(id: &str) -> bool {
    id == "toggle_grid" || id.starts_with("toggle_grid::")
}

fn is_slider_target(id: &str) -> bool {
    id == "slider_radius" || id.starts_with("slider_radius::")
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
