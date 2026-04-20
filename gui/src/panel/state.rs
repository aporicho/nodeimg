use crate::renderer::Rect;
use crate::widget::resize_edge::ResizeEdge;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PanelId(Cow<'static, str>);

impl PanelId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn into_cow(self) -> Cow<'static, str> {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct PanelConfig {
    pub id: PanelId,
    pub title: Cow<'static, str>,
    pub default_rect: Rect,
    pub min_size: [f32; 2],
    pub titlebar_visible: bool,
    pub draggable: bool,
    pub resizable: bool,
    pub closable: bool,
    pub initially_visible: bool,
}

#[derive(Clone, Debug)]
pub struct PanelRuntime {
    pub rect: Rect,
    pub min_size: [f32; 2],
    pub visible: bool,
    pub z_index: i32,
    pub collapsed: bool,
}

impl PanelRuntime {
    fn from_config(config: &PanelConfig, z_index: i32) -> Self {
        Self {
            rect: config.default_rect,
            min_size: config.min_size,
            visible: config.initially_visible,
            z_index,
            collapsed: false,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PanelPointerSession {
    pub(crate) id: String,
    pub(crate) last_x: f32,
    pub(crate) last_y: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct PanelResizeSession {
    pub(crate) id: String,
    pub(crate) edge: ResizeEdge,
    pub(crate) last_x: f32,
    pub(crate) last_y: f32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PanelRootRuntime {
    pub(crate) focused: Option<String>,
    pub(crate) next_z: i32,
    pub(crate) active_drag: Option<PanelPointerSession>,
    pub(crate) active_resize: Option<PanelResizeSession>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PanelRuntimeStore {
    panels: HashMap<String, PanelRuntime>,
    root: PanelRootRuntime,
}

impl PanelRuntimeStore {
    pub(crate) fn ensure_panel(&mut self, config: &PanelConfig) {
        let id = config.id.as_str().to_string();
        if let Some(panel) = self.panels.get_mut(&id) {
            panel.min_size = config.min_size;
            panel.rect.w = panel.rect.w.max(config.min_size[0]);
            panel.rect.h = panel.rect.h.max(config.min_size[1]);
            return;
        }
        let z_index = self.root.next_z;
        self.root.next_z += 1;
        self.panels
            .insert(id, PanelRuntime::from_config(config, z_index));
    }

    pub(crate) fn state(&self, id: &str) -> Option<&PanelRuntime> {
        self.panels.get(id)
    }

    pub(crate) fn state_mut(&mut self, id: &str) -> Option<&mut PanelRuntime> {
        self.panels.get_mut(id)
    }

    pub(crate) fn show(&mut self, id: &str) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.visible = true;
        }
    }

    pub(crate) fn hide(&mut self, id: &str) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.visible = false;
        }
    }

    pub(crate) fn toggle(&mut self, id: &str) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.visible = !panel.visible;
        }
    }

    pub(crate) fn bring_to_front(&mut self, id: &str) {
        let Some(panel) = self.panels.get_mut(id) else {
            return;
        };
        panel.z_index = self.root.next_z;
        self.root.next_z += 1;
        self.root.focused = Some(id.to_string());
    }

    pub(crate) fn move_by(&mut self, id: &str, dx: f32, dy: f32) {
        let Some(panel) = self.panels.get_mut(id) else {
            return;
        };
        panel.rect.x += dx;
        panel.rect.y += dy;
    }

    pub(crate) fn resize_by(&mut self, id: &str, edge: ResizeEdge, dx: f32, dy: f32) {
        let Some(panel) = self.panels.get_mut(id) else {
            return;
        };

        match edge {
            ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
                panel.rect.x += dx;
                panel.rect.w -= dx;
            }
            ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => {
                panel.rect.w += dx;
            }
            _ => {}
        }

