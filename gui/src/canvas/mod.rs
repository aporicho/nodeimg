pub mod state;
pub mod controller;

pub use state::{CanvasState, Viewport, ActiveInteraction, HoverTarget};

use std::collections::HashSet;
use std::sync::Arc;

use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::event;
use iced::{Rectangle, Theme};

use engine::graph::Graph;
use engine::registry::NodeManager;
use types::NodeId;

use crate::render;

pub struct NodeCanvasData<'a> {
    pub graph: &'a Graph,
    pub node_manager: &'a Arc<NodeManager>,
    pub selection: &'a HashSet<NodeId>,
    pub canvas_state: &'a CanvasState,
}

pub struct NodeCanvas<'a> {
    data: NodeCanvasData<'a>,
}

impl<'a> NodeCanvas<'a> {
    pub fn new(data: NodeCanvasData<'a>) -> Self {
        Self { data }
    }
}

#[derive(Default)]
pub struct CanvasCache {
    cache: canvas::Cache,
}

impl<'a> canvas::Program<crate::Message> for NodeCanvas<'a> {
    type State = CanvasCache;

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (event::Status, Option<crate::Message>) {
        if let Some(pos) = cursor.position_in(bounds) {
            return (
                event::Status::Captured,
                Some(crate::Message::CanvasEvent(event, pos.x, pos.y)),
            );
        }
        (event::Status::Ignored, None)
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geom = state.cache.draw(renderer, bounds.size(), |frame| {
            let vp = &self.data.canvas_state.viewport;

            // 背景网格
            render::background::draw_grid(frame, vp.offset.x, vp.offset.y, vp.zoom, bounds.size());

            // 连线
            for conn in &self.data.graph.connections {
                if let (Some(from_n), Some(to_n)) = (
                    self.data.graph.nodes.get(&conn.from_node),
                    self.data.graph.nodes.get(&conn.to_node),
                ) {
                    if let (Some(fd), Some(td)) = (
                        self.data.node_manager.get(&from_n.type_id),
                        self.data.node_manager.get(&to_n.type_id),
                    ) {
                        let from_idx = fd.outputs.iter().position(|p| p.name == conn.from_pin).unwrap_or(0);
                        let to_idx = td.inputs.iter().position(|p| p.name == conn.to_pin).unwrap_or(0);
                        let from_pt = render::node::output_pin_position(
                            from_n.position.x,
                            from_n.position.y,
                            from_idx,
                            fd.inputs.len(),
                            vp.offset.x,
                            vp.offset.y,
                            vp.zoom,
                        );
                        let to_pt = render::node::input_pin_position(
                            to_n.position.x,
                            to_n.position.y,
                            to_idx,
                            vp.offset.x,
                            vp.offset.y,
                            vp.zoom,
                        );
                        let dt = fd.outputs.get(from_idx).map(|p| p.data_type.0.as_str()).unwrap_or("string");
                        render::connection::draw_connection(frame, from_pt, to_pt, dt);
                    }
                }
            }

            // 节点
            for (id, node) in &self.data.graph.nodes {
                if let Some(def) = self.data.node_manager.get(&node.type_id) {
                    let selected = self.data.selection.contains(id);
                    render::node::draw_node(
                        frame,
                        node.position.x,
                        node.position.y,
                        def,
                        selected,
                        vp.offset.x,
                        vp.offset.y,
                        vp.zoom,
                    );
                }
            }

            // 拖拽连线中的临时连线
            if let Some(ActiveInteraction::Connecting { from_node, from_pin, cursor }) =
                &self.data.canvas_state.interaction
            {
                if let Some(node) = self.data.graph.nodes.get(from_node) {
                    if let Some(def) = self.data.node_manager.get(&node.type_id) {
                        let idx = def.outputs.iter().position(|p| p.name == *from_pin).unwrap_or(0);
                        let from_pt = render::node::output_pin_position(
                            node.position.x,
                            node.position.y,
                            idx,
                            def.inputs.len(),
                            vp.offset.x,
                            vp.offset.y,
                            vp.zoom,
                        );
                        render::connection::draw_temp_connection(
                            frame,
                            from_pt,
                            iced::Point::new(cursor.x, cursor.y),
                        );
                    }
                }
            }
        });

        vec![geom]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match &self.data.canvas_state.interaction {
            Some(ActiveInteraction::Panning { .. }) => mouse::Interaction::Grabbing,
            Some(ActiveInteraction::Dragging { .. }) => mouse::Interaction::Grabbing,
            _ => mouse::Interaction::default(),
        }
    }
}
