use std::borrow::Cow;
use std::time::Instant;

use gui::context::Context;
use gui::gesture::{GestureArena, TapRecognizer, DragRecognizer, GestureRecognizer};
use gui::widget::action::Action;
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::slider::SliderProps;
use gui::widget::layout::{BoxStyle, Direction};
use gui::panel::{PanelFrame, ResizeEdge, apply_drag_move, apply_resize, detect_edge, hit_test_panel};
use gui::panel::tree::Desc;
use gui::renderer::{Rect, Renderer};
use gui::shell::{App, AppContext, AppEvent, MouseButton};

const PADDING: f32 = 16.0;

fn build_view(_active: Option<&str>, slider_value: f32) -> Desc {
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
                props: Box::new(ButtonProps { label: "Button A".into(), icon: None, disabled: false }),
            },
            Desc::Widget {
                id: Cow::Borrowed("btn_b"),
                props: Box::new(ButtonProps { label: "Button B".into(), icon: None, disabled: false }),
            },
            Desc::Widget {
                id: Cow::Borrowed("slider_radius"),
                props: Box::new(SliderProps { label: "Radius".into(), min: 0.0, max: 10.0, step: 0.1, value: slider_value, disabled: false }),
            },
        ],
    }
}

struct PanelInteraction {
    drag_panel: Option<&'static str>,
    resize_panel: Option<&'static str>,
    resize_edge: Option<ResizeEdge>,
    last_x: f32,
    last_y: f32,
}

impl PanelInteraction {
    fn new() -> Self {
        Self { drag_panel: None, resize_panel: None, resize_edge: None, last_x: 0.0, last_y: 0.0 }
    }
}

pub struct DemoApp {
    gui: Context,
    arena: Option<GestureArena>,
    panel_interaction: PanelInteraction,
    active_button: Option<String>,
    slider_value: f32,
    last_tap_time: Option<Instant>,
    mouse_x: f32,
    mouse_y: f32,
}

impl App for DemoApp {
    fn init(_ctx: &mut AppContext) -> Self {
        let mut gui = Context::new();
        gui.layer.add(PanelFrame::new("demo", 100.0, 100.0, 300.0, 200.0));
        Self {
            gui,
            arena: None, panel_interaction: PanelInteraction::new(),
            active_button: None, slider_value: 5.0, last_tap_time: None,
            mouse_x: 0.0, mouse_y: 0.0,
        }
    }