        match edge {
            ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
                panel.rect.y += dy;
                panel.rect.h -= dy;
            }
            ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
                panel.rect.h += dy;
            }
            _ => {}
        }

        panel.rect.w = panel.rect.w.max(panel.min_size[0]);
        panel.rect.h = panel.rect.h.max(panel.min_size[1]);
    }

    pub(crate) fn clamp_min_size(&mut self, id: &str, min_size: [f32; 2]) {
        let Some(panel) = self.panels.get_mut(id) else {
            return;
        };
        if panel.rect.w < min_size[0] {
            panel.rect.w = min_size[0];
        }
        if panel.rect.h < min_size[1] {
            panel.rect.h = min_size[1];
        }
    }

    pub(crate) fn start_drag(&mut self, id: &str, x: f32, y: f32) {
        self.bring_to_front(id);
        self.root.active_drag = Some(PanelPointerSession {
            id: id.to_string(),
            last_x: x,
            last_y: y,
        });
    }

    pub(crate) fn drag_move(&mut self, id: &str, x: f32, y: f32) {
        let Some(session) = self.root.active_drag.as_mut() else {
            return;
        };
        if session.id != id {
            return;
        }
        let dx = x - session.last_x;
        let dy = y - session.last_y;
        session.last_x = x;
        session.last_y = y;
        self.move_by(id, dx, dy);
    }

    pub(crate) fn end_drag(&mut self) {
        self.root.active_drag = None;
    }

    pub(crate) fn start_resize(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        self.bring_to_front(id);
        self.root.active_resize = Some(PanelResizeSession {
            id: id.to_string(),
            edge,
            last_x: x,
            last_y: y,
        });
    }

    pub(crate) fn resize_move(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        let Some(session) = self.root.active_resize.as_mut() else {
            return;
        };
        if session.id != id || session.edge != edge {
            return;
        }
        let dx = x - session.last_x;
        let dy = y - session.last_y;
        session.last_x = x;
        session.last_y = y;
        self.resize_by(id, edge, dx, dy);
    }

    pub(crate) fn end_resize(&mut self) {
        self.root.active_resize = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(id: &'static str) -> PanelConfig {
        PanelConfig {
            id: PanelId::new(id),
            title: Cow::Borrowed("Panel"),
            default_rect: Rect {
                x: 10.0,
                y: 20.0,
                w: 240.0,
                h: 160.0,
            },
            min_size: [120.0, 80.0],
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: true,
        }
    }

    #[test]
    fn ensure_panel_initializes_runtime_state_once() {
        let mut store = PanelRuntimeStore::default();
        let first = config("tools");
        store.ensure_panel(&first);
        store.move_by("tools", 30.0, 40.0);

        let mut changed = config("tools");
        changed.default_rect.x = 1000.0;
        changed.min_size = [260.0, 200.0];
        store.ensure_panel(&changed);

        let state = store.state("tools").expect("panel state");
        assert_eq!(state.rect.x, 40.0);
        assert_eq!(state.rect.y, 60.0);
        assert_eq!(state.min_size, [260.0, 200.0]);
    }

    #[test]
    fn drag_session_moves_panel_and_focuses_it() {
        let mut store = PanelRuntimeStore::default();
        store.ensure_panel(&config("preview"));
        store.ensure_panel(&config("tools"));

        store.start_drag("preview", 100.0, 100.0);
        store.drag_move("preview", 118.0, 93.0);
        store.end_drag();

        let preview = store.state("preview").expect("preview state");
        let tools = store.state("tools").expect("tools state");
        assert_eq!(preview.rect.x, 28.0);
        assert_eq!(preview.rect.y, 13.0);
        assert!(preview.z_index > tools.z_index);
    }

    #[test]
    fn resize_session_clamps_to_min_size() {
        let mut store = PanelRuntimeStore::default();
        store.ensure_panel(&config("preview"));

        store.start_resize("preview", ResizeEdge::Right, 240.0, 160.0);
        store.resize_move("preview", ResizeEdge::Right, 0.0, 160.0);
        store.end_resize();

        let state = store.state("preview").expect("preview state");
        assert_eq!(state.rect.w, 120.0);
        assert_eq!(state.rect.h, 160.0);
    }
}
