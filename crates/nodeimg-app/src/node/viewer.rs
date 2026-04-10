use crate::gpu::GpuContext;
use crate::node::graph_controller::GraphController;
use crate::node::widget::WidgetRegistry;
use nodeimg_types::node_instance::NodeInstance;
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::{ExecuteProgress, ProcessingTransport};
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use nodeimg_types::widget_id::WidgetId;
use crate::theme::Theme;
use eframe::egui;
use egui::{Color32, Frame, Ui};
use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPin, NodeId, OutPin, Snarl,
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct NodeViewer {
    pub graph: GraphController,
    pub widget_registry: WidgetRegistry,
    pub textures: HashMap<NodeId, egui::TextureHandle>,
    pub preview_texture: Option<egui::TextureHandle>,
    pub theme: Arc<dyn Theme>,
}

impl NodeViewer {
    pub fn new(theme: Arc<dyn Theme>, gpu_ctx: Option<Arc<GpuContext>>, transport: Arc<LocalTransport>) -> Self {
        Self {
            graph: GraphController::new(gpu_ctx, transport),
            widget_registry: WidgetRegistry::with_builtins(),
            textures: HashMap::new(),
            preview_texture: None,
            theme,
        }
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        self.graph.invalidate(node_id);
        self.textures.remove(&node_id);
    }

    pub fn invalidate_all(&mut self) {
        self.graph.invalidate_all();
        self.textures.clear();
    }
}

#[allow(refining_impl_trait)]
impl SnarlViewer<NodeInstance> for NodeViewer {
    fn title(&mut self, node: &NodeInstance) -> String {
        self.graph
            .find_type_def(&node.type_id)
            .map(|def| def.title.clone())
            .unwrap_or_else(|| format!("[Unknown: {}]", node.type_id))
    }

    fn inputs(&mut self, node: &NodeInstance) -> usize {
        self.graph
            .find_type_def(&node.type_id)
            .map(|def| def.inputs.len())
            .unwrap_or(0)
    }

    fn outputs(&mut self, node: &NodeInstance) -> usize {
        self.graph
            .find_type_def(&node.type_id)
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
        let title = self.title(&snarl[node_id]);
        ui.colored_label(self.theme.node_header_text(), title);
    }

    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) -> PinInfo {
        let instance = &snarl[pin.id.node];
        if let Some(def) = self.graph.find_type_def(&instance.type_id) {
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
        if let Some(def) = self.graph.find_type_def(&instance.type_id) {
            if let Some(pin_def) = def.outputs.get(pin.id.output) {
                let color = self.theme.pin_color(&pin_def.data_type);
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
        self.graph
            .find_type_def(&node.type_id)
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
        let type_def = match self.graph.find_type_def(&instance.type_id) {
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

                    let disabled = false;
                    let constraint =
                        GraphController::constraint_info_to_engine(&param_def.constraint);
                    let constraint_type = constraint.constraint_type();
                    let data_type = DataTypeId::new(&param_def.data_type);
                    let widget_override =
                        param_def.widget_override.as_ref().map(|w| WidgetId::new(w));

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
                self.graph.execution_manager.cancel_all();
                self.graph.ai_errors.remove(&node_id);
                self.graph.graph_dirty = true;
            }

            // Render preview area
            if has_preview {
                // Step 1: Poll background execution results
                let poll_results = self.graph.execution_manager.poll();
                for (trigger, progress) in poll_results {
                    match progress {
                        ExecuteProgress::NodeCompleted { node_id: nid, .. } => {
                            self.textures.remove(&NodeId(nid));
                        }
                        ExecuteProgress::Error { message, .. } => {
                            self.graph.ai_errors.insert(NodeId(trigger), message);
                        }
                        ExecuteProgress::Finished => {
                            self.textures.remove(&NodeId(trigger));
                        }
                    }
                }

                // Step 2: Evaluate local nodes only
                let request = self.graph.build_graph_request(node_id, snarl);

                if let Err(e) = self.graph.transport.evaluate_local_sync(&request) {
                    self.graph.backend_status = Some(e);
                } else {
                    self.graph.backend_status = None;
                }

                // Step 3: Check for pending AI work
                if !self.graph.execution_manager.is_running(node_id.0) {
                    if let Some((ai_node_id, _graph_json)) =
                        self.graph.transport.pending_ai_execution(&request)
                    {
                        eprintln!(
                            "[backend] Spawning async AI execution: trigger={} ai_node={}",
                            node_id.0, ai_node_id
                        );
                        self.graph.ai_errors.remove(&node_id);
                        let ctx = ui.ctx().clone();
                        self.graph
                            .execution_manager
                            .set_repaint_callback(move || ctx.request_repaint());
                        self.graph.execution_manager.submit(node_id.0, request);
                    }
                }

                // Step 4: UI status display
                if self.graph.execution_manager.is_running(node_id.0) {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Generating...");
                    });
                }

                if let Some(ref status) = self.graph.backend_status {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), status);
                }

                if let Some(err) = self.graph.ai_errors.get(&node_id) {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
                }

                // Step 5: Render preview if cached image exists
                if let Some(cached) = self.graph.transport.get_cached(node_id.0) {
                    if let Some(Value::Image(img)) = cached.get("image") {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                        self.preview_texture = Some(ui.ctx().load_texture(
                            "preview-panel",
                            color_image.clone(),
                            egui::TextureOptions::LINEAR,
                        ));

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
                } else if !self.graph.execution_manager.is_running(node_id.0)
                    && !self.graph.ai_errors.contains_key(&node_id)
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
        let cat_id = self
            .graph
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
        let categories = self.graph.transport.generate_menu();
        ui.label("Add Node");
        ui.separator();
        for cat in &categories {
            ui.menu_button(&cat.name, |ui: &mut Ui| {
                for item in &cat.items {
                    if ui.button(&item.title).clicked() {
                        if let Some(instance) = self.graph.transport.instantiate(&item.type_id) {
                            snarl.insert_node(pos, instance);
                            self.graph.graph_dirty = true;
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
            self.graph.graph_dirty = true;
            ui.close();
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeInstance>) {
        if !self.graph.validate_connection(from, to, snarl) {
            return;
        }

        // Disconnect existing connections on this input pin
        for &remote in &to.remotes {
            snarl.disconnect(remote, to.id);
        }

        snarl.connect(from.id, to.id);
        self.invalidate(to.id.node);
        self.graph.graph_dirty = true;
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<NodeInstance>) {
        snarl.disconnect(from.id, to.id);
        self.invalidate(to.id.node);
        self.graph.graph_dirty = true;
    }
}