    fn event(&mut self, event: AppEvent, ctx: &mut AppContext) {
        if !matches!(event, AppEvent::MouseMove { .. }) {
            tracing::debug!("{:?}", event);
        }

        match &event {
            AppEvent::MousePress { x, y, button } if *button == MouseButton::Left => {
                if self.arena.is_some() { return; }
                self.mouse_x = *x;
                self.mouse_y = *y;

                if let Some(panel_id) = hit_test_panel(&self.gui.layer, *x, *y) {
                    let frame = self.gui.layer.get(panel_id).unwrap();

                    if let Some(edge) = detect_edge(frame, *x, *y) {
                        let mut arena = GestureArena::new(format!("panel_resize:{panel_id}"));
                        let mut rec = DragRecognizer::new(format!("panel_resize:{panel_id}"));
                        rec.on_pointer_down(*x, *y);
                        arena.add(Box::new(rec));
                        self.arena = Some(arena);
                        self.panel_interaction.resize_panel = Some(panel_id);
                        self.panel_interaction.resize_edge = Some(edge);
                        self.panel_interaction.last_x = *x;
                        self.panel_interaction.last_y = *y;
                    } else if self.gui.panel.root().is_some() {
                        if let Some(widget_id) = self.gui.panel.hit_test(*x, *y) {
                            let mut arena = GestureArena::new(widget_id.to_string());
                            let mut tap = TapRecognizer::new(widget_id.to_string(), self.last_tap_time);
                            tap.on_pointer_down(*x, *y);
                            arena.add(Box::new(tap));

                            if widget_id.contains("slider") || widget_id.contains("track") || widget_id.contains("fill") || widget_id.contains("spacer") {
                                let mut drag = DragRecognizer::new(widget_id.to_string());
                                drag.on_pointer_down(*x, *y);
                                arena.add(Box::new(drag));
                            }
                            self.arena = Some(arena);
                        } else {
                            let mut arena = GestureArena::new(format!("panel_drag:{panel_id}"));
                            let mut rec = DragRecognizer::new(format!("panel_drag:{panel_id}"));
                            rec.on_pointer_down(*x, *y);
                            arena.add(Box::new(rec));
                            self.arena = Some(arena);
                            self.panel_interaction.drag_panel = Some(panel_id);
                            self.panel_interaction.last_x = *x;
                            self.panel_interaction.last_y = *y;
                        }
                    } else {
                        let mut arena = GestureArena::new(format!("panel_drag:{panel_id}"));
                        let mut rec = DragRecognizer::new(format!("panel_drag:{panel_id}"));
                        rec.on_pointer_down(*x, *y);
                        arena.add(Box::new(rec));
                        self.arena = Some(arena);
                        self.panel_interaction.drag_panel = Some(panel_id);
                        self.panel_interaction.last_x = *x;
                        self.panel_interaction.last_y = *y;
                    }
                    self.gui.layer.bring_to_front(panel_id);
                    return;
                }
                self.gui.canvas.event(&event);
            }

            AppEvent::MouseMove { x, y } => {
                self.mouse_x = *x;
                self.mouse_y = *y;
                if let Some(arena) = &mut self.arena {
                    if let Some(action) = arena.pointer_move(*x, *y) {
                        self.handle_action(action);
                    }
                } else {
                    if let Some(panel_id) = hit_test_panel(&self.gui.layer, *x, *y) {
                        if let Some(frame) = self.gui.layer.get(panel_id) {
                            if let Some(edge) = detect_edge(frame, *x, *y) {
                                ctx.cursor.set(edge.cursor_style());
                            }
                        }
                    }
                    self.gui.canvas.event(&event);
                }
            }

            AppEvent::MouseRelease { x, y, button } if *button == MouseButton::Left => {
                if let Some(mut arena) = self.arena.take() {
                    if let Some(action) = arena.pointer_up(*x, *y) {
                        self.handle_action(action);
                    }
                    self.panel_interaction.drag_panel = None;
                    self.panel_interaction.resize_panel = None;
                    self.panel_interaction.resize_edge = None;
                } else {
                    self.gui.canvas.event(&event);
                }
            }

            _ => { self.gui.canvas.event(&event); }
        }
    }

    fn update(&mut self, _renderer: &mut Renderer, _ctx: &mut AppContext) {
    }

    fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
        if let Some(frame) = self.gui.layer.get("demo") {
            let content_rect = Rect {
                x: frame.x + PADDING, y: frame.y + PADDING,
                w: frame.w - PADDING * 2.0, h: frame.h - PADDING * 2.0,
            };
            let desc = build_view(self.active_button.as_deref(), self.slider_value);
            self.gui.panel.update(desc, content_rect, renderer.text_measurer());
        }
        let viewport_w = ctx.size.width as f32 / ctx.scale_factor as f32;
        let viewport_h = ctx.size.height as f32 / ctx.scale_factor as f32;
        self.gui.render(renderer, viewport_w, viewport_h);
    }
}

impl DemoApp {
    fn handle_action(&mut self, action: Action) {
        tracing::debug!("Action: {:?}", action);
        match &action {
            Action::Click(id) => {
                self.last_tap_time = Some(Instant::now());
                self.active_button = Some(id.clone());
            }
            Action::DoubleClick(id) => {
                self.last_tap_time = None;
                if id.contains("slider") { self.slider_value = 5.0; }
            }
            Action::DragMove { id: _, x, y } => {
                if let Some(panel_id) = self.panel_interaction.drag_panel {
                    apply_drag_move(&mut self.gui.layer, panel_id, *x, *y, self.panel_interaction.last_x, self.panel_interaction.last_y);
                    self.panel_interaction.last_x = *x;
                    self.panel_interaction.last_y = *y;
                } else if let Some(panel_id) = self.panel_interaction.resize_panel {
                    if let Some(edge) = self.panel_interaction.resize_edge {
                        apply_resize(&mut self.gui.layer, panel_id, edge, *x, *y, self.panel_interaction.last_x, self.panel_interaction.last_y);
                        self.panel_interaction.last_x = *x;
                        self.panel_interaction.last_y = *y;
                    }
                } else {
                    tracing::debug!("Slider drag at ({}, {})", x, y);
                }
            }
            Action::DragStart { .. } | Action::DragEnd { .. } => {}
            Action::LongPress(id) => { tracing::debug!("LongPress: {}", id); }
        }
    }
}
