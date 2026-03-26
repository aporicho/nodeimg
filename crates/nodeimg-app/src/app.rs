use eframe::egui;
use egui_snarl::Snarl;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Instant;

use crate::gpu::gpu_context_from_eframe;
use crate::node::serial::Serializer;
use crate::node::viewer::NodeViewer;
use nodeimg_engine::transport::BackendClient;
use nodeimg_engine::transport::NodeTypeDef;
use nodeimg_types::node_instance::NodeInstance;
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::ProcessingTransport;
use crate::theme::dark::DarkTheme;
use crate::theme::light::LightTheme;
use crate::theme::Theme;
use crate::ui::node_canvas::NodeCanvas;
use crate::ui::preview_panel::PreviewPanel;

/// Default local backend settings.
const LOCAL_BACKEND_HOST: &str = "127.0.0.1";
const LOCAL_BACKEND_PORT: u16 = 8188;

/// Auto-save file name stored next to the executable / CWD.
const AUTOSAVE_FILE: &str = "autosave.nis";

pub struct App {
    snarl: Snarl<NodeInstance>,
    viewer: NodeViewer,
    preview_panel: PreviewPanel,
    node_canvas: NodeCanvas,
    theme: Arc<dyn Theme>,
    use_dark: bool,
    /// Child process for locally spawned Python backend.
    /// None if using a remote backend or if spawn failed.
    backend_process: Option<Child>,
    /// Current project file path (set by Save As / Open).
    current_file: Option<PathBuf>,
    /// Snapshot of the last saved JSON — used to detect changes for auto-save.
    last_saved_json: Option<String>,
    /// Dirty flag — set when graph changes, cleared after save.
    /// Prevents auto_save from serializing the entire graph every frame.
    dirty: bool,
    /// Whether the FPS overlay is visible (toggle with F12).
    show_fps: bool,
    /// Recent frame timestamps for FPS calculation.
    frame_times: VecDeque<Instant>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gpu_ctx = gpu_context_from_eframe(cc);
        if gpu_ctx.is_some() {
            eprintln!("[gpu] GPU context initialized successfully");
        } else {
            eprintln!("[gpu] GPU not available, using CPU fallback");
        }

        // Create transport before viewer — transport owns the canonical registries
        let transport = Arc::new(LocalTransport::new(gpu_ctx.clone(), None));

        let theme: Arc<dyn Theme> = Arc::new(LightTheme);
        let mut viewer = NodeViewer::new(Arc::clone(&theme), gpu_ctx, Arc::clone(&transport));

        // NIS_BACKEND_URL overrides the default local backend address.
        // Use this to point at a remote/cloud GPU server, e.g.:
        //   NIS_BACKEND_URL=http://my-gpu-box:8188 cargo run --release
        let backend_url = std::env::var("NIS_BACKEND_URL")
            .unwrap_or_else(|_| format!("http://{}:{}", LOCAL_BACKEND_HOST, LOCAL_BACKEND_PORT));
        let is_remote = std::env::var("NIS_BACKEND_URL").is_ok();
        eprintln!("[backend] Using backend URL: {}", backend_url);
        let backend = BackendClient::new(&backend_url);

        // Try to connect to an already-running backend first
        let mut backend_process = None;
        let mut connected = backend.health_check().is_ok();

