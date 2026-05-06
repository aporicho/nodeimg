use super::canvas_drag::CanvasNodeDragController;
use super::canvas_resize::CanvasNodeResizeController;
use super::composition::{CanvasNodeComposition, WorkspaceUiComposition};
use super::engine_adapter;
use super::node_palette::NodePaletteState;
use super::project_layout;
use super::project_layout::ProjectLayout;
use super::showcase_node;
use super::showcase_state::ShowcaseState;
use super::ui_control_test_node;
use crate::image_demo::ImageDemoController;
use crate::panels::EnginePanelState;
use engine::facade::EngineFacade;
use engine::graph::validate::validate_connection_basic;
use engine::graph::{Connection, PinRef};
use engine::node_manager::ParamExpose;
use engine::Engine;
use gui::action::GuiAction;
use gui::canvas::camera::Camera;
use gui::canvas::node_template::CanvasNodeRenderView;
use gui::canvas::{
    canvas_node_event_owner_id, canvas_node_sizing_request, canvas_port_event_target_id,
    canvas_port_stable_id, parse_canvas_port_group_trigger_id, parse_canvas_port_id,
    CanvasConnectionView, CanvasNodeIdentity, CanvasNodeLayout, CanvasPortConnectionState,
    CanvasPortRef, CanvasPortSide,
};
use gui::context::{Context, HitChain};
use gui::control::ControlValue;
use gui::geometry::ResizeEdge;
use gui::theme::Theme;

