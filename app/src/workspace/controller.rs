use super::canvas_drag::CanvasNodeDragController;
use super::engine_adapter;
use super::node_palette::NodePaletteState;
use super::project_layout;
use super::project_layout::ProjectLayout;
use super::showcase_node;
use crate::image_demo::ImageDemoController;
use crate::panels::EnginePanelState;
use engine::facade::EngineFacade;
use engine::graph::validate::validate_connection_basic;
use engine::graph::{Connection, PinRef};
use engine::node_manager::ParamExpose;
use engine::Engine;
use gui::action::GuiAction;
use gui::canvas::camera::Camera;
use gui::canvas::node_card::CanvasNodeView;
use gui::canvas::{
    canvas_node_owner_id, canvas_port_event_target_id, canvas_port_stable_id,
    parse_canvas_port_group_trigger_id, parse_canvas_port_id, CanvasConnectionView,
    CanvasPendingConnectionView, CanvasPortConnectionState, CanvasPortRef, CanvasPortSide,
    CanvasPortView,
};
use gui::context::Context;

pub(crate) struct WorkspaceController {
    engine: Engine,
    image_demo: ImageDemoController,
    canvas_node_drag: CanvasNodeDragController,
    last_engine_action: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WorkspaceActionResult {
    pub(crate) handled_node_add: Option<String>,
    pub(crate) close_overlay: bool,
}

impl WorkspaceController {
    pub(crate) fn new() -> Self {
        Self {
            engine: Engine::new(None),
            image_demo: ImageDemoController::default(),
            canvas_node_drag: CanvasNodeDragController::default(),
            last_engine_action: "Ready".to_string(),
        }
    }

    pub(crate) fn handle_gui_action(&mut self, action: GuiAction) -> WorkspaceActionResult {
        match action {
            GuiAction::AddNode { type_id } => self.add_node_from_library(&type_id),
            GuiAction::OpenOverlay { .. }
            | GuiAction::CloseOverlay { .. }
            | GuiAction::WidgetClicked { .. } => WorkspaceActionResult::default(),
        }
    }

    pub(crate) fn add_node_from_library(&mut self, type_id: &str) -> WorkspaceActionResult {
        match self.engine.add_node(type_id) {
            Ok(node_id) => {
                self.last_engine_action = format!("Added node {type_id} as {:?}", node_id);
                WorkspaceActionResult {
                    handled_node_add: Some(type_id.to_string()),
                    close_overlay: true,
                }
            }
            Err(error) => {
                self.last_engine_action = format!("Add node failed: {error}");
                WorkspaceActionResult {
                    handled_node_add: Some(type_id.to_string()),
                    close_overlay: false,
                }
            }
        }
    }

    pub(crate) fn handle_image_demo_button(&mut self, id: &str) -> bool {
        let Some(outcome) = self.image_demo.handle_button(id, &mut self.engine) else {
            return false;
        };
        self.last_engine_action = outcome.to_string();
        true
    }

    pub(crate) fn engine_panel_state(&self) -> EnginePanelState {
        engine_adapter::engine_panel_state(&self.engine, &self.last_engine_action)
    }

    pub(crate) fn node_palette_state(&self) -> NodePaletteState {
        engine_adapter::node_palette_state(&self.engine)
    }

    pub(crate) fn canvas_node_views(&self, gui: &mut Context) -> Vec<CanvasNodeView> {
        let mut identities = engine_adapter::canvas_node_identities(&self.engine);
        identities.push(showcase_node::showcase_node_identity());
        identities.push(showcase_node::solo_node_identity());
        let layouts = gui.sync_canvas_node_layouts(&identities);
        let showcase_layouts = layouts
            .iter()
            .filter(|layout| showcase_node::is_showcase_node(&layout.owner_id))
            .cloned()
            .collect::<Vec<_>>();
        let engine_layouts = layouts
            .into_iter()
            .filter(|layout| !showcase_node::is_showcase_node(&layout.owner_id))
            .collect::<Vec<_>>();
        let mut views = engine_adapter::canvas_node_views(&self.engine, engine_layouts);
        views.extend(
            showcase_layouts
                .into_iter()
                .filter_map(showcase_node::showcase_view_for_layout),
        );
        let pending_connection = gui.pending_canvas_connection();
        let hovered_port_id = gui.hovered_canvas_port_id();
        for view in &mut views {
            view.input_group = gui.canvas_port_group_view(&view.owner_id, CanvasPortSide::Input);
            view.output_group = gui.canvas_port_group_view(&view.owner_id, CanvasPortSide::Output);
            view.selected = gui.is_canvas_node_selected(&view.owner_id);
            if let Some(pending) = pending_connection.as_ref() {
                self.apply_pending_connection_state(
                    pending,
                    hovered_port_id.as_deref(),
                    &mut view.inputs,
                );
                self.apply_pending_connection_state(
                    pending,
                    hovered_port_id.as_deref(),
                    &mut view.outputs,
                );
            }
        }
        views
    }

