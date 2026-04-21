use crate::renderer::Rect;
use crate::tree::RuntimeSlot;
use crate::widget::resize_edge::ResizeEdge;
use std::borrow::Cow;

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
    pub(crate) fn from_config(config: &PanelConfig, z_index: i32) -> Self {
        Self {
            rect: config.default_rect,
            min_size: config.min_size,
            visible: config.initially_visible,
            z_index,
            collapsed: false,
        }
    }
}

impl Default for PanelRuntime {
    fn default() -> Self {
        Self {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            min_size: [0.0, 0.0],
            visible: false,
            z_index: 0,
            collapsed: false,
        }
    }
}

impl RuntimeSlot for PanelRuntime {}

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

impl RuntimeSlot for PanelRootRuntime {}

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
        let mut tree = crate::tree::Tree::new();
        let first = config("tools");
        tree.ensure_panel(&first);
        tree.move_panel_by("tools", 30.0, 40.0);

        let mut changed = config("tools");
        changed.default_rect.x = 1000.0;
        changed.min_size = [260.0, 200.0];
        tree.ensure_panel(&changed);

        let state = tree.panel_state("tools").expect("panel state");
        assert_eq!(state.rect.x, 40.0);
        assert_eq!(state.rect.y, 60.0);
        assert_eq!(state.min_size, [260.0, 200.0]);
    }

    #[test]
    fn drag_session_moves_panel_and_focuses_it() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));
        tree.ensure_panel(&config("tools"));

        tree.start_panel_drag("preview", 100.0, 100.0);
        tree.move_panel_drag("preview", 118.0, 93.0);
        tree.end_panel_drag();

        let preview = tree.panel_state("preview").expect("preview state");
        let tools = tree.panel_state("tools").expect("tools state");
        assert_eq!(preview.rect.x, 28.0);
        assert_eq!(preview.rect.y, 13.0);
        assert!(preview.z_index > tools.z_index);
    }

    #[test]
    fn resize_session_clamps_to_min_size() {
        let mut tree = crate::tree::Tree::new();
        tree.ensure_panel(&config("preview"));

        tree.start_panel_resize("preview", ResizeEdge::Right, 240.0, 160.0);
        tree.move_panel_resize("preview", ResizeEdge::Right, 0.0, 160.0);
        tree.end_panel_resize();

        let state = tree.panel_state("preview").expect("preview state");
        assert_eq!(state.rect.w, 120.0);
        assert_eq!(state.rect.h, 160.0);
    }
}
