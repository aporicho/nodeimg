use eframe::egui;
use egui_snarl::Snarl;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::gpu::gpu_context_from_eframe;
use crate::node::viewer::NodeViewer;
use nodeimg_engine::transport::BackendClient;
use nodeimg_types::node_instance::NodeInstance;
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::ProcessingTransport;
use crate::theme::Theme;
use crate::theme::light::LightTheme;
use crate::ui::node_canvas::NodeCanvas;
use crate::ui::preview_panel::PreviewPanel;

pub struct App {
    snarl: Snarl<NodeInstance>,
    viewer: NodeViewer,
    preview_panel: PreviewPanel,
    node_canvas: NodeCanvas,
    state: AppState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gpu_ctx = gpu_context_from_eframe(cc);
        if gpu_ctx.is_some() {
            eprintln!("[gpu] GPU context initialized successfully");
        } else {
            eprintln!("[gpu] GPU not available, using CPU fallback");
        }

        let transport = Arc::new(LocalTransport::new(gpu_ctx.clone(), None));

        let theme: Arc<dyn Theme> = Arc::new(LightTheme);
        let mut viewer = NodeViewer::new(Arc::clone(&theme), gpu_ctx, Arc::clone(&transport));
        let mut state = AppState::new(Arc::clone(&theme));

        // Backend connection
        let (backend_url, is_remote) = AppState::backend_url();
        eprintln!("[backend] Using backend URL: {}", backend_url);
        let backend = BackendClient::new(&backend_url);

        let mut connected = backend.health_check().is_ok();

        if !connected && !is_remote {
            eprintln!(
                "[backend] No backend at {}, attempting to auto-start...",
                backend_url
            );
            match AppState::spawn_backend("127.0.0.1", 8188) {
                Ok(child) => {
                    state.backend_process = Some(child);
                    connected = AppState::wait_for_backend(&backend, 15);
                    if connected {
                        eprintln!("[backend] Local Python backend started successfully");
                    } else {
                        eprintln!(
                            "[backend] Backend process started but not responding on {}",
                            backend_url
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[backend] Failed to auto-start backend: {}", e);
                    eprintln!("[backend] Make sure Python is installed with: cd python && pip install -r requirements.txt");
                }
            }
        }

        if connected {
            match transport.register_remote_nodes(&backend) {
                Ok(count) => eprintln!("[backend] Registered {} AI node types", count),
                Err(e) => eprintln!("[backend] Failed to register AI nodes: {}", e),
            }
            viewer.graph.node_type_defs = transport
                .node_types()
                .unwrap_or_default()
                .into_iter()
                .map(|d| (d.type_id.clone(), d))
                .collect();
        }

        let snarl = AppState::try_auto_load(&transport, &viewer.graph.node_type_defs);

        Self {
            snarl,
            viewer,
            preview_panel: PreviewPanel::new(),
            node_canvas: NodeCanvas::new(),
            state,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.state
            .auto_save(&self.snarl, &self.viewer.graph.node_type_defs);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        let shortcuts = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::T) && i.modifiers.command,
                i.key_pressed(egui::Key::S) && i.modifiers.command && i.modifiers.shift,
                i.key_pressed(egui::Key::S) && i.modifiers.command && !i.modifiers.shift,
                i.key_pressed(egui::Key::O) && i.modifiers.command,
                i.key_pressed(egui::Key::F12),
            )
        });
        if shortcuts.0 {
            self.state.toggle_theme();
            self.viewer.theme = Arc::clone(&self.state.theme);
            self.viewer.invalidate_all();
        }
        if shortcuts.1 {
            self.state
                .save_as(&self.snarl, &self.viewer.graph.node_type_defs);
        } else if shortcuts.2 {
            self.state
                .quick_save(&self.snarl, &self.viewer.graph.node_type_defs);
        }
        if shortcuts.3 {
            let loaded = self.state.open_file(
                &mut self.snarl,
                &self.viewer.graph.transport,
                &self.viewer.graph.node_type_defs,
            );
            if loaded {
                self.viewer.invalidate_all();
            }
        }
        if shortcuts.4 {
            self.state.show_fps = !self.state.show_fps;
        }

        self.state.theme.apply(ctx);

        // FPS overlay
        let fps = self.state.update_fps();
        if self.state.show_fps {
            egui::Area::new(egui::Id::new("fps_overlay"))
                .fixed_pos(egui::pos2(8.0, 8.0))
                .order(egui::Order::Foreground)
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_black_alpha(160))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("FPS: {:.0}", fps))
                                    .color(egui::Color32::from_rgb(0, 255, 128))
                                    .monospace()
                                    .size(13.0),
                            );
                        });
                });
            ctx.request_repaint();
        }

        self.preview_panel
            .show(ctx, &*self.state.theme, self.viewer.preview_texture.as_ref());
        self.node_canvas.show(
            ctx,
            &*self.state.theme,
            &mut self.snarl,
            &mut self.viewer,
            &mut self.preview_panel,
        );

        if self.viewer.graph.graph_dirty {
            self.state.dirty = true;
            self.viewer.graph.graph_dirty = false;
        }

        self.state
            .auto_save(&self.snarl, &self.viewer.graph.node_type_defs);
    }
}