        if !connected && !is_remote {
            // No backend running — try to auto-launch the local Python server
            // (skip auto-launch for remote backends — can't spawn on a remote host)
            eprintln!(
                "[backend] No backend at {}, attempting to auto-start...",
                backend_url
            );
            match Self::spawn_backend(LOCAL_BACKEND_HOST, LOCAL_BACKEND_PORT) {
                Ok(child) => {
                    backend_process = Some(child);
                    // Wait for the backend to become ready (up to 15 seconds)
                    connected = Self::wait_for_backend(&backend, 15);
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

        // Register AI nodes if connected
        if connected {
            match transport.register_remote_nodes(&backend) {
                Ok(count) => eprintln!("[backend] Registered {} AI node types", count),
                Err(e) => eprintln!("[backend] Failed to register AI nodes: {}", e),
            }
            viewer.node_type_defs = transport
                .node_types()
                .unwrap_or_default()
                .into_iter()
                .map(|d| (d.type_id.clone(), d))
                .collect();
        }
        // Don't set viewer.backend — backend is inside transport now

        // Auto-load: try to restore from autosave file
        let snarl = Self::try_auto_load(&transport, &viewer.node_type_defs);

        Self {
            snarl,
            viewer,
            preview_panel: PreviewPanel::new(),
            node_canvas: NodeCanvas::new(),
            theme,
            use_dark: false,
            backend_process,
            current_file: None,
            last_saved_json: None,
            dirty: true, // save once on first frame
            show_fps: false,
            frame_times: VecDeque::with_capacity(120),
        }
    }

    /// Spawn the Python backend as a child process.
    /// Automatically creates venv and installs dependencies if needed.
    fn spawn_backend(host: &str, port: u16) -> Result<Child, String> {
        let python_dir = std::env::current_dir()
            .map(|d| d.join("python"))
            .map_err(|e| format!("Cannot get CWD: {}", e))?;

        if !python_dir.join("server.py").exists() {
            return Err(format!(
                "python/server.py not found in {}",
                python_dir.display()
            ));
        }

        // Resolve the venv python path
        let venv_dir = python_dir.join(".venv");
        let venv_python = Self::venv_python_path(&venv_dir);

        // Auto-setup: create venv + install deps if needed
        if !venv_python.exists() {
            eprintln!("[backend] Python venv not found, setting up automatically...");
            Self::setup_python_env(&python_dir, &venv_dir)?;
        }

        // Use the venv python to start uvicorn
        Command::new(&venv_python)
            .args([
                "-m",
                "uvicorn",
                "server:app",
                "--host",
                host,
                "--port",
                &port.to_string(),
            ])
            .current_dir(&python_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn backend: {}", e))
    }

    /// Get the python executable path inside a venv (platform-aware).
    fn venv_python_path(venv_dir: &std::path::Path) -> std::path::PathBuf {
        if cfg!(windows) {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        }
    }

    /// Find the system python command (python3 or python).
    fn find_system_python() -> Result<String, String> {
        for cmd in &["python3", "python"] {
            if Command::new(cmd).arg("--version").output().is_ok() {
                return Ok(cmd.to_string());
            }
        }
        Err("Python not found. Please install Python 3.8+".to_string())
    }

    /// Create venv and install requirements automatically.
    fn setup_python_env(
        python_dir: &std::path::Path,
        venv_dir: &std::path::Path,
    ) -> Result<(), String> {
        let system_python = Self::find_system_python()?;

        // Create venv
        eprintln!("[backend] Creating virtual environment...");
        let status = Command::new(&system_python)
            .args(["-m", "venv", &venv_dir.to_string_lossy()])
            .status()
            .map_err(|e| format!("Failed to create venv: {}", e))?;
        if !status.success() {
            return Err("Failed to create Python virtual environment".to_string());
        }

        // Install requirements
        let venv_python = Self::venv_python_path(venv_dir);
        let requirements = python_dir.join("requirements.txt");
        if requirements.exists() {
            eprintln!("[backend] Installing Python dependencies (this may take a few minutes on first run)...");
            let status = Command::new(&venv_python)
                .args([
                    "-m",
                    "pip",
                    "install",
                    "-q",
                    "-r",
                    &requirements.to_string_lossy(),
                ])
                .current_dir(python_dir)
                .stderr(std::process::Stdio::inherit())
                .status()
                .map_err(|e| format!("Failed to install requirements: {}", e))?;
            if !status.success() {
                return Err("pip install failed. Check your Python/pip installation.".to_string());
            }
            eprintln!("[backend] Dependencies installed successfully");
        }

        Ok(())
    }

    /// Poll /health until the backend responds or timeout (in seconds).
    fn wait_for_backend(backend: &BackendClient, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        while start.elapsed() < timeout {
            if backend.health_check().is_ok() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        false
    }

    /// Try to load the autosave file on startup.
    fn try_auto_load(transport: &LocalTransport, type_defs: &HashMap<String, NodeTypeDef>) -> Snarl<NodeInstance> {
        let path = Self::autosave_path();
        if path.exists() {
            if let Ok(json) = std::fs::read_to_string(&path) {
                match transport.load_graph(&json) {
                    Ok(graph) => {
                        let snarl = Serializer::restore(&graph, type_defs);
                        eprintln!("[project] Restored autosave ({} nodes)", graph.nodes.len());
                        return snarl;
                    }
                    Err(e) => {
                        eprintln!("[project] Failed to load autosave: {}", e);
                    }
                }
            }
        }
        Snarl::new()
    }

    /// Auto-save the current graph if dirty.
    fn auto_save(&mut self) {
        if !self.dirty {
            return;
        }

        let graph = Serializer::snapshot(&self.snarl, &self.viewer.node_type_defs);
        let json = match Serializer::save(&graph) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[project] Auto-save serialize error: {}", e);
                return;
            }
        };

        // Skip write if serialized content is identical
        if self.last_saved_json.as_deref() == Some(&json) {
            self.dirty = false;
            return;
        }

        let path = Self::autosave_path();
        if let Err(e) = std::fs::write(&path, &json) {
            eprintln!("[project] Auto-save write error: {}", e);
        } else {
            self.last_saved_json = Some(json);
            self.dirty = false;
        }
    }

    fn autosave_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(AUTOSAVE_FILE)
    }

    /// Save As: pick a file and write the graph.
    fn save_as(&mut self) {
        let graph = Serializer::snapshot(&self.snarl, &self.viewer.node_type_defs);
        let json = match Serializer::save(&graph) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[project] Save error: {}", e);
                return;
            }
        };

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save Project")
            .add_filter("Node Image Studio", &["nis"])
            .save_file()
        {
            if let Err(e) = std::fs::write(&path, &json) {
                eprintln!("[project] Save write error: {}", e);
            } else {
                eprintln!("[project] Saved to {}", path.display());
                self.current_file = Some(path);
                self.last_saved_json = Some(json);
            }
        }
    }

    /// Open: pick a file and load the graph.
    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open Project")
            .add_filter("Node Image Studio", &["nis"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(json) => {
                    let load_result = self.viewer.transport.load_graph(&json);
                    match load_result {
                        Ok(graph) => {
                            self.snarl = Serializer::restore(&graph, &self.viewer.node_type_defs);
                            self.viewer.invalidate_all();
                            self.current_file = Some(path.clone());
                            self.last_saved_json = Some(json);
                            eprintln!("[project] Opened {}", path.display());
                        }
                        Err(e) => {
                            eprintln!("[project] Failed to load {}: {}", path.display(), e);
                        }
                    }
                },
                Err(e) => {
                    eprintln!("[project] Failed to read {}: {}", path.display(), e);
                }
            }
        }
    }

    /// Quick save: write to current_file or autosave path.
    fn quick_save(&mut self) {
        let graph = Serializer::snapshot(&self.snarl, &self.viewer.node_type_defs);
        let json = match Serializer::save(&graph) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[project] Save error: {}", e);
                return;
            }
        };
        let path = self
            .current_file
            .clone()
            .unwrap_or_else(Self::autosave_path);
        if let Err(e) = std::fs::write(&path, &json) {
            eprintln!("[project] Save error: {}", e);
        } else {
            eprintln!("[project] Saved to {}", path.display());
            self.last_saved_json = Some(json);
        }
    }

    /// Record the current frame timestamp and return the smoothed FPS.
    fn update_fps(&mut self) -> f64 {
        let now = Instant::now();
        self.frame_times.push_back(now);
        // Keep at most 60 samples for a ~1 second rolling window
        while self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let elapsed = now.duration_since(self.frame_times[0]).as_secs_f64();
        if elapsed > 0.0 {
            (self.frame_times.len() - 1) as f64 / elapsed
        } else {
            0.0
        }
    }

    fn toggle_theme(&mut self) {
        self.use_dark = !self.use_dark;
        self.theme = if self.use_dark {
            Arc::new(DarkTheme)
        } else {
            Arc::new(LightTheme)
        };
        self.viewer.theme = Arc::clone(&self.theme);
        self.viewer.invalidate_all();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Auto-save before exit
        self.auto_save();

        // Kill the auto-started Python backend when the app exits
        if let Some(ref mut child) = self.backend_process {
            eprintln!("[backend] Shutting down local Python backend...");
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts (Cmd on mac, Ctrl elsewhere)
        let shortcuts = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::T) && i.modifiers.command,                     // toggle theme
                i.key_pressed(egui::Key::S) && i.modifiers.command && i.modifiers.shift, // save as
                i.key_pressed(egui::Key::S) && i.modifiers.command && !i.modifiers.shift, // quick save
                i.key_pressed(egui::Key::O) && i.modifiers.command,                      // open
                i.key_pressed(egui::Key::F12),                                           // toggle FPS
            )
        });
        if shortcuts.0 {
            self.toggle_theme();
        }
        if shortcuts.1 {
            self.save_as();
        } else if shortcuts.2 {
            self.quick_save();
        }
        if shortcuts.3 {
            self.open_file();
        }
        if shortcuts.4 {
            self.show_fps = !self.show_fps;
        }

        self.theme.apply(ctx);

        // FPS overlay (toggle with F12)
        let fps = self.update_fps();
        if self.show_fps {
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
            .show(ctx, &*self.theme, self.viewer.preview_texture.as_ref());
        self.node_canvas.show(
            ctx,
            &*self.theme,
            &mut self.snarl,
            &mut self.viewer,
            &mut self.preview_panel,
        );

        // Propagate dirty flag from viewer (set on param change, connect, etc.)
        if self.viewer.graph_dirty {
            self.dirty = true;
            self.viewer.graph_dirty = false;
        }

        // Auto-save (only runs when dirty flag is set)
        self.auto_save();
    }
}
