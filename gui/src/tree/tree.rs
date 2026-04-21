use super::node::{NodeId, TreeNode};
use super::runtime_slots::RuntimeSlot;
use super::{RuntimeSlots, StableId};
use crate::panel::{
    PanelConfig, PanelPointerSession, PanelResizeSession, PanelRootRuntime, PanelRuntime,
};
use crate::widget::resize_edge::ResizeEdge;
use std::collections::HashMap;

const PANEL_ROOT_ID: &str = "panel_root";

/// 全局控件树存储。用 Vec<Option<>> 做 arena，索引访问。
pub struct Tree {
    nodes: Vec<Option<TreeNode>>,
    root: Option<NodeId>,
    free: Vec<NodeId>,
    retained_runtime: HashMap<String, RuntimeSlots>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            free: Vec::new(),
            retained_runtime: HashMap::new(),
        }
    }

    pub fn insert(&mut self, node: TreeNode) -> NodeId {
        if let Some(id) = self.free.pop() {
            self.nodes[id] = Some(node);
            id
        } else {
            let id = self.nodes.len();
            self.nodes.push(Some(node));
            id
        }
    }

    pub fn remove(&mut self, id: NodeId) {
        if id < self.nodes.len() {
            // 递归删除子节点
            if let Some(node) = self.nodes[id].take() {
                let children = node.children;
                for child_id in children {
                    self.remove(child_id);
                }
                if !node.runtime_slots.is_empty() {
                    self.retained_runtime
                        .insert(node.id.as_ref().to_string(), node.runtime_slots);
                }
            }
            self.free.push(id);
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&TreeNode> {
        self.nodes.get(id).and_then(|n| n.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut TreeNode> {
        self.nodes.get_mut(id).and_then(|n| n.as_mut())
    }

    pub fn node_by_stable_id(&self, stable_id: &StableId) -> Option<NodeId> {
        self.iter()
            .find_map(|(id, node)| (node.id.as_ref() == stable_id.as_str()).then_some(id))
    }

    pub fn node_by_str(&self, stable_id: &str) -> Option<NodeId> {
        self.iter()
            .find_map(|(id, node)| (node.id.as_ref() == stable_id).then_some(id))
    }

    pub fn runtime_slot<T: RuntimeSlot>(&self, id: NodeId) -> Option<&T> {
        self.get(id)?.runtime_slots.get::<T>()
    }

    pub fn runtime_slot_mut<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<&mut T> {
        self.get_mut(id)?.runtime_slots.get_mut::<T>()
    }

    pub fn ensure_runtime_slot<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<&mut T> {
        Some(self.get_mut(id)?.runtime_slots.ensure::<T>())
    }

    pub fn remove_runtime_slot<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<T> {
        self.get_mut(id)?.runtime_slots.remove::<T>()
    }

    pub(crate) fn take_retained_runtime_slots(&mut self, id: &str) -> Option<RuntimeSlots> {
        self.retained_runtime.remove(id)
    }

    pub(crate) fn runtime_slot_by_stable_id<T: RuntimeSlot>(&self, id: &str) -> Option<&T> {
        if let Some(node_id) = self.node_by_str(id) {
            return self.runtime_slot::<T>(node_id);
        }
        self.retained_runtime.get(id)?.get::<T>()
    }

    pub(crate) fn runtime_slot_by_stable_id_mut<T: RuntimeSlot>(
        &mut self,
        id: &str,
    ) -> Option<&mut T> {
        if let Some(node_id) = self.node_by_str(id) {
            return self.runtime_slot_mut::<T>(node_id);
        }
        self.retained_runtime.get_mut(id)?.get_mut::<T>()
    }

    pub(crate) fn ensure_runtime_slot_by_stable_id<T: RuntimeSlot>(&mut self, id: &str) -> &mut T {
        if let Some(node_id) = self.node_by_str(id) {
            return self
                .ensure_runtime_slot::<T>(node_id)
                .expect("node id came from this tree");
        }
        self.retained_runtime
            .entry(id.to_string())
            .or_default()
            .ensure::<T>()
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &TreeNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(id, node)| node.as_ref().map(|node| (id, node)))
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TreeNode> {
        self.nodes.iter_mut().filter_map(|n| n.as_mut())
    }

    pub fn ensure_panel(&mut self, config: &PanelConfig) {
        let id = config.id.as_str();
        if self.panel_state(id).is_some() {
            let panel = self.panel_state_mut(id).expect("panel state checked above");
            panel.min_size = config.min_size;
            panel.rect.w = panel.rect.w.max(config.min_size[0]);
            panel.rect.h = panel.rect.h.max(config.min_size[1]);
            return;
        }

        let z_index = {
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let z_index = root.next_z;
            root.next_z += 1;
            z_index
        };
        *self.ensure_runtime_slot_by_stable_id::<PanelRuntime>(id) =
            PanelRuntime::from_config(config, z_index);
    }

    pub fn panel_state(&self, id: &str) -> Option<&PanelRuntime> {
        self.runtime_slot_by_stable_id::<PanelRuntime>(id)
    }

    pub fn panel_state_mut(&mut self, id: &str) -> Option<&mut PanelRuntime> {
        self.runtime_slot_by_stable_id_mut::<PanelRuntime>(id)
    }

    pub fn move_panel_by(&mut self, id: &str, dx: f32, dy: f32) {
        let Some(panel) = self.panel_state_mut(id) else {
            return;
        };
        panel.rect.x += dx;
        panel.rect.y += dy;
    }

    pub fn resize_panel_by(&mut self, id: &str, edge: ResizeEdge, dx: f32, dy: f32) {
        let Some(panel) = self.panel_state_mut(id) else {
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

    pub fn show_panel(&mut self, id: &str) {
        if let Some(panel) = self.panel_state_mut(id) {
            panel.visible = true;
        }
    }

    pub fn hide_panel(&mut self, id: &str) {
        if let Some(panel) = self.panel_state_mut(id) {
            panel.visible = false;
        }
    }

    pub fn toggle_panel(&mut self, id: &str) {
        if let Some(panel) = self.panel_state_mut(id) {
            panel.visible = !panel.visible;
        }
    }

    pub fn bring_panel_to_front(&mut self, id: &str) {
        if self.panel_state(id).is_none() {
            return;
        }
        let next_z = {
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let z = root.next_z;
            root.next_z += 1;
            root.focused = Some(id.to_string());
            z
        };
        if let Some(panel) = self.panel_state_mut(id) {
            panel.z_index = next_z;
        }
    }

    pub fn start_panel_drag(&mut self, id: &str, x: f32, y: f32) {
        self.bring_panel_to_front(id);
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_drag = Some(PanelPointerSession {
            id: id.to_string(),
            last_x: x,
            last_y: y,
        });
    }

    pub fn move_panel_drag(&mut self, id: &str, x: f32, y: f32) {
        let Some((dx, dy)) = ({
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let Some(session) = root.active_drag.as_mut() else {
                return;
            };
            if session.id != id {
                return;
            }
            let dx = x - session.last_x;
            let dy = y - session.last_y;
            session.last_x = x;
            session.last_y = y;
            Some((dx, dy))
        }) else {
            return;
        };
        self.move_panel_by(id, dx, dy);
    }

    pub fn end_panel_drag(&mut self) {
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_drag = None;
    }

    pub fn start_panel_resize(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        self.bring_panel_to_front(id);
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_resize = Some(PanelResizeSession {
            id: id.to_string(),
            edge,
            last_x: x,
            last_y: y,
        });
    }

    pub fn move_panel_resize(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        let Some((dx, dy)) = ({
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let Some(session) = root.active_resize.as_mut() else {
                return;
            };
            if session.id != id || session.edge != edge {
                return;
            }
            let dx = x - session.last_x;
            let dy = y - session.last_y;
            session.last_x = x;
            session.last_y = y;
            Some((dx, dy))
        }) else {
            return;
        };
        self.resize_panel_by(id, edge, dx, dy);
    }

    pub fn end_panel_resize(&mut self) {
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_resize = None;
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;
    use crate::tree::layout::{BoxStyle, Size};
    use crate::tree::{reconcile, Desc};
    use crate::widget::props::WidgetBuildCx;
    use std::borrow::Cow;

    #[derive(Default)]
    struct TestRuntime {
        value: usize,
    }

    impl RuntimeSlot for TestRuntime {}

    fn build_cx<'a>(theme: &'a crate::theme::Theme) -> WidgetBuildCx<'a> {
        WidgetBuildCx {
            theme,
            force_rebuild: false,
        }
    }

    fn root_desc(width: f32) -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(width),
                height: Size::Fixed(100.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn runtime_slot_roundtrips_by_type() {
        let mut tree = Tree::new();
        let theme = light_theme();
        reconcile(&mut tree, root_desc(100.0), build_cx(&theme));
        let root = tree.root().expect("root");

        tree.ensure_runtime_slot::<TestRuntime>(root)
            .expect("slot")
            .value = 42;

        assert_eq!(
            tree.runtime_slot::<TestRuntime>(root)
                .expect("stored runtime")
                .value,
            42
        );

        let removed = tree
            .remove_runtime_slot::<TestRuntime>(root)
            .expect("removed runtime");
        assert_eq!(removed.value, 42);
        assert!(tree.runtime_slot::<TestRuntime>(root).is_none());
    }

    #[test]
    fn reconcile_preserves_runtime_for_stable_node() {
        let mut tree = Tree::new();
        let theme = light_theme();
        reconcile(&mut tree, root_desc(100.0), build_cx(&theme));
        let root = tree.root().expect("root");
        tree.ensure_runtime_slot::<TestRuntime>(root)
            .expect("slot")
            .value = 7;

        reconcile(&mut tree, root_desc(200.0), build_cx(&theme));
        let root_after = tree.root().expect("root after reconcile");

        assert_eq!(root_after, root);
        assert_eq!(
            tree.runtime_slot::<TestRuntime>(root_after)
                .expect("runtime survives reconcile")
                .value,
            7
        );
    }

    #[test]
    fn stable_id_lookup_uses_tree_node_identity() {
        let mut tree = Tree::new();
        let theme = light_theme();
        reconcile(&mut tree, root_desc(100.0), build_cx(&theme));

        assert_eq!(tree.node_by_stable_id(&StableId::from("root")), tree.root());
        assert_eq!(tree.node_by_str("root"), tree.root());
    }
}
