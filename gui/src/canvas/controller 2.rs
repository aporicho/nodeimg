<<<<<<< HEAD
// canvas::controller — 画布交互控制器（Task 5 实现）

use std::collections::HashSet;
use std::sync::Arc;

use iced::widget::canvas::Event;
use iced::mouse;
use iced::Point;

use engine::graph::Graph;
use engine::graph::Connection;
use engine::registry::NodeManager;
use engine::graph_controller::GraphController;
use types::{NodeId, Vec2};

use super::state::{ActiveInteraction, CanvasState};
use crate::render::node::{
    node_bounds, input_pin_position, output_pin_position, PIN_RADIUS,
};

/// 命中测试命中输入引脚时，需要的距离阈值（屏幕像素）
const HIT_RADIUS: f32 = PIN_RADIUS * 2.0;

pub struct CanvasController;

impl CanvasController {
    /// 处理画布事件，更新 canvas_state / selection / graph_controller。
    pub fn handle_event(
        event: &Event,
        cursor_x: f32,
        cursor_y: f32,
        canvas_state: &mut CanvasState,
        graph: &Graph,
        node_manager: &Arc<NodeManager>,
        selection: &mut HashSet<NodeId>,
        graph_controller: &mut GraphController,
    ) {
        match event {
            // ── 鼠标按下 ──────────────────────────────────────────────────────────
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let vp = &canvas_state.viewport;

                // 1. 命中输出引脚 → 开始连线
                if let Some((node_id, pin_name)) =
                    Self::hit_output_pin(cursor_x, cursor_y, graph, node_manager, vp.offset.x, vp.offset.y, vp.zoom)
                {
                    canvas_state.interaction = Some(ActiveInteraction::Connecting {
                        from_node: node_id,
                        from_pin: pin_name,
                        cursor: Vec2::new(cursor_x, cursor_y),
                    });
                    return;
                }

                // 2. 命中节点 → 选中并开始拖拽
                if let Some(node_id) =
                    Self::hit_node(cursor_x, cursor_y, graph, node_manager, vp.offset.x, vp.offset.y, vp.zoom)
                {
                    selection.clear();
                    selection.insert(node_id);
                    canvas_state.interaction = Some(ActiveInteraction::Dragging {
                        node: node_id,
                        start_pos: Vec2::new(cursor_x, cursor_y),
                    });
                    return;
                }

                // 3. 命中空白 → 清除选中，开始平移
                selection.clear();
                canvas_state.interaction = Some(ActiveInteraction::Panning {
                    start_offset: canvas_state.viewport.offset,
                    start_cursor: Vec2::new(cursor_x, cursor_y),
                });
            }

            // ── 鼠标移动 ──────────────────────────────────────────────────────────
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                match canvas_state.interaction.clone() {
                    Some(ActiveInteraction::Panning { start_offset, start_cursor }) => {
                        let zoom = canvas_state.viewport.zoom;
                        // 屏幕坐标差除以 zoom → 画布坐标差
                        let dx = (cursor_x - start_cursor.x) / zoom;
                        let dy = (cursor_y - start_cursor.y) / zoom;
                        canvas_state.viewport.offset.x = start_offset.x + dx;
                        canvas_state.viewport.offset.y = start_offset.y + dy;
                    }

                    Some(ActiveInteraction::Dragging { node, .. }) => {
                        // 屏幕坐标 → 画布坐标
                        let canvas_pos = canvas_state.viewport.to_canvas(Vec2::new(cursor_x, cursor_y));
                        graph_controller.move_node(node, canvas_pos);
                    }

                    Some(ActiveInteraction::Connecting { from_node, from_pin, .. }) => {
                        canvas_state.interaction = Some(ActiveInteraction::Connecting {
                            from_node,
                            from_pin,
                            cursor: Vec2::new(cursor_x, cursor_y),
                        });
                    }

                    None => {}
                }
            }

