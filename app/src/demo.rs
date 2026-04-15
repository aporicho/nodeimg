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
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::checkbox::CheckboxProps;
use gui::widget::atoms::dropdown::DropdownProps;
use gui::widget::atoms::image_viewer::ImageViewerProps;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::atoms::number_input::NumberInputProps;
use gui::widget::atoms::radio::RadioProps;
use gui::widget::atoms::separator::{SeparatorOrientation, SeparatorProps};
use gui::widget::atoms::slider::SliderProps;
use gui::widget::atoms::text_input::TextInputProps;
use gui::widget::atoms::toggle::ToggleProps;
use gui::widget::frameworks::collapsible::CollapsibleProps;
use gui::widget::frameworks::group::GroupProps;
use gui::widget::frameworks::list_view::ListViewProps;
use gui::widget::frameworks::panel::PanelProps;
use gui::widget::resize_edge::ResizeEdge;
use std::borrow::Cow;

const GRID_SPACING: f32 = 20.0;
const GRID_DOT_SIZE: f32 = 1.5;
const PANEL_ID: &str = "demo_panel";
const DEMO_IMAGE_HANDLE: TextureHandle = TextureHandle(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QualityMode {
    Fast,
    Balanced,
}

fn align_grid_start(min_canvas: f32, spacing: f32) -> f32 {
    (min_canvas / spacing).floor() * spacing - spacing * 2.0
}

#[derive(Debug, Clone, Copy)]
struct PanelState {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    auto_h: bool,
    min_w: f32,
    min_h: f32,
}

impl PanelState {
    fn new() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            w: 300.0,
            h: 0.0,
            auto_h: true,
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

fn build_panel_content(
    text_value: &str,
    slider_value: f32,
    toggle_value: bool,
    snap_to_grid: bool,
    quality_mode: QualityMode,
    advanced_open: bool,
    blend_mode: usize,
) -> Vec<Desc> {
    let list_items = (1..=18)
        .map(|index| Desc::Widget {
            id: Cow::Owned(format!("history_item_{index}")),
            props: Box::new(LabelProps {
                text: Cow::Owned(format!("Preset #{index:02} · Gaussian Blur")),
                variant: LabelVariant::Body,
                muted: index % 2 == 0,
            }),
        })
        .collect();

    vec![
        Desc::Widget {
            id: Cow::Borrowed("intro_label"),
            props: Box::new(LabelProps {
                text: Cow::Borrowed("Shadcn 风格基础控件演示"),
                variant: LabelVariant::Title,
                muted: false,
            }),
        },
        Desc::Widget {
            id: Cow::Borrowed("intro_caption"),
            props: Box::new(LabelProps {
                text: Cow::Borrowed(
                    "下面的按钮、输入框、滑块和开关现在都建立在统一主题和框架边界上。",
                ),
                variant: LabelVariant::Caption,
                muted: true,
            }),
        },
        Desc::Widget {
            id: Cow::Borrowed("top_separator"),
            props: Box::new(SeparatorProps {
                orientation: SeparatorOrientation::Horizontal,
            }),
        },
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
            id: Cow::Borrowed("popup_trigger"),
            props: Box::new(ButtonProps {
                label: "Open Popup".into(),
                icon: None,
                disabled: false,
            }),
        },
        Desc::Widget {
            id: Cow::Borrowed("controls_group"),
            props: Box::new(GroupProps {
                title: Cow::Borrowed("Parameters"),
                content: vec![
                    Desc::Widget {
                        id: Cow::Borrowed("text_prompt"),
                        props: Box::new(TextInputProps {
                            label: "Prompt".into(),
                            value: Cow::Owned(text_value.to_string()),
                            disabled: false,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("group_separator"),
                        props: Box::new(SeparatorProps {
                            orientation: SeparatorOrientation::Horizontal,
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
                        id: Cow::Borrowed("number_radius"),
                        props: Box::new(NumberInputProps {
                            label: "Radius value".into(),
                            value: slider_value,
                            min: 0.0,
                            max: 10.0,
                            step: 0.1,
                            precision: 2,
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
                    Desc::Widget {
                        id: Cow::Borrowed("checkbox_snap"),
                        props: Box::new(CheckboxProps {
                            label: "Snap to grid".into(),
                            checked: snap_to_grid,
                            disabled: false,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("dropdown_blend"),
                        props: Box::new(DropdownProps {
                            label: Cow::Borrowed("Blend Mode"),
                            options: vec![
                                Cow::Borrowed("Normal"),
                                Cow::Borrowed("Multiply"),
                                Cow::Borrowed("Screen"),
                            ],
                            selected: blend_mode,
                            disabled: false,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("radio_quality_fast"),
                        props: Box::new(RadioProps {
                            label: "Fast quality".into(),
                            selected: quality_mode == QualityMode::Fast,
                            disabled: false,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("radio_quality_balanced"),
                        props: Box::new(RadioProps {
                            label: "Balanced quality".into(),
                            selected: quality_mode == QualityMode::Balanced,
                            disabled: false,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("preview_label"),
                        props: Box::new(LabelProps {
                            text: Cow::Borrowed("Preview"),
                            variant: LabelVariant::Caption,
                            muted: true,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("preview_image"),
                        props: Box::new(ImageViewerProps {
                            texture: DEMO_IMAGE_HANDLE,
                            height: 96.0,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("history_separator"),
                        props: Box::new(SeparatorProps {
                            orientation: SeparatorOrientation::Horizontal,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("history_list"),
                        props: Box::new(ListViewProps {
                            height: 128.0,
                            items: list_items,
                        }),
                    },
                    Desc::Widget {
                        id: Cow::Borrowed("advanced_section"),
                        props: Box::new(CollapsibleProps {
                            title: Cow::Borrowed("Advanced"),
                            expanded: advanced_open,
                            disabled: false,
                            content: vec![
                                Desc::Widget {
                                    id: Cow::Borrowed("advanced_label"),
                                    props: Box::new(LabelProps {
                                        text: Cow::Borrowed(
                                            "Collapsible 目前是 controlled 模式，点击头部会由 app 层切换 expanded。",
                                        ),
                                        variant: LabelVariant::Caption,
                                        muted: true,
                                    }),
                                },
                                Desc::Widget {
                                    id: Cow::Borrowed("advanced_separator"),
                                    props: Box::new(SeparatorProps {
                                        orientation: SeparatorOrientation::Horizontal,
                                    }),
                                },
                                Desc::Widget {
                                    id: Cow::Borrowed("advanced_toggle"),
                                    props: Box::new(ToggleProps {
                                        label: "Use denoise pass".into(),
                                        value: toggle_value,
                                        disabled: false,
                                    }),
                                },
                            ],
                        }),
                    },
                ],
            }),
        },
    ]
}

fn build_demo_popup() -> Desc {
    Desc::Widget {
        id: Cow::Borrowed("demo_popup_group"),
        props: Box::new(GroupProps {
            title: Cow::Borrowed("Quick Actions"),
            content: vec![
                Desc::Widget {
                    id: Cow::Borrowed("popup_label"),
                    props: Box::new(LabelProps {
                        text: Cow::Borrowed(
                            "Overlay 现在由 framework 持有，支持 outside click 和 Escape 关闭。",
                        ),
                        variant: LabelVariant::Caption,
                        muted: true,
                    }),
                },
                Desc::Widget {
                    id: Cow::Borrowed("popup_separator"),
                    props: Box::new(SeparatorProps {
                        orientation: SeparatorOrientation::Horizontal,
                    }),
                },
                Desc::Widget {
                    id: Cow::Borrowed("popup_close"),
                    props: Box::new(ButtonProps {
                        label: "Close Popup".into(),
                        icon: None,
                        disabled: false,
                    }),
                },
            ],
        }),
    }
}

fn build_demo_tree(
    viewport: Rect,
    camera: &Camera,
    theme: &Theme,
    panel: PanelState,
    text_value: &str,
    slider_value: f32,
    toggle_value: bool,
    snap_to_grid: bool,
    quality_mode: QualityMode,
    advanced_open: bool,
    blend_mode: usize,
) -> Desc {
    let (canvas_min_x, canvas_min_y) = camera.screen_to_canvas(0.0, 0.0);
    let (canvas_max_x, canvas_max_y) = camera.screen_to_canvas(viewport.w, viewport.h);
    let grid_x = align_grid_start(canvas_min_x, GRID_SPACING);
    let grid_y = align_grid_start(canvas_min_y, GRID_SPACING);
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
                children: vec![Desc::Widget {
                    id: Cow::Borrowed(PANEL_ID),
                    props: Box::new(PanelProps {
                        id: Cow::Borrowed(PANEL_ID),
                        title: Cow::Borrowed("Demo Panel"),
                        x: panel.x,
                        y: panel.y,
                        w: panel.w,
                        h: panel.h,
                        content: build_panel_content(
                            text_value,
                            slider_value,
                            toggle_value,
                            snap_to_grid,
                            quality_mode,
                            advanced_open,
                            blend_mode,
                        ),
                    }),
                }],
            },
        ],
    }
}

pub struct DemoApp {
    gui: Context,
    pointer_session: Option<PointerSession>,
    panel: PanelState,
    camera: Camera,
    navigation: CanvasNavigationController,
    active_button: Option<String>,
    text_value: String,
    toggle_value: bool,
    snap_to_grid: bool,
    quality_mode: QualityMode,
    advanced_open: bool,
    blend_mode: usize,
    slider_value: f32,
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
            panel: PanelState::new(),
            camera: Camera::new(),
            navigation: CanvasNavigationController::new(),
            active_button: None,
            text_value: "Hello nodeimg".to_string(),
            toggle_value: true,
            snap_to_grid: true,
            quality_mode: QualityMode::Balanced,
            advanced_open: true,
            blend_mode: 0,
            slider_value: 5.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            theme: light_theme(),
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
        let mut panel = self.panel;
        if panel.auto_h {
            panel.h = 0.0;
        }
        let desc = build_demo_tree(
            viewport,
            &self.camera,
            &self.theme,
            panel,
            &self.text_value,
            self.slider_value,
            self.toggle_value,
            self.snap_to_grid,
            self.quality_mode,
            self.advanced_open,
            self.blend_mode,
        );
        self.gui
            .update(desc, viewport, renderer.text_measurer(), &self.theme);
        self.auto_grow_panel_to_fit_content();
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
    fn current_panel_height(&self) -> Option<f32> {
        self.gui
            .tree()
            .iter()
            .find_map(|(_, node)| (node.id.as_ref() == PANEL_ID).then_some(node.rect.h))
    }

    fn auto_grow_panel_to_fit_content(&mut self) {
        if !self.panel.auto_h {
            return;
        }
        if let Some(height) = self.current_panel_height() {
            self.panel.h = height.max(self.panel.min_h);
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
                if is_toggle_target(&id) {
                    self.toggle_value = !self.toggle_value;
                    tracing::info!("toggle_value -> {}", self.toggle_value);
                }
                if id == "checkbox_snap" {
                    self.snap_to_grid = !self.snap_to_grid;
                }
                if id == "radio_quality_fast" {
                    self.quality_mode = QualityMode::Fast;
                }
                if id == "radio_quality_balanced" {
                    self.quality_mode = QualityMode::Balanced;
                }
                if id == "advanced_section::header" {
                    self.advanced_open = !self.advanced_open;
                }
                if id == "advanced_toggle" {
                    self.toggle_value = !self.toggle_value;
                }
                if id == "popup_trigger" {
                    if self.gui.overlay_open() {
                        self.gui.close_overlay();
                    } else {
                        self.gui.open_overlay(OverlayRequest {
                            id: "demo_popup".to_string(),
                            anchor_id: "popup_trigger".to_string(),
                            restore_focus_id: Some("popup_trigger".to_string()),
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
                if id == "popup_close" {
                    self.gui.close_overlay();
                }
                if is_slider_target(&id) {
                    self.update_slider_from_pointer(self.mouse_x);
                    tracing::info!("slider_value(click) -> {}", self.slider_value);
                }
                self.active_button = Some(id);
            }
            Action::DoubleClick(id) => {
                if id.contains("slider") {
                    self.slider_value = 5.0;
                }
            }
            Action::TextChange { id, value } => {
                if id == "text_prompt" {
                    self.text_value = value;
                }
            }
            Action::NumberChange { id, value } => {
                if id == "number_radius" {
                    self.slider_value = value;
                }
            }
            Action::SelectChange { id, selected } => {
                if id == "dropdown_blend" {
                    self.blend_mode = selected;
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
            Action::DragStart { x, y, .. } => {
                self.pointer_session = Some(PointerSession::new(x, y));
            }
            Action::ResizeStart { id, x, y, .. } => {
                if id == PANEL_ID && self.panel.auto_h {
                    if let Some(height) = self.current_panel_height() {
                        self.panel.h = height.max(self.panel.min_h);
                    }
                    self.panel.auto_h = false;
                }
                self.pointer_session = Some(PointerSession::new(x, y));
            }
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
