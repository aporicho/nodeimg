use crate::execution::ExecutionManager;
use crate::gpu::GpuContext;
use crate::node::widget::WidgetRegistry;
use nodeimg_engine::NodeInstance;
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::{
    ConnectionRequest, ConstraintInfo, ExecuteProgress, GraphRequest, NodeRequest, NodeTypeDef,
    ParamValue, ProcessingTransport,
};
use nodeimg_types::constraint::Constraint;
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use nodeimg_types::widget_id::WidgetId;
use crate::theme::Theme;
use eframe::egui;
use egui::{Color32, Frame, Ui};
use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPin, InPinId, NodeId, OutPin, Snarl,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct NodeViewer {
    // Transport abstraction (new)
    pub transport: Arc<LocalTransport>,
    pub node_type_defs: Vec<NodeTypeDef>,
    pub execution_manager: ExecutionManager,

    pub widget_registry: WidgetRegistry,
    /// GPU textures per node. Managed here (not in Cache) because TextureHandle
    /// requires egui::Context which is only available during rendering.
    pub textures: HashMap<NodeId, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
    pub theme: Arc<dyn Theme>,
    /// GPU context for hardware-accelerated processing. None if GPU is unavailable.
    pub gpu_ctx: Option<Arc<GpuContext>>,
    /// Status message shown during AI execution (e.g. "Generating..." or error).
    pub backend_status: Option<String>,
    /// Per-node error messages from AI execution.
    ai_errors: HashMap<NodeId, String>,
    /// Set when the graph is mutated (param change, connect, disconnect, node add/remove).
    /// Cleared by the caller (App) after saving.
    pub graph_dirty: bool,
}

impl NodeViewer {
    pub fn new(theme: Arc<dyn Theme>, gpu_ctx: Option<Arc<GpuContext>>, transport: Arc<LocalTransport>) -> Self {
        let node_type_defs = transport.node_types().unwrap_or_default();
        let execution_manager = ExecutionManager::new(Arc::clone(&transport) as Arc<dyn ProcessingTransport>);

        Self {
            transport,
            node_type_defs,
            execution_manager,
            widget_registry: WidgetRegistry::with_builtins(),
            textures: HashMap::new(),
            preview_texture: None,
            theme,
            gpu_ctx,
            backend_status: None,
            ai_errors: HashMap::new(),
            graph_dirty: false,
        }
    }

    /// Find a NodeTypeDef by type_id from the cached list.
    fn find_type_def(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.node_type_defs.iter().find(|d| d.type_id == type_id)
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        self.transport.invalidate(node_id.0);
        self.textures.remove(&node_id);
    }

    pub fn invalidate_all(&mut self) {
        self.transport.invalidate_all();
        self.textures.clear();
    }

    fn build_graph_request(&self, target: NodeId, snarl: &Snarl<NodeInstance>) -> GraphRequest {
        let mut nodes = HashMap::new();
        for (nid, instance) in snarl.node_ids() {
            let params: HashMap<String, ParamValue> = instance
                .params
                .iter()
                .filter_map(|(k, v)| ParamValue::from_value(v).map(|pv| (k.clone(), pv)))
                .collect();
            nodes.insert(
                nid.0,
                NodeRequest {
                    type_id: instance.type_id.clone(),
                    params,
                },
            );
        }
        let connections = self.build_connection_requests(snarl);
        GraphRequest {
            nodes,
            connections,
            target_node: target.0,
        }
    }

    fn build_connection_requests(&self, snarl: &Snarl<NodeInstance>) -> Vec<ConnectionRequest> {
        let mut connections = Vec::new();
        for (node_id, instance) in snarl.node_ids() {
            let def = match self.find_type_def(&instance.type_id) {
                Some(d) => d,
                None => continue,
            };
            for (i, input_pin) in def.inputs.iter().enumerate() {
                let in_pin = snarl.in_pin(InPinId {
                    node: node_id,
                    input: i,
                });
                for remote in &in_pin.remotes {
                    let upstream_instance = &snarl[remote.node];
                    if let Some(upstream_def) = self.find_type_def(&upstream_instance.type_id) {
                        if let Some(out_pin) = upstream_def.outputs.get(remote.output) {
                            connections.push(ConnectionRequest {
                                from_node: remote.node.0,
                                from_pin: out_pin.name.clone(),
                                to_node: node_id.0,
                                to_pin: input_pin.name.clone(),
                            });
                        }
                    }
                }
            }
        }
        connections
    }