            // ── 鼠标释放 ──────────────────────────────────────────────────────────
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(ActiveInteraction::Connecting { from_node, from_pin, .. }) =
                    canvas_state.interaction.clone()
                {
                    let vp = &canvas_state.viewport;
                    // 尝试命中输入引脚
                    if let Some((to_node, to_pin)) =
                        Self::hit_input_pin(cursor_x, cursor_y, graph, node_manager, vp.offset.x, vp.offset.y, vp.zoom)
                    {
                        // 不连接自身
                        if to_node != from_node {
                            let conn = Connection {
                                from_node,
                                from_pin,
                                to_node,
                                to_pin,
                            };
                            let _ = graph_controller.connect(conn);
                        }
                    }
                }
                canvas_state.interaction = None;
            }

            // ── 滚轮缩放 ──────────────────────────────────────────────────────────
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 16.0,
                };
                let factor = if lines > 0.0 { 1.1f32.powf(lines) } else { (1.0 / 1.1f32).powf(-lines) };
                let center = Vec2::new(cursor_x, cursor_y);
                canvas_state.viewport.zoom_at(center, factor);
            }

            _ => {}
        }
    }

    // ── 私有命中测试函数 ──────────────────────────────────────────────────────

    /// 命中测试：检查屏幕坐标是否落在某个节点矩形内。
    fn hit_node(
        cx: f32,
        cy: f32,
        graph: &Graph,
        node_manager: &Arc<NodeManager>,
        offset_x: f32,
        offset_y: f32,
        zoom: f32,
    ) -> Option<NodeId> {
        for (id, node) in &graph.nodes {
            if let Some(def) = node_manager.get(&node.type_id) {
                let rect = node_bounds(node.position.x, node.position.y, def, offset_x, offset_y, zoom);
                if rect.contains(Point::new(cx, cy)) {
                    return Some(*id);
                }
            }
        }
        None
    }

    /// 命中测试：检查屏幕坐标是否在某个输入引脚的命中半径内。
    fn hit_input_pin(
        cx: f32,
        cy: f32,
        graph: &Graph,
        node_manager: &Arc<NodeManager>,
        offset_x: f32,
        offset_y: f32,
        zoom: f32,
    ) -> Option<(NodeId, String)> {
        for (id, node) in &graph.nodes {
            if let Some(def) = node_manager.get(&node.type_id) {
                for (i, pin) in def.inputs.iter().enumerate() {
                    let pt = input_pin_position(
                        node.position.x,
                        node.position.y,
                        i,
                        offset_x,
                        offset_y,
                        zoom,
                    );
                    let dx = pt.x - cx;
                    let dy = pt.y - cy;
                    if (dx * dx + dy * dy).sqrt() <= HIT_RADIUS * zoom {
                        return Some((*id, pin.name.clone()));
                    }
                }
            }
        }
        None
    }

    /// 命中测试：检查屏幕坐标是否在某个输出引脚的命中半径内。
    fn hit_output_pin(
        cx: f32,
        cy: f32,
        graph: &Graph,
        node_manager: &Arc<NodeManager>,
        offset_x: f32,
        offset_y: f32,
        zoom: f32,
    ) -> Option<(NodeId, String)> {
        for (id, node) in &graph.nodes {
            if let Some(def) = node_manager.get(&node.type_id) {
                let num_inputs = def.inputs.len();
                for (i, pin) in def.outputs.iter().enumerate() {
                    let pt = output_pin_position(
                        node.position.x,
                        node.position.y,
                        i,
                        num_inputs,
                        offset_x,
                        offset_y,
                        zoom,
                    );
                    let dx = pt.x - cx;
                    let dy = pt.y - cy;
                    if (dx * dx + dy * dy).sqrt() <= HIT_RADIUS * zoom {
                        return Some((*id, pin.name.clone()));
                    }
                }
            }
        }
        None
    }
=======
//! 画布控制器 (2.1.1)
//! 维护 CanvasState，处理画布内所有交互事件。

use iced::keyboard;
use iced::mouse;
use iced::widget::canvas;
use iced::{Point, Rectangle, Vector};

const MIN_SCALE: f32 = 0.1;
const MAX_SCALE: f32 = 5.0;
const ZOOM_STEP: f32 = 0.03;

/// 画布状态：视口变换 + 交互状态。
#[derive(Debug)]
pub struct CanvasState {
    /// 视口偏移
    pub offset: Vector,
    /// 视口缩放
    pub scale: f32,
    /// 拖拽中：记录鼠标按下时的光标位置
    grab_origin: Option<Point>,
    /// 拖拽开始时的 offset 快照
    starting_offset: Vector,
    /// Space 键是否按下（Space+左键拖拽平移）
    space_held: bool,
    /// 当前修饰键状态
    modifiers: keyboard::Modifiers,
    /// 网格缓存
    pub grid_cache: canvas::Cache,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            offset: Vector::ZERO,
            scale: 1.0,
            grab_origin: None,
            starting_offset: Vector::ZERO,
            space_held: false,
            modifiers: keyboard::Modifiers::empty(),
            grid_cache: canvas::Cache::new(),
        }
    }
}