    pub(crate) fn canvas_connection_views(&self) -> Vec<CanvasConnectionView> {
        engine_adapter::canvas_connection_views(&self.engine)
    }

    pub(crate) fn start_canvas_node_drag(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        self.canvas_node_drag.start(gui, camera, id, x, y)
    }

    pub(crate) fn drag_canvas_node(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        self.canvas_node_drag.drag(gui, camera, id, x, y)
    }

    pub(crate) fn end_canvas_node_drag(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        self.canvas_node_drag.end(gui, camera, id, x, y)
    }

    pub(crate) fn toggle_canvas_port_group(&mut self, gui: &mut Context, id: &str) -> bool {
        let Some((owner_id, side)) = parse_canvas_port_group_trigger_id(id) else {
            return false;
        };
        gui.toggle_canvas_port_group(owner_id, side);
        true
    }

    pub(crate) fn select_canvas_node(&mut self, gui: &mut Context, id: &str) -> bool {
        let Some(owner_id) = canvas_node_owner_id(id) else {
            return false;
        };
        gui.select_canvas_node(owner_id)
    }

    pub(crate) fn clear_canvas_selection(&mut self, gui: &mut Context) {
        gui.clear_canvas_selection();
    }

    pub(crate) fn begin_canvas_port_connection(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(port_id) = canvas_port_event_target_id(id) else {
            return false;
        };
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        gui.begin_pending_canvas_connection(port_id, [canvas_x, canvas_y])
    }

    pub(crate) fn update_canvas_port_connection(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        x: f32,
        y: f32,
    ) -> bool {
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        gui.update_pending_canvas_connection([canvas_x, canvas_y])
    }

    pub(crate) fn end_canvas_port_connection(&mut self, gui: &mut Context, x: f32, y: f32) -> bool {
        let Some(pending) = gui.end_pending_canvas_connection() else {
            return false;
        };
        let Some(from) = parse_canvas_port_id(&pending.from_port_id) else {
            self.last_engine_action = "Connection cancelled: invalid output port".to_string();
            return true;
        };
        let Some(to) = self.input_port_at(gui, x, y) else {
            self.last_engine_action = "Connection cancelled".to_string();
            return true;
        };
        match self.connect_canvas_ports(from, to) {
            Ok(()) => {
                self.last_engine_action = "Connected canvas ports".to_string();
            }
            Err(error) => {
                self.last_engine_action = format!("Connect failed: {error}");
            }
        }
        true
    }

    pub(crate) fn cancel_canvas_port_connection(&mut self, gui: &mut Context) -> bool {
        if !gui.cancel_pending_canvas_connection() {
            return false;
        }
        self.last_engine_action = "Connection cancelled".to_string();
        true
    }

    pub(crate) fn update_canvas_hover(&mut self, gui: &mut Context, x: f32, y: f32) -> bool {
        let port_id = self.port_id_at(gui, x, y);
        gui.set_hovered_canvas_port(port_id.as_deref())
    }

    fn input_port_at(&self, gui: &Context, x: f32, y: f32) -> Option<CanvasPortRef> {
        self.port_id_at(gui, x, y).and_then(|port_id| {
            let port = parse_canvas_port_id(&port_id)?;
            (port.side == CanvasPortSide::Input).then_some(port)
        })
    }

    fn port_id_at(&self, gui: &Context, x: f32, y: f32) -> Option<String> {
        let chain = gui.hit_test(x, y);
        let port_id = chain.iter().find_map(|node_id| {
            let id = gui.node_name(node_id)?;
            canvas_port_event_target_id(id).map(str::to_string)
        });
        port_id
    }

