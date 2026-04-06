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
}