impl CanvasState {
    /// 处理画布事件。
    pub fn handle_event<Message>(
        &mut self,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            // ── 键盘：跟踪 Space 和修饰键 ──
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Space),
                ..
            }) => {
                self.space_held = true;
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(keyboard::key::Named::Space),
                ..
            }) => {
                self.space_held = false;
                if self.grab_origin.is_some() {
                    self.grab_origin = None;
                }
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                self.modifiers = *modifiers;
                None
            }

            // +/= 键 → 放大
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ch),
                ..
            }) if ch.as_str() == "+" || ch.as_str() == "=" => {
                let center = Point::new(
                    bounds.x + bounds.width / 2.0,
                    bounds.y + bounds.height / 2.0,
                );
                self.zoom_at(center, bounds, 1.0);
                Some(canvas::Action::request_redraw().and_capture())
            }
            // - 键 → 缩小
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ch),
                ..
            }) if ch.as_str() == "-" => {
                let center = Point::new(
                    bounds.x + bounds.width / 2.0,
                    bounds.y + bounds.height / 2.0,
                );
                self.zoom_at(center, bounds, -1.0);
                Some(canvas::Action::request_redraw().and_capture())
            }
            // Cmd+0 → 重置视口
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ch),
                ..
            }) if ch.as_str() == "0" && self.modifiers.command() => {
                self.offset = Vector::ZERO;
                self.scale = 1.0;
                self.grid_cache.clear();
                Some(canvas::Action::request_redraw().and_capture())
            }

            // ── 鼠标按下 ──
            canvas::Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                let pos = cursor.position_over(bounds)?;

                let should_pan = match button {
                    mouse::Button::Middle => true,
                    mouse::Button::Left if self.space_held => true,
                    _ => false,
                };

                if should_pan {
                    self.grab_origin = Some(pos);
                    self.starting_offset = self.offset;
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }

            // ── 鼠标释放 ──
            canvas::Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                if self.grab_origin.is_some()
                    && matches!(button, mouse::Button::Middle | mouse::Button::Left)
                {
                    self.grab_origin = None;
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }

            // ── 光标移动 → 拖拽中更新偏移 ──
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(origin) = self.grab_origin {
                    let pos = cursor.position_over(bounds)?;
                    let delta = pos - origin;
                    self.offset = self.starting_offset + delta;
                    self.grid_cache.clear();
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                None
            }

            // ── 滚轮 ──
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let cursor_position = cursor.position_over(bounds)?;

                match delta {
                    // x = INFINITY → 触控板捏合（patch 标记），直接缩放
                    mouse::ScrollDelta::Lines { x, y } if x.is_infinite() => {
                        self.zoom_at(cursor_position, bounds, *y * 20.0);
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                    // Cmd+滚轮 → 缩放
                    _ if self.modifiers.command() => {
                        let dy = match delta {
                            mouse::ScrollDelta::Lines { y, .. } => *y * 20.0,
                            mouse::ScrollDelta::Pixels { y, .. } => *y,
                        };
                        self.zoom_at(cursor_position, bounds, dy);
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                    // 普通滚动 → 平移
                    _ => {
                        let (dx, dy) = match delta {
                            mouse::ScrollDelta::Lines { x, y } => (*x * 20.0, *y * 20.0),
                            mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                        };
                        self.offset = self.offset + Vector::new(dx, dy);
                        self.grid_cache.clear();
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                }
            }

            _ => None,
        }
    }

    /// 以指定位置为中心缩放。
    fn zoom_at(&mut self, cursor_position: Point, bounds: Rectangle, scroll_y: f32) {
        if (scroll_y < 0.0 && self.scale <= MIN_SCALE)
            || (scroll_y > 0.0 && self.scale >= MAX_SCALE)
        {
            return;
        }

        let old_scale = self.scale;
        self.scale = if scroll_y > 0.0 {
            self.scale * (1.0 + ZOOM_STEP)
        } else {
            self.scale / (1.0 + ZOOM_STEP)
        }
        .clamp(MIN_SCALE, MAX_SCALE);

        let cursor_in_bounds =
            Vector::new(cursor_position.x - bounds.x, cursor_position.y - bounds.y);
        let factor = self.scale / old_scale;
        self.offset = Vector::new(
            cursor_in_bounds.x - (cursor_in_bounds.x - self.offset.x) * factor,
            cursor_in_bounds.y - (cursor_in_bounds.y - self.offset.y) * factor,
        );

        self.grid_cache.clear();
    }

    pub fn is_panning(&self) -> bool {
        self.grab_origin.is_some()
    }

    pub fn space_held(&self) -> bool {
        self.space_held
    }
>>>>>>> 08a5b55 (feat: iced 0.14 无限画布 + 触控板捏合缩放 patch)
}
