use crate::canvas::Canvas;
use crate::controls::atoms::button::Button;
use crate::controls::infra::layout::{BoxStyle, Direction};
use crate::panel::{DragState, PanelFrame, PanelLayer, ResizeState, hit_test_panel};
use crate::panel_tree::{reconcile, layout, paint, hit_test, Desc, PanelTree};
use crate::renderer::{Color, Rect, Renderer};
use crate::shell::{App, AppContext, AppEvent};

const PADDING: f32 = 16.0;

const COLOR_ACTIVE: Color = Color { r: 0.2, g: 0.5, b: 0.9, a: 1.0 };
const COLOR_INACTIVE: Color = Color { r: 0.85, g: 0.85, b: 0.87, a: 1.0 };

pub struct DemoApp {
    canvas: Canvas,
    layer: PanelLayer,
    drag: DragState,
    resize: ResizeState,
    tree: PanelTree,
    active_button: Option<&'static str>,
    mouse_x: f32,
    mouse_y: f32,
}

impl DemoApp {
    fn view(&self) -> Desc {
        let active = self.active_button;
        Desc::Container {
            id: "__root",
            style: BoxStyle {
                direction: Direction::Column,
                gap: 8.0,
                ..BoxStyle::default()
            },
            children: vec![
                Desc::Widget(Box::new(Button {
                    id: "btn_a",
                    label: "Button A",
                    color: if active == Some("btn_a") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                })),
                Desc::Widget(Box::new(Button {
                    id: "btn_b",
                    label: "Button B",
                    color: if active == Some("btn_b") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                })),
            ],
        }
    }
}

impl App for DemoApp {
    fn init(_ctx: &mut AppContext) -> Self {
        let mut layer = PanelLayer::new();
        layer.add(PanelFrame::new("demo", 100.0, 100.0, 300.0, 200.0));

        Self {
            canvas: Canvas::new(),
            layer,
            drag: DragState::new(),
            resize: ResizeState::new(),
            tree: PanelTree::new(),
            active_button: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    fn event(&mut self, event: AppEvent, _ctx: &mut AppContext) {
        if !matches!(event, AppEvent::MouseMove { .. }) {
            tracing::debug!("{:?}", event);
        }

        // 面板优先拦截
        let panel_consumed = match &event {
            AppEvent::MousePress { x, y, button } => {
                if *button == crate::shell::MouseButton::Left {
                    if let Some(panel_id) = hit_test_panel(&self.layer, *x, *y) {
                        let frame = self.layer.get(panel_id).unwrap();

                        if let Some(edge) = ResizeState::detect_edge(frame, *x, *y) {
                            self.resize.start(panel_id, edge, *x, *y);
                        } else {
                            let mut button_hit = false;
                            if let Some(root) = self.tree.root() {
                                if let Some(id) = hit_test(&self.tree, root, *x, *y) {
                                    self.active_button = Some(id);
                                    button_hit = true;
                                }
                            }
                            if !button_hit {
                                self.drag.start(panel_id, *x, *y);
                            }
                        }

                        self.layer.bring_to_front(panel_id);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            AppEvent::MouseMove { x, y } => {
                self.mouse_x = *x;
                self.mouse_y = *y;
                let mut consumed = false;
                if self.drag.is_active() {
                    self.drag.update(*x, *y, &mut self.layer);
                    consumed = true;
                }
                if self.resize.is_active() {
                    self.resize.update(*x, *y, &mut self.layer);
                    consumed = true;
                }
                consumed
            }
            AppEvent::MouseRelease { .. } => {
                let was_active = self.drag.is_active() || self.resize.is_active();
                self.drag.end();
                self.resize.end();
                was_active
            }
            _ => false,
        };

        // 面板没消费的事件给画布
        if !panel_consumed {
            self.canvas.event(&event);
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        // 光标样式
        if let Some(edge) = self.resize.current_edge() {
            ctx.cursor.set(edge.cursor_style());
        } else if let Some(panel_id) = hit_test_panel(&self.layer, self.mouse_x, self.mouse_y) {
            if let Some(frame) = self.layer.get(panel_id) {
                if let Some(edge) = ResizeState::detect_edge(frame, self.mouse_x, self.mouse_y) {
                    ctx.cursor.set(edge.cursor_style());
                }
            }
        }
        let desc = self.view();
        reconcile(&mut self.tree, desc);
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        // layout 需要 renderer（文字度量），所以在 render 开头做
        if let (Some(frame), Some(root)) = (self.layer.get("demo"), self.tree.root()) {
            let content_rect = Rect {
                x: frame.x + PADDING,
                y: frame.y + PADDING,
                w: frame.w - PADDING * 2.0,
                h: frame.h - PADDING * 2.0,
            };
            layout(&mut self.tree, root, content_rect, renderer);
        }
        let viewport_w = ctx.size.width as f32 / ctx.scale_factor as f32;
        let viewport_h = ctx.size.height as f32 / ctx.scale_factor as f32;

        // 先渲染画布（背景 + 点阵）
        self.canvas.render(renderer, viewport_w, viewport_h);

        // 再渲染面板（在画布上方）
        let tree = &self.tree;
        self.layer.render(renderer, |_frame, renderer| {
            if let Some(root) = tree.root() {
                paint(tree, root, renderer);
            }
        });
    }
}
