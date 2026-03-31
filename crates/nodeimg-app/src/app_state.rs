use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Instant;

use egui_snarl::Snarl;
use nodeimg_engine::transport::local::LocalTransport;
use nodeimg_engine::transport::{BackendClient, NodeTypeDef, ProcessingTransport};
use nodeimg_types::node_instance::NodeInstance;

use crate::node::serial::Serializer;
use crate::theme::dark::DarkTheme;
use crate::theme::light::LightTheme;
use crate::theme::Theme;

/// Default local backend settings.
const LOCAL_BACKEND_HOST: &str = "127.0.0.1";
const LOCAL_BACKEND_PORT: u16 = 8188;

/// Auto-save file name stored next to the executable / CWD.
const AUTOSAVE_FILE: &str = "autosave.nis";

pub struct AppState {
    /// Child process for locally spawned Python backend.
    pub backend_process: Option<Child>,
    /// Current project file path (set by Save As / Open).
    pub current_file: Option<PathBuf>,
    /// Snapshot of the last saved JSON — used to detect changes for auto-save.
    pub last_saved_json: Option<String>,
    /// Dirty flag — set when graph changes, cleared after save.
    pub dirty: bool,
    /// Whether the FPS overlay is visible (toggle with F12).
    pub show_fps: bool,
    /// Recent frame timestamps for FPS calculation.
    frame_times: VecDeque<Instant>,
    /// Current theme.
    pub theme: Arc<dyn Theme>,
    /// Whether dark theme is active.
    use_dark: bool,
}

impl AppState {
    pub fn new(theme: Arc<dyn Theme>) -> Self {
        Self {
            backend_process: None,
            current_file: None,
            last_saved_json: None,
            dirty: true,
            show_fps: false,
            frame_times: VecDeque::with_capacity(120),
            theme,
            use_dark: false,
        }
    }

    // ── Backend process management ──

    /// Spawn the Python backend as a child process.
    pub fn spawn_backend(host: &str, port: u16) -> Result<Child, String> {
        let python_dir = std::env::current_dir()
            .map(|d| d.join("python"))
            .map_err(|e| format!("Cannot get CWD: {}", e))?;

        if !python_dir.join("server.py").exists() {
            return Err(format!(
                "python/server.py not found in {}",
                python_dir.display()
            ));
        }

        let venv_dir = python_dir.join(".venv");
        let venv_python = Self::venv_python_path(&venv_dir);

        if !venv_python.exists() {
            eprintln!("[backend] Python venv not found, setting up automatically...");
            Self::setup_python_env(&python_dir, &venv_dir)?;
        }

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

    fn venv_python_path(venv_dir: &std::path::Path) -> std::path::PathBuf {
        if cfg!(windows) {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python")
        }
    }

    fn find_system_python() -> Result<String, String> {
        for cmd in &["python3", "python"] {
            if Command::new(cmd).arg("--version").output().is_ok() {
                return Ok(cmd.to_string());
            }
        }
        Err("Python not found. Please install Python 3.8+".to_string())
    }

    fn setup_python_env(
        python_dir: &std::path::Path,
        venv_dir: &std::path::Path,
    ) -> Result<(), String> {
        let system_python = Self::find_system_python()?;

        eprintln!("[backend] Creating virtual environment...");
        let status = Command::new(&system_python)
            .args(["-m", "venv", &venv_dir.to_string_lossy()])
            .status()
            .map_err(|e| format!("Failed to create venv: {}", e))?;
        if !status.success() {
            return Err("Failed to create Python virtual environment".to_string());
        }

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

    pub fn wait_for_backend(backend: &BackendClient, timeout_secs: u64) -> bool {
        let start = Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        while start.elapsed() < timeout {
            if backend.health_check().is_ok() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        false
    }

    pub fn backend_url() -> (String, bool) {
        let is_remote = std::env::var("NIS_BACKEND_URL").is_ok();
        let url = std::env::var("NIS_BACKEND_URL")
            .unwrap_or_else(|_| format!("http://{}:{}", LOCAL_BACKEND_HOST, LOCAL_BACKEND_PORT));
        (url, is_remote)
    }

    // ── Project file management ──

    pub fn auto_save(
        &mut self,
        snarl: &Snarl<NodeInstance>,
        type_defs: &HashMap<String, NodeTypeDef>,
    ) {
        if !self.dirty {
            return;
        }

        let graph = Serializer::snapshot(snarl, type_defs);
        let json = match Serializer::save(&graph) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("[project] Auto-save serialize error: {}", e);
                return;
            }
        };

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

    pub fn autosave_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(AUTOSAVE_FILE)
    }

    pub fn try_auto_load(
        transport: &LocalTransport,
        type_defs: &HashMap<String, NodeTypeDef>,
    ) -> Snarl<NodeInstance> {
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

    pub fn save_as(
        &mut self,
        snarl: &Snarl<NodeInstance>,
        type_defs: &HashMap<String, NodeTypeDef>,
    ) {
        let graph = Serializer::snapshot(snarl, type_defs);
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

    /// Open a project file. Returns true if a file was loaded (caller should invalidate caches).
    pub fn open_file(
        &mut self,
        snarl: &mut Snarl<NodeInstance>,
        transport: &LocalTransport,
        type_defs: &HashMap<String, NodeTypeDef>,
    ) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open Project")
            .add_filter("Node Image Studio", &["nis"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(json) => match transport.load_graph(&json) {
                    Ok(graph) => {
                        *snarl = Serializer::restore(&graph, type_defs);
                        self.current_file = Some(path.clone());
                        self.last_saved_json = Some(json);
                        eprintln!("[project] Opened {}", path.display());
                        return true;
                    }
                    Err(e) => {
                        eprintln!("[project] Failed to load {}: {}", path.display(), e);
                    }
                },
                Err(e) => {
                    eprintln!("[project] Failed to read {}: {}", path.display(), e);
                }
            }
        }
        false
    }

    pub fn quick_save(
        &mut self,
        snarl: &Snarl<NodeInstance>,
        type_defs: &HashMap<String, NodeTypeDef>,
    ) {
        let graph = Serializer::snapshot(snarl, type_defs);
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

    // ── FPS ──

    pub fn update_fps(&mut self) -> f64 {
        let now = Instant::now();
        self.frame_times.push_back(now);
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

    // ── Theme ──

    pub fn toggle_theme(&mut self) {
        self.use_dark = !self.use_dark;
        self.theme = if self.use_dark {
            Arc::new(DarkTheme)
        } else {
            Arc::new(LightTheme)
        };
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.backend_process {
            eprintln!("[backend] Shutting down local Python backend...");
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
