use std::borrow::Cow;
use crate::canvas::Canvas;
use crate::widget::atoms::button::ButtonProps;
use crate::widget::layout::{BoxStyle, Direction, resolve};
use crate::panel::{DragState, PanelFrame, PanelLayer, ResizeState, hit_test_panel};
use crate::panel::tree::{reconcile, layout, paint, hit_test, Desc, PanelTree};
use crate::renderer::{Rect, Renderer};
use crate::shell::{App, AppContext, AppEvent};

const PADDING: f32 = 16.0;

fn build_view(_active: Option<&str>) -> Desc {
    Desc::Container {
        id: Cow::Borrowed("__root"),
        style: BoxStyle {
            direction: Direction::Column,
            gap: 8.0,
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![
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
        ],
    }
}

pub struct DemoApp {
    canvas: Canvas,
    layer: PanelLayer,
    drag: DragState,
    resize: ResizeState,
    tree: PanelTree,
    active_button: Option<String>,
    mouse_x: f32,
    mouse_y: f32,
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
                                    self.active_button = Some(id.to_string());
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

        if !panel_consumed {
            self.canvas.event(&event);
        }
    }

    fn update(&mut self, _renderer: &mut Renderer, ctx: &mut AppContext) {
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

        let desc = build_view(self.active_button.as_deref());
        reconcile(&mut self.tree, desc);
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        if let (Some(frame), Some(root)) = (self.layer.get("demo"), self.tree.root()) {
            let content_rect = Rect {
                x: frame.x + PADDING,
                y: frame.y + PADDING,
                w: frame.w - PADDING * 2.0,
                h: frame.h - PADDING * 2.0,
            };
            resolve::resolve(&mut self.tree, renderer.text_measurer());
            layout(&mut self.tree, root, content_rect);
        }
        let viewport_w = ctx.size.width as f32 / ctx.scale_factor as f32;
        let viewport_h = ctx.size.height as f32 / ctx.scale_factor as f32;

        self.canvas.render(renderer, viewport_w, viewport_h);

        let tree = &self.tree;
        self.layer.render(renderer, |_frame, renderer| {
            if let Some(root) = tree.root() {
                paint(tree, root, renderer);
            }
        });
    }
}