    fn constraint_info_to_engine(info: &Option<ConstraintInfo>) -> Constraint {
        match info {
            None => Constraint::None,
            Some(ConstraintInfo::Range { min, max }) => Constraint::Range {
                min: *min,
                max: *max,
            },
            Some(ConstraintInfo::Options(opts)) => Constraint::Enum {
                options: opts.clone(),
            },
            Some(ConstraintInfo::FilePath { filters }) => Constraint::FilePath {
                filters: filters.clone(),
            },
        }
    }
}

#[allow(refining_impl_trait)]
impl SnarlViewer<NodeInstance> for NodeViewer {
    fn title(&mut self, node: &NodeInstance) -> String {
        self.find_type_def(&node.type_id)
            .map(|def| def.title.clone())
            .unwrap_or_else(|| format!("[Unknown: {}]", node.type_id))
    }

    fn inputs(&mut self, node: &NodeInstance) -> usize {
        // V1: only data input pins. TODO: add parameter pins for V2.
        self.find_type_def(&node.type_id)
            .map(|def| def.inputs.len())
            .unwrap_or(0)
    }

    fn outputs(&mut self, node: &NodeInstance) -> usize {
        self.find_type_def(&node.type_id)
            .map(|def| def.outputs.len())
            .unwrap_or(0)
    }

