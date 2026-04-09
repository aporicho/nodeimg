use std::borrow::Cow;
use crate::canvas::Canvas;
use crate::widget::layout::{Align, BoxStyle, Decoration, Direction, Justify, LeafKind, Size};
use crate::panel::{DragState, PanelFrame, PanelLayer, ResizeState, hit_test_panel};
use crate::panel::tree::{reconcile, layout, paint, hit_test, Desc, PanelTree};
use crate::renderer::{Border, Color, Rect, Renderer};
use crate::shell::{App, AppContext, AppEvent};

const PADDING: f32 = 16.0;
const FONT_SIZE: f32 = 12.0;
const PADDING_V: f32 = 8.0;
const RADIUS: f32 = 4.0;
const BORDER_COLOR: Color = Color { r: 0.894, g: 0.894, b: 0.906, a: 1.0 };
const TEXT_COLOR: Color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };
const COLOR_ACTIVE: Color = Color { r: 0.2, g: 0.5, b: 0.9, a: 1.0 };
const COLOR_INACTIVE: Color = Color { r: 0.85, g: 0.85, b: 0.87, a: 1.0 };

fn button(
    id: &'static str,
    label: &'static str,
    color: Color,
    renderer: &mut Renderer,
) -> Desc {
    let (text_w, text_h) = renderer.measure_text(label, FONT_SIZE);
    Desc::Container {
        id: Cow::Borrowed(id),
        style: BoxStyle {
            height: Size::Fixed(text_h + PADDING_V * 2.0),
            direction: Direction::Row,
            align_items: Align::Center,
            justify_content: Justify::Center,
            ..BoxStyle::default()
        },
        decoration: Some(Decoration {
            background: Some(color),
            border: Some(Border { width: 1.0, color: BORDER_COLOR }),
            radius: [RADIUS; 4],
        }),
        children: vec![Desc::Leaf {
            id: Cow::Owned(format!("{id}::text")),
            style: BoxStyle {
                width: Size::Fixed(text_w),
                height: Size::Fixed(text_h),
                ..BoxStyle::default()
            },
            kind: LeafKind::Text {
                content: label.to_string(),
                font_size: FONT_SIZE,
                color: TEXT_COLOR,
            },
        }],
    }
}

fn build_view(active: Option<&str>, renderer: &mut Renderer) -> Desc {
    Desc::Container {
        id: Cow::Borrowed("__root"),
        style: BoxStyle {
            direction: Direction::Column,
            gap: 8.0,
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![
            button(
                "btn_a", "Button A",
                if active == Some("btn_a") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                renderer,
            ),
            button(
                "btn_b", "Button B",
                if active == Some("btn_b") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                renderer,
            ),
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

    fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext) {
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

        let desc = build_view(self.active_button.as_deref(), renderer);
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