    fn connect_canvas_ports(
        &mut self,
        from: CanvasPortRef,
        to: CanvasPortRef,
    ) -> Result<(), engine::facade::EngineError> {
        if from.side != CanvasPortSide::Output || to.side != CanvasPortSide::Input {
            return Err(engine::facade::EngineError::Graph {
                message: "canvas connections must run from output to input".to_string(),
            });
        }
        let Some(from_node) = engine_adapter::parse_engine_node_owner_id(&from.owner_id) else {
            return Err(engine::facade::EngineError::Graph {
                message: format!("invalid source node '{}'", from.owner_id),
            });
        };
        let Some(to_node) = engine_adapter::parse_engine_node_owner_id(&to.owner_id) else {
            return Err(engine::facade::EngineError::Graph {
                message: format!("invalid target node '{}'", to.owner_id),
            });
        };
        self.engine.connect(Connection {
            from: PinRef {
                node: from_node,
                interface: from.name,
            },
            to: PinRef {
                node: to_node,
                interface: to.name,
            },
        })
    }

    fn apply_pending_connection_state(
        &self,
        pending: &CanvasPendingConnectionView,
        hovered_port_id: Option<&str>,
        ports: &mut [CanvasPortView],
    ) {
        let Some(from) = parse_canvas_port_id(&pending.from_port_id) else {
            return;
        };
        for port in ports {
            port.connection_state = self.pending_connection_state(&from, hovered_port_id, port);
        }
    }

    fn pending_connection_state(
        &self,
        from: &CanvasPortRef,
        hovered_port_id: Option<&str>,
        port: &CanvasPortView,
    ) -> CanvasPortConnectionState {
        if port.stable_id == canvas_port_stable_id(&from.owner_id, from.side, &from.name) {
            return CanvasPortConnectionState::Source;
        }
        if port.side != CanvasPortSide::Input {
            return CanvasPortConnectionState::Idle;
        }
        let target = CanvasPortRef {
            owner_id: port
                .stable_id
                .strip_prefix("canvas_node::")
                .and_then(|id| id.split_once("::port::").map(|(owner_id, _)| owner_id))
                .unwrap_or_default()
                .to_string(),
            side: port.side,
            name: port.name.clone(),
        };
        let hovered = hovered_port_id == Some(port.stable_id.as_str());
        if self.can_connect_canvas_ports(from, &target).is_ok() {
            if hovered {
                CanvasPortConnectionState::DropTarget
            } else {
                CanvasPortConnectionState::CompatibleTarget
            }
        } else if hovered {
            CanvasPortConnectionState::RejectedDropTarget
        } else {
            CanvasPortConnectionState::IncompatibleTarget
        }
    }

    fn can_connect_canvas_ports(
        &self,
        from: &CanvasPortRef,
        to: &CanvasPortRef,
    ) -> Result<(), String> {
        if from.side != CanvasPortSide::Output || to.side != CanvasPortSide::Input {
            return Err("canvas connections must run from output to input".to_string());
        }
        let from_node = engine_adapter::parse_engine_node_owner_id(&from.owner_id)
            .ok_or_else(|| format!("invalid source node '{}'", from.owner_id))?;
        let to_node = engine_adapter::parse_engine_node_owner_id(&to.owner_id)
            .ok_or_else(|| format!("invalid target node '{}'", to.owner_id))?;
        let graph = self.engine.query_graph_snapshot();
        validate_connection_basic(
            &graph,
            &Connection {
                from: PinRef {
                    node: from_node,
                    interface: from.name.clone(),
                },
                to: PinRef {
                    node: to_node,
                    interface: to.name.clone(),
                },
            },
        )
        .map_err(|error| error.to_string())?;
        let from_type = self
            .canvas_port_data_type(from)
            .ok_or_else(|| format!("invalid source port '{}'", from.name))?;
        let to_type = self
            .canvas_port_data_type(to)
            .ok_or_else(|| format!("invalid target port '{}'", to.name))?;
        if from_type == to_type {
            Ok(())
        } else {
            Err(format!(
                "type mismatch: '{}' -> '{}'",
                from_type.0, to_type.0
            ))
        }
    }

