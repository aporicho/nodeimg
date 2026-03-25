use crate::gpu::GpuContext;
use crate::node::widget::WidgetRegistry;
use nodeimg_engine::ai_task::AiExecutor;
use nodeimg_engine::backend::BackendClient;
use nodeimg_engine::cache::Cache;
use nodeimg_engine::eval::{Connection, EvalEngine};
use nodeimg_engine::registry::{NodeInstance, NodeRegistry};
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::{NodeTypeDef, ProcessingTransport};
use nodeimg_types::category::CategoryRegistry;
use nodeimg_types::data_type::DataTypeRegistry;
use nodeimg_types::value::Value;
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

    // Kept for show_body compatibility (direct engine access)
    pub type_registry: DataTypeRegistry,
    pub category_registry: CategoryRegistry,
    pub node_registry: NodeRegistry,
    pub widget_registry: WidgetRegistry,
    pub cache: Cache,
    /// GPU textures per node. Managed here (not in Cache) because TextureHandle
    /// requires egui::Context which is only available during rendering.
    pub textures: HashMap<NodeId, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
    pub theme: Arc<dyn Theme>,
    /// GPU context for hardware-accelerated processing. None if GPU is unavailable.
    pub gpu_ctx: Option<Arc<GpuContext>>,
    /// Backend client for executing AI nodes. None if backend is unavailable.
    pub backend: Option<BackendClient>,
    /// Status message shown during AI execution (e.g. "Generating..." or error).
    pub backend_status: Option<String>,
    /// Async executor for background AI operations.
    pub ai_executor: AiExecutor,
    /// Per-node error messages from AI execution.
    ai_errors: HashMap<NodeId, String>,
    /// Set when the graph is mutated (param change, connect, disconnect, node add/remove).
    /// Cleared by the caller (App) after saving.
    pub graph_dirty: bool,
}

impl NodeViewer {
    pub fn new(theme: Arc<dyn Theme>, gpu_ctx: Option<Arc<GpuContext>>, transport: Arc<LocalTransport>) -> Self {
        let node_type_defs = transport.node_types().unwrap_or_default();

        // Still init local copies for show_body compatibility
        let mut node_registry = NodeRegistry::new();
        nodeimg_engine::builtins::register_all(&mut node_registry);

        Self {
            transport,
            node_type_defs,
            type_registry: DataTypeRegistry::with_builtins(),
            category_registry: CategoryRegistry::with_builtins(),
            node_registry,
            widget_registry: WidgetRegistry::with_builtins(),
            cache: Cache::new(),
            textures: HashMap::new(),
            preview_texture: None,
            theme,
            gpu_ctx,
            backend: None,
            backend_status: None,
            ai_executor: AiExecutor::new(),
            ai_errors: HashMap::new(),
            graph_dirty: false,
        }
    }

    /// Find a NodeTypeDef by type_id from the cached list.
    fn find_type_def(&self, type_id: &str) -> Option<&NodeTypeDef> {
        self.node_type_defs.iter().find(|d| d.type_id == type_id)
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        self.cache.invalidate(node_id.0);
        self.transport.invalidate(node_id.0);
        self.textures.remove(&node_id);
    }

    pub fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
        self.transport.invalidate_all();
        self.textures.clear();
    }

    /// Build Connection list from the Snarl graph (used by EvalEngine).
    pub fn build_connections(&self, snarl: &Snarl<NodeInstance>) -> Vec<Connection> {
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
                            connections.push(Connection {
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
        let def = match self.node_registry.get(&instance.type_id) {
            Some(d) => d,
            None => {
                ui.label("Unknown node type");
                return;
            }
        };
        let params_def: Vec<_> = def.params.clone();
        let has_preview = def.has_preview;

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
                    let constraint_type = param_def.constraint.constraint_type();

                    // Clone value out, modify in place, write back if changed
                    let old_value = snarl[node_id].params.get(&param_def.name).cloned();
                    if let Some(mut value) = old_value {
                        let changed = self.widget_registry.render(
                            param_def.widget_override.as_ref(),
                            &param_def.data_type,
                            &constraint_type,
                            ui,
                            &mut value,
                            &param_def.constraint,
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
                self.ai_executor.cancel_all();
                self.ai_errors.remove(&node_id);
                self.graph_dirty = true;
            }

            // Render preview area
            if has_preview {
                // Step 1: Poll background AI results (non-blocking).
                // Successful results invalidate downstream caches (including this
                // Preview node) so they re-evaluate with fresh AI output below.
                let (completed, errors) = self.ai_executor.poll_results(&mut self.cache);
                for trigger in &completed {
                    self.textures.remove(&NodeId(*trigger));
                }
                for (trigger, err) in errors {
                    self.ai_errors.insert(NodeId(trigger), err);
                }

                // Step 2: Evaluate local nodes only (backend=None, never blocks).
                // AI nodes are silently skipped — their results come from Step 1.
                let nodes_map: HashMap<usize, NodeInstance> = snarl
                    .node_ids()
                    .map(|(id, inst)| (id.0, inst.clone()))
                    .collect();
                let connections = self.build_connections(snarl);

                if let Err(e) = EvalEngine::evaluate(
                    node_id.0,
                    &nodes_map,
                    &connections,
                    &self.node_registry,
                    &self.type_registry,
                    &mut self.cache,
                    None, // no backend → skips AI nodes
                    self.gpu_ctx.as_ref(),
                ) {
                    self.backend_status = Some(e);
                } else {
                    self.backend_status = None;
                }

                // Step 3: Check for pending AI work.
                if !self.ai_executor.is_running(node_id.0) {
                    if let Some(client) = &self.backend {
                        if let Some((ai_node_id, graph_json)) =
                            EvalEngine::pending_ai_execution(
                                node_id.0,
                                &nodes_map,
                                &connections,
                                &self.node_registry,
                                &self.cache,
                            )
                        {
                            eprintln!(
                                "[backend] Spawning async AI execution: trigger={} ai_node={}",
                                node_id.0, ai_node_id
                            );
                            self.ai_errors.remove(&node_id);
                            let ctx = ui.ctx().clone();
                            self.ai_executor.spawn(
                                node_id.0,
                                ai_node_id,
                                graph_json,
                                client.clone(),
                                Box::new(move || ctx.request_repaint()),
                            );
                        }
                    }
                }

                // Step 4: UI status display
                if self.ai_executor.is_running(node_id.0) {
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
                if let Some(cached) = self.cache.get(node_id.0) {
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
                } else if !self.ai_executor.is_running(node_id.0)
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
            let mut connections = self.build_connections(snarl);
            let from_pin_name = self
                .find_type_def(&from_type_id)
                .and_then(|def| def.outputs.get(from.id.output).map(|p| p.name.clone()))
                .unwrap_or_default();
            let to_pin_name = self
                .find_type_def(&to_type_id)
                .and_then(|def| def.inputs.get(to.id.input).map(|p| p.name.clone()))
                .unwrap_or_default();
            connections.push(Connection {
                from_node: from.id.node.0,
                from_pin: from_pin_name,
                to_node: to.id.node.0,
                to_pin: to_pin_name,
            });
            EvalEngine::topo_sort(to.id.node.0, &connections).is_err()
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