    fn show_header(
        &mut self,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<NodeInstance>,
    ) {
        // header_frame() provides the colored background via its fill color.
        // We only need to render the title text here with the correct color.
        let title = self.title(&snarl[node_id]);
        ui.colored_label(self.theme.node_header_text(), title);
    }

    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) -> PinInfo {
        let instance = &snarl[pin.id.node];
        if let Some(def) = self.find_type_def(&instance.type_id) {
            if let Some(pin_def) = def.inputs.get(pin.id.input) {
                ui.colored_label(self.theme.text_secondary(), &pin_def.name);
                let color = self.theme.pin_color(&pin_def.data_type);
                return PinInfo::circle().with_fill(color);
            }
        }
        ui.colored_label(self.theme.text_secondary(), "?");
        PinInfo::circle().with_fill(Color32::GRAY)
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        snarl: &mut Snarl<NodeInstance>,
    ) -> PinInfo {
        let instance = &snarl[pin.id.node];
        if let Some(def) = self.find_type_def(&instance.type_id) {
            if let Some(pin_def) = def.outputs.get(pin.id.output) {
                let color = self.theme.pin_color(&pin_def.data_type);
                // Use egui's built-in right-aligned label instead of layout wrapper
                // to avoid vertical misalignment with the pin circle
                let label = egui::Label::new(
                    egui::RichText::new(&pin_def.name).color(self.theme.text_secondary()),
                )
                .halign(egui::Align::RIGHT);
                ui.add(label);
                return PinInfo::circle().with_fill(color);
            }
        }
        ui.colored_label(self.theme.text_secondary(), "?");
        PinInfo::circle().with_fill(Color32::GRAY)
    }

    fn has_body(&mut self, node: &NodeInstance) -> bool {
        self.find_type_def(&node.type_id)
            .map(|def| !def.params.is_empty() || def.has_preview)
            .unwrap_or(false)
    }

    fn show_body(
        &mut self,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<NodeInstance>,
    ) {
        let instance = snarl[node_id].clone();
        let type_def = match self.find_type_def(&instance.type_id) {
            Some(d) => d.clone(),
            None => {
                ui.label("Unknown node type");
                return;
            }
        };
        let params_def = type_def.params.clone();
        let has_preview = type_def.has_preview;

        ui.vertical(|ui: &mut Ui| {
            ui.set_max_width(crate::theme::tokens::NODE_MAX_WIDTH);
            let mut any_changed = false;

            // Render parameter widgets using two-column layout
            for param_def in &params_def {
                ui.horizontal(|ui| {
                    // Fixed-width label column
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            crate::theme::tokens::PARAM_LABEL_WIDTH,
                            ui.available_height(),
                        ),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.colored_label(self.theme.text_secondary(), &param_def.name);
                        },
                    );

                    let disabled = false; // TODO V2: check param pin connection
                    let constraint = Self::constraint_info_to_engine(&param_def.constraint);
                    let constraint_type = constraint.constraint_type();
                    let data_type = DataTypeId::new(&param_def.data_type);
                    let widget_override = param_def.widget_override.as_ref().map(|w| WidgetId::new(w));

                    // Clone value out, modify in place, write back if changed
                    let old_value = snarl[node_id].params.get(&param_def.name).cloned();
                    if let Some(mut value) = old_value {
                        let changed = self.widget_registry.render(
                            widget_override.as_ref(),
                            &data_type,
                            &constraint_type,
                            ui,
                            &mut value,
                            &constraint,
                            &param_def.name,
                            disabled,
                        );
                        if changed {
                            snarl[node_id].params.insert(param_def.name.clone(), value);
                            any_changed = true;
                        }
                    }
                });
            }

            if any_changed {
                self.invalidate(node_id);
                // Cancel ALL in-flight AI tasks — any param change may invalidate
                // upstream results, and we don't track which trigger depends on
                // which AI node.
                self.execution_manager.cancel_all();
                self.ai_errors.remove(&node_id);
                self.graph_dirty = true;
            }

            // Render preview area
            if has_preview {
                // Step 1: Poll background execution results (non-blocking).
                let poll_results = self.execution_manager.poll();
                for (trigger, progress) in poll_results {
                    match progress {
                        ExecuteProgress::NodeCompleted { node_id: nid, .. } => {
                            self.textures.remove(&NodeId(nid));
                        }
                        ExecuteProgress::Error { message, .. } => {
                            self.ai_errors.insert(NodeId(trigger), message);
                        }
                        ExecuteProgress::Finished => {
                            self.textures.remove(&NodeId(trigger));
                        }
                    }
                }

                // Step 2: Evaluate local nodes only (skips AI nodes, never blocks).
                let request = self.build_graph_request(node_id, snarl);

                if let Err(e) = self.transport.evaluate_local_sync(request.clone()) {
                    self.backend_status = Some(e);
                } else {
                    self.backend_status = None;
                }

                // Step 3: Check for pending AI work.
                if !self.execution_manager.is_running(node_id.0) {
                    if let Some((ai_node_id, _graph_json)) =
                        self.transport.pending_ai_execution(request.clone())
                    {
                        eprintln!(
                            "[backend] Spawning async AI execution: trigger={} ai_node={}",
                            node_id.0, ai_node_id
                        );
                        self.ai_errors.remove(&node_id);
                        self.execution_manager.set_repaint_ctx(ui.ctx().clone());
                        self.execution_manager.submit(node_id.0, request);
                    }
                }

                // Step 4: UI status display
                if self.execution_manager.is_running(node_id.0) {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Generating...");
                    });
                }

                if let Some(ref status) = self.backend_status {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), status);
                }

                if let Some(err) = self.ai_errors.get(&node_id) {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
                }

                // Step 5: Render preview if cached image exists
                if let Some(cached) = self.transport.get_cached(node_id.0) {
                    if let Some(Value::Image(img)) = cached.get("image") {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                        self.preview_texture = Some(ui.ctx().load_texture(
                            "preview-panel",
                            color_image.clone(),
                            egui::TextureOptions::LINEAR,
                        ));

                        // Show thumbnail in node body
                        self.textures.entry(node_id).or_insert_with(|| {
                            ui.ctx().load_texture(
                                format!("node-{:?}", node_id),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            )
                        });
                        if let Some(tex) = self.textures.get(&node_id) {
                            let tex_size = tex.size_vec2();
                            let scale = (200.0_f32 / tex_size.x.max(tex_size.y)).min(1.0);
                            ui.image(egui::load::SizedTexture::new(tex.id(), tex_size * scale));
                        }
                    }
                } else if !self.execution_manager.is_running(node_id.0)
                    && !self.ai_errors.contains_key(&node_id)
                {
                    ui.label("No input connected");
                }
            }
        });
    }

    fn header_frame(
        &mut self,
        _default: Frame,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        snarl: &Snarl<NodeInstance>,
    ) -> Frame {
        // Use category color for header accent
        let cat_id = self
            .find_type_def(&snarl[node_id].type_id)
            .map(|def| def.category.clone())
            .unwrap_or_else(|| "tool".to_string());
        self.theme.node_header_frame(&cat_id)
    }

    fn node_frame(
        &mut self,
        _default: Frame,
        _node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        _snarl: &Snarl<NodeInstance>,
    ) -> Frame {
        self.theme.node_body_frame()
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<NodeInstance>) -> bool {
        true
    }

    fn show_graph_menu(&mut self, pos: egui::Pos2, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) {
        let categories = self.transport.generate_menu();
        ui.label("Add Node");
        ui.separator();
        for cat in &categories {
            ui.menu_button(&cat.name, |ui: &mut Ui| {
                for item in &cat.items {
                    if ui.button(&item.title).clicked() {
                        if let Some(instance) = self.transport.instantiate(&item.type_id) {
                            snarl.insert_node(pos, instance);
                            self.graph_dirty = true;
                        }
                        ui.close();
                    }
                }
            });
        }
    }

    fn has_node_menu(&mut self, _node: &NodeInstance) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<NodeInstance>,
    ) {
        if ui.button("Delete").clicked() {
            snarl.remove_node(node_id);
            self.invalidate_all();
            self.graph_dirty = true;
            ui.close();
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeInstance>) {
        // Type compatibility check using transport
        let from_type_id = snarl[from.id.node].type_id.clone();
        let to_type_id = snarl[to.id.node].type_id.clone();
        let compatible = (|| {
            let from_def = self.find_type_def(&from_type_id)?;
            let to_def = self.find_type_def(&to_type_id)?;
            let from_type = &from_def.outputs.get(from.id.output)?.data_type;
            let to_type = &to_def.inputs.get(to.id.input)?.data_type;
            Some(self.transport.is_compatible(from_type, to_type))
        })();

        if compatible != Some(true) {
            return;
        }

        // Cycle detection
        let would_create_cycle = {
            let mut connections = self.build_connection_requests(snarl);
            let from_pin_name = self
                .find_type_def(&from_type_id)
                .and_then(|def| def.outputs.get(from.id.output).map(|p| p.name.clone()))
                .unwrap_or_default();
            let to_pin_name = self
                .find_type_def(&to_type_id)
                .and_then(|def| def.inputs.get(to.id.input).map(|p| p.name.clone()))
                .unwrap_or_default();
            connections.push(ConnectionRequest {
                from_node: from.id.node.0,
                from_pin: from_pin_name,
                to_node: to.id.node.0,
                to_pin: to_pin_name,
            });
            self.transport.would_create_cycle(to.id.node.0, &connections)
        };

        if would_create_cycle {
            return;
        }

        // Disconnect existing connections on this input pin
        for &remote in &to.remotes {
            snarl.disconnect(remote, to.id);
        }

        snarl.connect(from.id, to.id);
        self.invalidate(to.id.node);
        self.graph_dirty = true;
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeInstance>) {
        snarl.disconnect(from.id, to.id);
        self.invalidate(to.id.node);
        self.graph_dirty = true;
    }
}