pub(crate) struct WorkspaceController {
    engine: Engine,
    clean_room_engine: Engine,
    image_demo: ImageDemoController,
    canvas_node_drag: CanvasNodeDragController,
    canvas_node_resize: CanvasNodeResizeController,
    canvas_node_templates: engine_adapter::CanvasNodeTemplateCache,
    clean_room_canvas_node_templates: engine_adapter::CanvasNodeTemplateCache,
    showcase_state: ShowcaseState,
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
            clean_room_engine: ui_control_test_node::clean_room_engine(),
            image_demo: ImageDemoController::default(),
            canvas_node_drag: CanvasNodeDragController::default(),
            canvas_node_resize: CanvasNodeResizeController::default(),
            canvas_node_templates: engine_adapter::CanvasNodeTemplateCache::default(),
            clean_room_canvas_node_templates: engine_adapter::CanvasNodeTemplateCache::default(),
            showcase_state: ShowcaseState::default(),
            last_engine_action: "Ready".to_string(),
        }
    }

    pub(crate) fn handle_gui_action(&mut self, action: GuiAction) -> WorkspaceActionResult {
        match action {
            GuiAction::AddNode { type_id } => self.add_node_from_library(&type_id),
            GuiAction::OpenOverlay { .. }
            | GuiAction::CloseOverlay { .. }
            | GuiAction::ControlClicked { .. } => WorkspaceActionResult::default(),
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

    pub(crate) fn canvas_node_render_views(
        &mut self,
        gui: &mut Context,
        theme: &Theme,
        composition: WorkspaceUiComposition,
    ) -> Vec<CanvasNodeRenderView> {
        let identities = self.canvas_node_identities(composition);
        let mut layouts = gui.canvas_mut().sync_node_layouts(&identities);

        let mut views = self.canvas_node_render_views_for_layouts(layouts.clone(), composition);
        let mut resized_to_fit = false;
        let dirty_intrinsics = gui.controls_mut().take_dirty_intrinsics();
        let control_intrinsics = gui.controls().intrinsics();
        if !dirty_intrinsics.is_empty() || !control_intrinsics.is_empty() {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                intrinsic_count = control_intrinsics.len(),
                dirty_intrinsic_count = dirty_intrinsics.len(),
                view_count = views.len(),
                "collect control intrinsics before canvas node sizing"
            );
        }
        for view in &views {
            let owner_id = &view.state.owner_id;
            let request = canvas_node_sizing_request(
                &view.template,
                &view.state.layout,
                &control_intrinsics,
                theme,
            );
            let sizing_changed = gui.canvas_mut().apply_node_sizing(owner_id, request);
            let height_delta = request.target_height - view.state.layout.rect.h;
            if sizing_changed || height_delta.abs() > 0.5 {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    owner_id = %owner_id,
                    current_w = view.state.layout.rect.w,
                    current_h = view.state.layout.rect.h,
                    min_w = request.min_width,
                    min_h = request.min_height,
                    target_w = request.target_width,
                    target_h = request.target_height,
                    height_delta,
                    missing_auto_height_intrinsics = request.missing_auto_height_intrinsics,
                    sizing_changed,
                    "apply canvas node sizing request while building render views"
                );
            } else {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    owner_id = %owner_id,
                    min_w = request.min_width,
                    min_h = request.min_height,
                    target_w = request.target_width,
                    target_h = request.target_height,
                    missing_auto_height_intrinsics = request.missing_auto_height_intrinsics,
                    sizing_changed,
                    "apply canvas node sizing request while building render views"
                );
            }
            resized_to_fit = resized_to_fit || sizing_changed;
        }
        if resized_to_fit {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                "resync canvas node layouts after sizing request"
            );
            layouts = gui.canvas_mut().sync_node_layouts(&identities);
            views = self.canvas_node_render_views_for_layouts(layouts, composition);
        }
        self.decorate_canvas_node_render_views(gui, &mut views);
        views
    }

    fn canvas_node_render_views_for_layouts(
        &mut self,
        layouts: Vec<CanvasNodeLayout>,
        composition: WorkspaceUiComposition,
    ) -> Vec<CanvasNodeRenderView> {
        if composition.canvas_nodes() == CanvasNodeComposition::EngineCleanRoom {
            return engine_adapter::canvas_node_render_views(
                &self.clean_room_engine,
                layouts,
                &mut self.clean_room_canvas_node_templates,
            );
        }

        let showcase_layouts = layouts
            .iter()
            .filter(|layout| showcase_node::is_showcase_node(&layout.owner_id))
            .cloned()
            .collect::<Vec<_>>();
        let engine_layouts = layouts
            .into_iter()
            .filter(|layout| !showcase_node::is_showcase_node(&layout.owner_id))
            .collect::<Vec<_>>();
        let mut views = engine_adapter::canvas_node_render_views(
            &self.engine,
            engine_layouts,
            &mut self.canvas_node_templates,
        );
        views.extend(showcase_layouts.into_iter().filter_map(|layout| {
            showcase_node::showcase_render_view_for_layout(layout, &self.showcase_state)
        }));
        views
    }

    fn canvas_node_identities(
        &self,
        composition: WorkspaceUiComposition,
    ) -> Vec<CanvasNodeIdentity> {
        match composition.canvas_nodes() {
            CanvasNodeComposition::EngineCleanRoom => {
                engine_adapter::canvas_node_identities(&self.clean_room_engine)
            }
            CanvasNodeComposition::EngineAndShowcase => {
                let mut identities = engine_adapter::canvas_node_identities(&self.engine);
                identities.push(showcase_node::showcase_node_identity());
                identities.push(showcase_node::solo_node_identity());
                identities.push(showcase_node::text_area_node_identity());
                identities
            }
        }
    }

    fn decorate_canvas_node_render_views(&self, gui: &Context, views: &mut [CanvasNodeRenderView]) {
        let pending_connection = gui.canvas().pending_connection();
        let hovered_port_id = gui.canvas().hovered_port_id();
        for view in views {
            view.state.selected = gui.canvas().is_node_selected(&view.state.owner_id);
            view.state.input_group = gui
                .canvas()
                .port_group_view(&view.state.owner_id, CanvasPortSide::Input);
            view.state.output_group = gui
                .canvas()
                .port_group_view(&view.state.owner_id, CanvasPortSide::Output);
            if let Some(pending) = pending_connection.as_ref() {
                let Some(from) = parse_canvas_port_id(&pending.from_port_id) else {
                    continue;
                };
                for port_state in &mut view.state.port_states {
                    let port_id = canvas_port_stable_id(
                        &view.state.owner_id,
                        port_state.side,
                        &port_state.key,
                    );
                    let target = CanvasPortRef {
                        owner_id: view.state.owner_id.clone(),
                        side: port_state.side,
                        name: port_state.key.clone(),
                    };
                    port_state.connection_state = self.pending_connection_state(
                        &from,
                        hovered_port_id.as_deref(),
                        &port_id,
                        &target,
                    );
                }
            }
        }
    }

    pub(crate) fn canvas_connection_views(
        &self,
        composition: WorkspaceUiComposition,
    ) -> Vec<CanvasConnectionView> {
        if !composition.connections_enabled() {
            return Vec::new();
        }
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

    pub(crate) fn start_canvas_node_resize(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        id: &str,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    ) -> bool {
        self.canvas_node_resize.start(gui, camera, id, edge, x, y)
    }

    pub(crate) fn resize_canvas_node(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        id: &str,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    ) -> bool {
        self.canvas_node_resize.resize(gui, camera, id, edge, x, y)
    }

    pub(crate) fn end_canvas_node_resize(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        id: &str,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    ) -> bool {
        self.canvas_node_resize.end(gui, camera, id, edge, x, y)
    }

    pub(crate) fn update_control_value(
        &mut self,
        id: &str,
        value: ControlValue,
        composition: WorkspaceUiComposition,
    ) -> bool {
        match composition.canvas_nodes() {
            CanvasNodeComposition::EngineCleanRoom => {
                engine_adapter::update_engine_control_value(&mut self.clean_room_engine, id, value)
            }
            CanvasNodeComposition::EngineAndShowcase => {
                engine_adapter::update_engine_control_value(&mut self.engine, id, value.clone())
                    || self.showcase_state.update_control_value(id, value)
            }
        }
    }

    pub(crate) fn toggle_canvas_port_group(&mut self, gui: &mut Context, id: &str) -> bool {
        let Some((owner_id, side)) = parse_canvas_port_group_trigger_id(id) else {
            return false;
        };
        gui.canvas_mut().toggle_port_group(owner_id, side);
        true
    }

    pub(crate) fn select_canvas_node(&mut self, gui: &mut Context, id: &str) -> bool {
        let Some(owner_id) = canvas_node_event_owner_id(id) else {
            return false;
        };
        let selected = gui.canvas_mut().select_node(owner_id);
        let raised = gui.canvas_mut().bring_node_to_front(owner_id);
        selected || raised
    }

    pub(crate) fn clear_canvas_selection(&mut self, gui: &mut Context) -> bool {
        gui.canvas_mut().clear_selection()
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
        gui.canvas_mut()
            .begin_pending_connection(port_id, [canvas_x, canvas_y])
    }

    pub(crate) fn update_canvas_port_connection(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        x: f32,
        y: f32,
    ) -> bool {
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        gui.canvas_mut()
            .update_pending_connection([canvas_x, canvas_y])
    }

    pub(crate) fn end_canvas_port_connection(&mut self, gui: &mut Context, x: f32, y: f32) -> bool {
        let Some(pending) = gui.canvas_mut().end_pending_connection() else {
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
        if !gui.canvas_mut().cancel_pending_connection() {
            return false;
        }
        self.last_engine_action = "Connection cancelled".to_string();
        true
    }

    pub(crate) fn update_canvas_hover_from_chain(
        &mut self,
        gui: &mut Context,
        chain: &HitChain,
    ) -> bool {
        let port_id = self.port_id_at_chain(gui, chain);
        gui.canvas_mut().set_hovered_port(port_id.as_deref())
    }

    fn input_port_at(&self, gui: &Context, x: f32, y: f32) -> Option<CanvasPortRef> {
        self.port_id_at(gui, x, y).and_then(|port_id| {
            let port = parse_canvas_port_id(&port_id)?;
            (port.side == CanvasPortSide::Input).then_some(port)
        })
    }

    fn port_id_at(&self, gui: &Context, x: f32, y: f32) -> Option<String> {
        let query = gui.query();
        let chain = query.hit_test(x, y);
        self.port_id_at_chain(gui, &chain)
    }

    fn port_id_at_chain(&self, gui: &Context, chain: &HitChain) -> Option<String> {
        let query = gui.query();
        chain.iter().find_map(|node_id| {
            let id = query.node_name(node_id)?;
            canvas_port_event_target_id(id).map(str::to_string)
        })
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

    fn pending_connection_state(
        &self,
        from: &CanvasPortRef,
        hovered_port_id: Option<&str>,
        port_id: &str,
        target: &CanvasPortRef,
    ) -> CanvasPortConnectionState {
        if port_id == canvas_port_stable_id(&from.owner_id, from.side, &from.name) {
            return CanvasPortConnectionState::Source;
        }
        if target.side != CanvasPortSide::Input {
            return CanvasPortConnectionState::Idle;
        }
        let hovered = hovered_port_id == Some(port_id);
        if self.can_connect_canvas_ports(from, target).is_ok() {
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
    use crate::workspace::composition::WorkspaceUiComposition;
    use crate::workspace::ui_control_test_node::UI_CONTROL_TEST_TYPE_ID;
    use gui::control::ControlSpec;
    use gui::theme::light_theme;

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
            gui.canvas()
                .port_group_view("engine_node::1", CanvasPortSide::Input)
                .open
        );
        assert!(!controller.toggle_canvas_port_group(&mut gui, "slider"));
    }

    #[test]
    fn canvas_node_render_views_include_gui_interaction_state() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();

        controller.add_node_from_library("image_gen");
        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());
        let owner_id = views[0].state.owner_id.clone();
        assert!(!views[0].state.selected);

        assert!(controller.select_canvas_node(&mut gui, &format!("canvas_node::{owner_id}")));

        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());
        assert!(views[0].state.selected);

        controller.clear_canvas_selection(&mut gui);
        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());
        assert!(!views[0].state.selected);
    }

    #[test]
    fn canvas_node_render_views_include_showcase_node() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();

        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());

        assert!(views
            .iter()
            .any(|view| view.state.owner_id == showcase_node::SHOWCASE_OWNER_ID));
        assert!(views
            .iter()
            .any(|view| view.state.owner_id == showcase_node::SOLO_OWNER_ID));
        assert!(views
            .iter()
            .any(|view| view.state.owner_id == showcase_node::TEXT_AREA_OWNER_ID));
    }

    #[test]
    fn showcase_control_values_update_rendered_specs() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();

        assert!(controller.update_control_value(
            "canvas_node::showcase_node::all_controls::body::param::Sampler::control::content",
            ControlValue::Selection(2),
            WorkspaceUiComposition::full(),
        ));
        assert!(controller.update_control_value(
            "canvas_node::showcase_node::all_controls::body::param::Tint::control::content",
            ControlValue::Color([0.2, 0.4, 0.6, 1.0]),
            WorkspaceUiComposition::full(),
        ));
        assert!(controller.update_control_value(
            "canvas_node::showcase_node::all_controls::body::param::Output::control::content",
            ControlValue::FilePath("/tmp/out.png".to_string()),
            WorkspaceUiComposition::full(),
        ));

        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());
        let showcase = views
            .iter()
            .find(|view| view.state.owner_id == showcase_node::SHOWCASE_OWNER_ID)
            .expect("showcase view");

        assert!(showcase
            .template
            .params
            .iter()
            .any(|param| { matches!(&param.control, ControlSpec::Select { selected: 2, .. }) }));
        assert!(showcase.template.params.iter().any(|param| {
            matches!(
                &param.control,
                ControlSpec::Color { rgba }
                if (rgba[0] - 0.2).abs() < 0.0001
                    && (rgba[1] - 0.4).abs() < 0.0001
                    && (rgba[2] - 0.6).abs() < 0.0001
            )
        }));
        assert!(showcase.template.params.iter().any(|param| {
            matches!(
                &param.control,
                ControlSpec::FilePath { path, .. } if path == "/tmp/out.png"
            )
        }));
    }

    #[test]
    fn clean_room_canvas_nodes_include_only_dev_engine_test_node() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();

        controller.add_node_from_library("image_gen");
        let views = controller.canvas_node_render_views(
            &mut gui,
            &theme,
            WorkspaceUiComposition::clean_room(),
        );

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].state.owner_id, "engine_node::0");
        assert_eq!(views[0].template.type_id, UI_CONTROL_TEST_TYPE_ID);
        assert_eq!(views[0].template.params.len(), 8);
        assert!(!views
            .iter()
            .any(|view| view.state.owner_id == showcase_node::SHOWCASE_OWNER_ID));
    }

    #[test]
    fn clean_room_engine_control_values_update_rendered_specs() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();
        let composition = WorkspaceUiComposition::clean_room();

        assert!(controller.update_control_value(
            "canvas_node::engine_node::0::body::param::strength::control::content",
            ControlValue::Number(0.9),
            composition,
        ));
        assert!(controller.update_control_value(
            "canvas_node::engine_node::0::body::param::seed::control::content",
            ControlValue::Number(7.2),
            composition,
        ));
        assert!(controller.update_control_value(
            "canvas_node::engine_node::0::body::param::enabled::control::content",
            ControlValue::Bool(false),
            composition,
        ));
        assert!(controller.update_control_value(
            "canvas_node::engine_node::0::body::param::mode::control::content",
            ControlValue::Selection(2),
            composition,
        ));
        assert!(controller.update_control_value(
            "canvas_node::engine_node::0::body::param::tint::control::content",
            ControlValue::Color([0.2, 0.4, 0.6, 1.0]),
            composition,
        ));
        assert!(controller.update_control_value(
            "canvas_node::engine_node::0::body::param::output_path::control::content",
            ControlValue::FilePath("next.png".to_string()),
            composition,
        ));

        let views = controller.canvas_node_render_views(&mut gui, &theme, composition);
        let node = &views[0];

        assert!(node.template.params.iter().any(|param| {
            matches!(
                &param.control,
                ControlSpec::Slider { value, .. } if (*value - 0.9).abs() < 0.0001
            )
        }));
        assert!(node.template.params.iter().any(|param| {
            matches!(
                &param.control,
                ControlSpec::Number { value, .. } if (*value - 7.0).abs() < 0.0001
            )
        }));
        assert!(node
            .template
            .params
            .iter()
            .any(|param| { matches!(&param.control, ControlSpec::Toggle { checked: false }) }));
        assert!(node
            .template
            .params
            .iter()
            .any(|param| { matches!(&param.control, ControlSpec::Select { selected: 2, .. }) }));
        assert!(node.template.params.iter().any(|param| {
            matches!(
                &param.control,
                ControlSpec::Color { rgba }
                    if (rgba[0] - 0.2).abs() < 0.0001
                        && (rgba[1] - 0.4).abs() < 0.0001
                        && (rgba[2] - 0.6).abs() < 0.0001
            )
        }));
        assert!(node.template.params.iter().any(|param| {
            matches!(
                &param.control,
                ControlSpec::FilePath { path, .. } if path == "next.png"
            )
        }));
        assert!(!controller.update_control_value("other", ControlValue::Bool(false), composition));
    }

    #[test]
    fn canvas_node_render_views_preserve_text_area_height_before_intrinsic_sync() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();
        let identity = showcase_node::text_area_node_identity();

        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());
        let text_area_view = views
            .iter()
            .find(|view| view.state.owner_id == showcase_node::TEXT_AREA_OWNER_ID)
            .expect("text area showcase view");
        let runtime_layout = gui
            .canvas()
            .export_node_layouts()
            .into_iter()
            .find(|layout| layout.owner_id == showcase_node::TEXT_AREA_OWNER_ID)
            .expect("text area runtime layout");

        assert_eq!(text_area_view.state.layout.rect.h, identity.default_rect.h);
        assert_eq!(runtime_layout.rect.h, identity.default_rect.h);
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
            gui.canvas()
                .pending_connection()
                .map(|pending| pending.from_port_id),
            Some("canvas_node::engine_node::1::port::output::image".to_string())
        );

        assert!(controller.update_canvas_port_connection(&mut gui, &camera, 30.0, 40.0));
        assert_eq!(
            gui.canvas()
                .pending_connection()
                .map(|pending| pending.cursor_canvas),
            Some([30.0, 40.0])
        );
        assert!(controller.end_canvas_port_connection(&mut gui, 30.0, 40.0));
        assert!(gui.canvas().pending_connection().is_none());
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

        let connections = controller.canvas_connection_views(WorkspaceUiComposition::full());
        assert_eq!(connections.len(), 1);
        assert_eq!(
            connections[0].from_port_id,
            "canvas_node::engine_node::0::port::output::image"
        );
        assert_eq!(
            connections[0].to_port_id,
            "canvas_node::engine_node::1::port::input::image"
        );
        assert!(controller
            .canvas_connection_views(WorkspaceUiComposition::clean_room())
            .is_empty());
    }

    #[test]
    fn canvas_node_render_views_mark_pending_connection_targets() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();

        controller.add_node_from_library("image_gen");
        controller.add_node_from_library("color_adjust");
        assert!(gui.canvas_mut().begin_pending_connection(
            "canvas_node::engine_node::0::port::output::image",
            [0.0, 0.0],
        ));

        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());
        let source = views
            .iter()
            .find(|view| view.state.owner_id == "engine_node::0")
            .and_then(|view| port_state(view, CanvasPortSide::Output, "image"));
        let target = views
            .iter()
            .find(|view| view.state.owner_id == "engine_node::1")
            .and_then(|view| port_state(view, CanvasPortSide::Input, "image"));

        assert_eq!(source, Some(CanvasPortConnectionState::Source));
        assert_eq!(target, Some(CanvasPortConnectionState::CompatibleTarget));
    }

    #[test]
    fn canvas_node_render_views_mark_hovered_pending_connection_drop_target() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();
        let theme = light_theme();

        controller.add_node_from_library("image_gen");
        controller.add_node_from_library("color_adjust");
        assert!(gui.canvas_mut().begin_pending_connection(
            "canvas_node::engine_node::0::port::output::image",
            [0.0, 0.0],
        ));
        assert!(gui
            .canvas_mut()
            .set_hovered_port(Some("canvas_node::engine_node::1::port::input::image")));

        let views =
            controller.canvas_node_render_views(&mut gui, &theme, WorkspaceUiComposition::full());
        let target = views
            .iter()
            .find(|view| view.state.owner_id == "engine_node::1")
            .and_then(|view| port_state(view, CanvasPortSide::Input, "image"));

        assert_eq!(target, Some(CanvasPortConnectionState::DropTarget));
    }

    #[test]
    fn cancel_canvas_port_connection_clears_pending_and_hover_state() {
        let mut controller = WorkspaceController::new();
        let mut gui = Context::new();

        assert!(gui.canvas_mut().begin_pending_connection(
            "canvas_node::engine_node::0::port::output::image",
            [0.0, 0.0],
        ));
        assert!(gui
            .canvas_mut()
            .set_hovered_port(Some("canvas_node::engine_node::1::port::input::image")));

        assert!(controller.cancel_canvas_port_connection(&mut gui));
        assert!(gui.canvas().pending_connection().is_none());
        assert!(gui.canvas().hovered_port_id().is_none());
        assert!(!controller.cancel_canvas_port_connection(&mut gui));
        assert_eq!(
            controller.engine_panel_state().last_action,
            "Connection cancelled"
        );
    }

    fn port_state(
        view: &CanvasNodeRenderView,
        side: CanvasPortSide,
        key: &str,
    ) -> Option<CanvasPortConnectionState> {
        view.state
            .port_states
            .iter()
            .find(|port| port.side == side && port.key == key)
            .map(|port| port.connection_state)
    }
}