    fn canvas_port_data_type(&self, port: &CanvasPortRef) -> Option<types::DataType> {
        let node_id = engine_adapter::parse_engine_node_owner_id(&port.owner_id)?;
        let graph = self.engine.query_graph_snapshot();
        let node = graph.nodes.get(&node_id)?;
        let def = self
            .engine
            .list_node_defs()
            .into_iter()
            .find(|def| def.type_id == node.type_id)?;
        match port.side {
            CanvasPortSide::Input => def
                .inputs
                .iter()
                .find(|pin| pin.name == port.name)
                .map(|pin| pin.data_type.clone())
                .or_else(|| {
                    def.params
                        .iter()
                        .find(|param| {
                            param.name == port.name && param.expose.contains(&ParamExpose::Input)
                        })
                        .map(|param| param.data_type.clone())
                }),
            CanvasPortSide::Output => def
                .outputs
                .iter()
                .find(|pin| pin.name == port.name)
                .map(|pin| pin.data_type.clone())
                .or_else(|| {
                    def.params
                        .iter()
                        .find(|param| {
                            param.name == port.name && param.expose.contains(&ParamExpose::Output)
                        })
                        .map(|param| param.data_type.clone())
                }),
        }
    }

    pub(crate) fn note_node_library_opened(&mut self) {
        self.last_engine_action = "Node library opened".to_string();
    }

    #[allow(dead_code)]
    pub(crate) fn export_project_layout(&self, gui: &Context, camera: &Camera) -> ProjectLayout {
        project_layout::export_project_layout(gui, camera)
    }

    #[allow(dead_code)]
    pub(crate) fn import_project_layout(
        &mut self,
        gui: &mut Context,
        camera: &mut Camera,
        layout: ProjectLayout,
    ) {
        project_layout::import_project_layout(gui, camera, layout);
    }
}

impl Default for WorkspaceController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_node_action_updates_engine_state_and_requests_overlay_close() {
        let mut controller = WorkspaceController::new();

        let result = controller.handle_gui_action(GuiAction::AddNode {
            type_id: "image_gen".to_string(),
        });
        let panel = controller.engine_panel_state();

        assert_eq!(result.handled_node_add, Some("image_gen".to_string()));
        assert!(result.close_overlay);
        assert_eq!(panel.node_count, 1);
        assert!(panel.last_action.contains("Added node image_gen"));
    }

    #[test]
    fn port_group_trigger_toggles_gui_tree_state() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();

