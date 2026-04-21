use super::node::{NodeId, PanelNode};
use super::runtime::TreeRuntime;
use super::runtime_slots::RuntimeSlot;
use super::StableId;
use crate::panel::{PanelConfig, PanelRuntime};
use crate::widget::resize_edge::ResizeEdge;

/// 全局控件树存储。用 Vec<Option<>> 做 arena，索引访问。
pub struct Tree {
    nodes: Vec<Option<PanelNode>>,
    root: Option<NodeId>,
    free: Vec<NodeId>,
    runtime: TreeRuntime,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            free: Vec::new(),
            runtime: TreeRuntime::default(),
        }
    }

    pub fn insert(&mut self, node: PanelNode) -> NodeId {
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
            }
            self.free.push(id);
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&PanelNode> {
        self.nodes.get(id).and_then(|n| n.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut PanelNode> {
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

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &PanelNode)> {
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

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PanelNode> {
        self.nodes.iter_mut().filter_map(|n| n.as_mut())
    }

    pub fn ensure_panel(&mut self, config: &PanelConfig) {
        self.runtime.panels.ensure_panel(config);
        self.runtime
            .panels
            .clamp_min_size(config.id.as_str(), config.min_size);
    }

    pub fn panel_state(&self, id: &str) -> Option<&PanelRuntime> {
        self.runtime.panels.state(id)
    }

    pub fn panel_state_mut(&mut self, id: &str) -> Option<&mut PanelRuntime> {
        self.runtime.panels.state_mut(id)
    }

    pub fn move_panel_by(&mut self, id: &str, dx: f32, dy: f32) {
        self.runtime.panels.move_by(id, dx, dy);
    }

    pub fn resize_panel_by(&mut self, id: &str, edge: ResizeEdge, dx: f32, dy: f32) {
        self.runtime.panels.resize_by(id, edge, dx, dy);
    }

    pub fn show_panel(&mut self, id: &str) {
        self.runtime.panels.show(id);
    }

    pub fn hide_panel(&mut self, id: &str) {
        self.runtime.panels.hide(id);
    }

    pub fn toggle_panel(&mut self, id: &str) {
        self.runtime.panels.toggle(id);
    }

    pub fn bring_panel_to_front(&mut self, id: &str) {
        self.runtime.panels.bring_to_front(id);
    }

    pub fn start_panel_drag(&mut self, id: &str, x: f32, y: f32) {
        self.runtime.panels.start_drag(id, x, y);
    }

    pub fn move_panel_drag(&mut self, id: &str, x: f32, y: f32) {
        self.runtime.panels.drag_move(id, x, y);
    }

    pub fn end_panel_drag(&mut self) {
        self.runtime.panels.end_drag();
    }

    pub fn start_panel_resize(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        self.runtime.panels.start_resize(id, edge, x, y);
    }

    pub fn move_panel_resize(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        self.runtime.panels.resize_move(id, edge, x, y);
    }

    pub fn end_panel_resize(&mut self) {
        self.runtime.panels.end_resize();
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