        assert!(controller.toggle_canvas_port_group(
            &mut gui,
            "canvas_node::engine_node::1::port_group::input::trigger",
        ));
        assert!(
            gui.canvas_port_group_view("engine_node::1", CanvasPortSide::Input)
                .open
        );
        assert!(!controller.toggle_canvas_port_group(&mut gui, "slider"));
    }

    #[test]
    fn canvas_node_views_include_gui_interaction_state() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();

        controller.add_node_from_library("image_gen");
        let views = controller.canvas_node_views(&mut gui);
        let owner_id = views[0].owner_id.clone();
        assert!(!views[0].selected);

        assert!(controller.select_canvas_node(&mut gui, &format!("canvas_node::{owner_id}")));
        assert!(controller.toggle_canvas_port_group(
            &mut gui,
            &format!("canvas_node::{owner_id}::port_group::output::trigger"),
        ));

        let views = controller.canvas_node_views(&mut gui);
        assert!(views[0].selected);
        assert!(views[0].output_group.open);

        controller.clear_canvas_selection(&mut gui);
        let views = controller.canvas_node_views(&mut gui);
        assert!(!views[0].selected);
    }

    #[test]
    fn canvas_node_views_include_showcase_node() {
        let controller = WorkspaceController::new();
        let mut gui = Context::new();

        let views = controller.canvas_node_views(&mut gui);

        assert!(views
            .iter()
            .any(|view| view.owner_id == showcase_node::SHOWCASE_OWNER_ID));
        assert!(views
            .iter()
            .any(|view| view.owner_id == showcase_node::SOLO_OWNER_ID));
    }

    #[test]
    fn canvas_port_connection_runtime_is_reached_through_controller_api() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let camera = Camera::new();

        assert!(controller.begin_canvas_port_connection(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1::port::output::image::pin_item",
            10.0,
            20.0,
        ));
        assert_eq!(
            gui.pending_canvas_connection()
                .map(|pending| pending.from_port_id),
            Some("canvas_node::engine_node::1::port::output::image".to_string())
        );

        assert!(controller.update_canvas_port_connection(&mut gui, &camera, 30.0, 40.0));
        assert_eq!(
            gui.pending_canvas_connection()
                .map(|pending| pending.cursor_canvas),
            Some([30.0, 40.0])
        );
        assert!(controller.end_canvas_port_connection(&mut gui, 30.0, 40.0));
        assert!(gui.pending_canvas_connection().is_none());
    }

    #[test]
    fn canvas_port_refs_connect_engine_graph() {
        let mut controller = WorkspaceController::new();

        controller.add_node_from_library("image_gen");
        controller.add_node_from_library("color_adjust");

        controller
            .connect_canvas_ports(
                CanvasPortRef {
                    owner_id: "engine_node::0".to_string(),
                    side: CanvasPortSide::Output,
                    name: "image".to_string(),
                },
                CanvasPortRef {
                    owner_id: "engine_node::1".to_string(),
                    side: CanvasPortSide::Input,
                    name: "image".to_string(),
                },
            )
            .unwrap();

        let connections = controller.canvas_connection_views();
        assert_eq!(connections.len(), 1);
        assert_eq!(
            connections[0].from_port_id,
            "canvas_node::engine_node::0::port::output::image"
        );
        assert_eq!(
            connections[0].to_port_id,
            "canvas_node::engine_node::1::port::input::image"
        );
    }

    #[test]
    fn canvas_node_views_mark_pending_connection_targets() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();

        controller.add_node_from_library("image_gen");
        controller.add_node_from_library("color_adjust");
        assert!(gui.begin_pending_canvas_connection(
            "canvas_node::engine_node::0::port::output::image",
            [0.0, 0.0],
        ));

        let views = controller.canvas_node_views(&mut gui);
        let source = views
            .iter()
            .find(|view| view.owner_id == "engine_node::0")
            .and_then(|view| view.outputs.iter().find(|port| port.name == "image"))
            .map(|port| port.connection_state);
        let target = views
            .iter()
            .find(|view| view.owner_id == "engine_node::1")
            .and_then(|view| view.inputs.iter().find(|port| port.name == "image"))
            .map(|port| port.connection_state);

        assert_eq!(source, Some(CanvasPortConnectionState::Source));
        assert_eq!(target, Some(CanvasPortConnectionState::CompatibleTarget));
    }

    #[test]
    fn canvas_node_views_mark_hovered_pending_connection_drop_target() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();

        controller.add_node_from_library("image_gen");
        controller.add_node_from_library("color_adjust");
        assert!(gui.begin_pending_canvas_connection(
            "canvas_node::engine_node::0::port::output::image",
            [0.0, 0.0],
        ));
        assert!(
            gui.set_hovered_canvas_port(Some("canvas_node::engine_node::1::port::input::image"))
        );

        let views = controller.canvas_node_views(&mut gui);
        let target = views
            .iter()
            .find(|view| view.owner_id == "engine_node::1")
            .and_then(|view| view.inputs.iter().find(|port| port.name == "image"))
            .map(|port| port.connection_state);

        assert_eq!(target, Some(CanvasPortConnectionState::DropTarget));
    }

    #[test]
    fn cancel_canvas_port_connection_clears_pending_and_hover_state() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();

        assert!(gui.begin_pending_canvas_connection(
            "canvas_node::engine_node::0::port::output::image",
            [0.0, 0.0],
        ));
        assert!(
            gui.set_hovered_canvas_port(Some("canvas_node::engine_node::1::port::input::image"))
        );

        assert!(controller.cancel_canvas_port_connection(&mut gui));
        assert!(gui.pending_canvas_connection().is_none());
        assert!(gui.hovered_canvas_port_id().is_none());
        assert!(!controller.cancel_canvas_port_connection(&mut gui));
        assert_eq!(
            controller.engine_panel_state().last_action,
            "Connection cancelled"
        );
    }
}
