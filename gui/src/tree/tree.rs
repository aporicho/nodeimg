use super::node::{NodeId, TreeNode};
use super::runtime_slots::RuntimeSlot;
use super::{RuntimeSlots, StableId};
use crate::canvas::runtime::{CanvasInteractionRuntime, CanvasNodeRuntime};
use crate::canvas::{
    canvas_node_stable_id, CanvasNodeIdentity, CanvasNodeLayout, CanvasPortGroupView,
    CanvasPortSide,
};
use crate::panel::{
    PanelConfig, PanelLayout, PanelPointerSession, PanelResizeSession, PanelRootRuntime,
    PanelRuntime,
};
use crate::renderer::Rect;
use crate::widget::resize_edge::ResizeEdge;
use std::collections::{HashMap, HashSet};

const PANEL_ROOT_ID: &str = "panel_root";
const CANVAS_INTERACTION_ID: &str = "canvas_interaction";
const CANVAS_NODE_MIN_WIDTH: f32 = 304.0;
const CANVAS_NODE_MIN_HEIGHT: f32 = 132.0;

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

    pub(crate) fn sync_canvas_node_layouts(
        &mut self,
        identities: &[CanvasNodeIdentity],
    ) -> Vec<CanvasNodeLayout> {
        let owner_ids: HashSet<&str> = identities
            .iter()
            .map(|identity| identity.owner_id.as_str())
            .collect();
        self.retained_runtime.retain(|stable_id, slots| {
            if slots.get::<CanvasNodeRuntime>().is_none() {
                return true;
            }
            let Some(owner_id) = stable_id.strip_prefix("canvas_node::") else {
                return true;
            };
            owner_ids.contains(owner_id)
        });
        if let Some(interaction) =
            self.runtime_slot_by_stable_id_mut::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        {
            interaction.retain_owner_ids(&owner_ids);
        }

        let mut layouts = Vec::with_capacity(identities.len());
        for (index, identity) in identities.iter().enumerate() {
            let stable_id = canvas_node_stable_id(&identity.owner_id);
            let runtime = self.ensure_runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id);
            if runtime.owner_id.is_empty() {
                *runtime = CanvasNodeRuntime::from_identity(identity, index as i32);
            }
            layouts.push(runtime.to_layout());
        }

        layouts.sort_by(|a, b| {
            a.z_index
                .cmp(&b.z_index)
                .then_with(|| a.owner_id.cmp(&b.owner_id))
        });
        layouts
    }

    pub(crate) fn export_canvas_node_layouts(&self) -> Vec<CanvasNodeLayout> {
        let mut layouts: Vec<CanvasNodeLayout> = self
            .iter()
            .filter_map(|(_, node)| {
                node.runtime_slots
                    .get::<CanvasNodeRuntime>()
                    .map(CanvasNodeRuntime::to_layout)
            })
            .chain(self.retained_runtime.values().filter_map(|slots| {
                slots
                    .get::<CanvasNodeRuntime>()
                    .map(CanvasNodeRuntime::to_layout)
            }))
            .collect();

        layouts.sort_by(|a, b| a.owner_id.cmp(&b.owner_id));
        layouts
    }

    pub(crate) fn import_canvas_node_layouts(&mut self, layouts: &[CanvasNodeLayout]) {
        for layout in layouts {
            let stable_id = canvas_node_stable_id(&layout.owner_id);
            let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
            else {
                continue;
            };
            runtime.rect = layout.rect;
            runtime.z_index = layout.z_index;
            runtime.collapsed = layout.collapsed;
            runtime.user_min_height = layout.user_min_height;
        }
    }

    pub(crate) fn move_canvas_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            return false;
        };
        runtime.rect.x += dx;
        runtime.rect.y += dy;
        true
    }

    pub(crate) fn resize_canvas_node_by(
        &mut self,
        owner_id: &str,
        edge: ResizeEdge,
        dx: f32,
        dy: f32,
    ) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            tracing::debug!(
                target: "gui::canvas::node_resize",
                owner_id,
                edge = ?edge,
                dx,
                dy,
                "ignore canvas node resize: runtime missing"
            );
            return false;
        };

        let before = runtime.rect;
        let requested_w = requested_resize_width(before, edge, dx);
        let requested_h = requested_resize_height(before, edge, dy);
        resize_rect_by_edge(
            &mut runtime.rect,
            edge,
            dx,
            dy,
            CANVAS_NODE_MIN_WIDTH,
            CANVAS_NODE_MIN_HEIGHT,
        );
        if is_vertical_resize_edge(edge) && runtime.rect.h != before.h {
            runtime.user_min_height = Some(runtime.rect.h);
        }
        tracing::debug!(
            target: "gui::canvas::node_resize",
            owner_id,
            edge = ?edge,
            dx,
            dy,
            before_x = before.x,
            before_y = before.y,
            before_w = before.w,
            before_h = before.h,
            after_x = runtime.rect.x,
            after_y = runtime.rect.y,
            after_w = runtime.rect.w,
            after_h = runtime.rect.h,
            requested_w,
            requested_h,
            min_w = CANVAS_NODE_MIN_WIDTH,
            min_h = CANVAS_NODE_MIN_HEIGHT,
            clamped_w = requested_w < CANVAS_NODE_MIN_WIDTH,
            clamped_h = requested_h < CANVAS_NODE_MIN_HEIGHT,
            applied_dx = runtime.rect.x - before.x,
            applied_dy = runtime.rect.y - before.y,
            applied_dw = runtime.rect.w - before.w,
            applied_dh = runtime.rect.h - before.h,
            user_min_height = runtime.user_min_height,
            "resize canvas node runtime rect"
        );
        true
    }

    pub(crate) fn ensure_canvas_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            return false;
        };

        let next_width = runtime.rect.w.max(min_width.max(CANVAS_NODE_MIN_WIDTH));
        let next_height = runtime.rect.h.max(min_height.max(CANVAS_NODE_MIN_HEIGHT));
        let changed = next_width != runtime.rect.w || next_height != runtime.rect.h;
        let before = runtime.rect;
        runtime.rect.w = next_width;
        runtime.rect.h = next_height;
        tracing::debug!(
            target: "gui::canvas::node_resize",
            owner_id,
            requested_min_w = min_width,
            requested_min_h = min_height,
            engine_min_w = CANVAS_NODE_MIN_WIDTH,
            engine_min_h = CANVAS_NODE_MIN_HEIGHT,
            before_w = before.w,
            before_h = before.h,
            after_w = runtime.rect.w,
            after_h = runtime.rect.h,
            changed,
            "ensure canvas node min size"
        );
        changed
    }

    pub(crate) fn apply_canvas_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            return false;
        };

        let before = runtime.rect;
        runtime.rect.w = request
            .target_width
            .max(request.min_width)
            .max(CANVAS_NODE_MIN_WIDTH);
        runtime.rect.h = request
            .target_height
            .max(request.min_height)
            .max(CANVAS_NODE_MIN_HEIGHT);
        let changed = runtime.rect.w != before.w || runtime.rect.h != before.h;
        tracing::debug!(
            target: "gui::canvas::node_resize",
            owner_id,
            target_w = request.target_width,
            target_h = request.target_height,
            request_min_w = request.min_width,
            request_min_h = request.min_height,
            engine_min_w = CANVAS_NODE_MIN_WIDTH,
            engine_min_h = CANVAS_NODE_MIN_HEIGHT,
            before_w = before.w,
            before_h = before.h,
            after_w = runtime.rect.w,
            after_h = runtime.rect.h,
            user_min_height = runtime.user_min_height,
            changed,
            "apply canvas node absolute sizing"
        );
        changed
    }

    pub(crate) fn canvas_port_group_view(
        &self,
        owner_id: &str,
        side: CanvasPortSide,
    ) -> CanvasPortGroupView {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .map(|interaction| interaction.port_group_view(owner_id, side))
            .unwrap_or_default()
    }

    pub(crate) fn toggle_canvas_port_group(
        &mut self,
        owner_id: &str,
        side: CanvasPortSide,
    ) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .toggle_port_group(owner_id, side)
    }

    pub(crate) fn select_canvas_node(&mut self, owner_id: &str) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        if self
            .runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id)
            .is_none()
        {
            return false;
        }
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .select_single(owner_id);
        true
    }

    pub(crate) fn clear_canvas_selection(&mut self) {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .clear();
    }

    pub(crate) fn is_canvas_node_selected(&self, owner_id: &str) -> bool {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .is_some_and(|interaction| interaction.is_selected(owner_id))
    }

    pub(crate) fn pending_canvas_connection(
        &self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .and_then(|interaction| interaction.pending_connection().cloned())
    }

    pub(crate) fn begin_pending_canvas_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .begin_pending_connection(from_port_id, cursor_canvas)
    }

    pub(crate) fn update_pending_canvas_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .update_pending_connection(cursor_canvas)
    }

    pub(crate) fn end_pending_canvas_connection(
        &mut self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .end_pending_connection()
    }

    pub(crate) fn cancel_pending_canvas_connection(&mut self) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .cancel_pending_connection()
    }

    pub(crate) fn hovered_canvas_port_id(&self) -> Option<String> {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .and_then(|interaction| interaction.hovered_port_id().map(str::to_string))
    }

    pub(crate) fn set_hovered_canvas_port(&mut self, port_id: Option<&str>) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .set_hovered_port(port_id)
    }

    pub(crate) fn export_panel_layouts(&self) -> Vec<PanelLayout> {
        let mut layouts: Vec<PanelLayout> = self
            .iter()
            .filter_map(|(_, node)| {
                let panel = node.runtime_slots.get::<PanelRuntime>()?;
                Some(panel_layout_from_runtime(node.id.as_ref(), panel))
            })
            .chain(self.retained_runtime.iter().filter_map(|(id, slots)| {
                let panel = slots.get::<PanelRuntime>()?;
                Some(panel_layout_from_runtime(id, panel))
            }))
            .collect();

        layouts.sort_by(|a, b| a.id.cmp(&b.id));
        layouts
    }

    pub(crate) fn import_panel_layouts(&mut self, layouts: &[PanelLayout]) {
        let mut max_imported_z: Option<i32> = None;
        for layout in layouts {
            let Some(panel) = self.panel_state_mut(&layout.id) else {
                continue;
            };
            panel.rect = layout.rect;
            panel.rect.w = panel.rect.w.max(panel.min_size[0]);
            panel.rect.h = panel.rect.h.max(panel.min_size[1]);
            panel.visible = layout.visible;
            panel.z_index = layout.z_index;
            panel.collapsed = layout.collapsed;
            max_imported_z = Some(max_imported_z.map_or(layout.z_index, |z| z.max(layout.z_index)));
        }

        if let Some(max_imported_z) = max_imported_z {
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            root.next_z = root.next_z.max(max_imported_z + 1);
        }
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

fn resize_rect_by_edge(
    rect: &mut Rect,
    edge: ResizeEdge,
    dx: f32,
    dy: f32,
    min_width: f32,
    min_height: f32,
) {
    let right = rect.x + rect.w;
    let bottom = rect.y + rect.h;

    match edge {
        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
            let next_x = (rect.x + dx).min(right - min_width);
            rect.x = next_x;
            rect.w = right - next_x;
        }
        ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => {
            rect.w = (rect.w + dx).max(min_width);
        }
        _ => {}
    }

    match edge {
        ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
            let next_y = (rect.y + dy).min(bottom - min_height);
            rect.y = next_y;
            rect.h = bottom - next_y;
        }
        ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
            rect.h = (rect.h + dy).max(min_height);
        }
        _ => {}
    }

    rect.w = rect.w.max(min_width);
    rect.h = rect.h.max(min_height);
}

fn requested_resize_width(rect: Rect, edge: ResizeEdge, dx: f32) -> f32 {
    match edge {
        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => rect.w - dx,
        ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => rect.w + dx,
        ResizeEdge::Top | ResizeEdge::Bottom => rect.w,
    }
}

fn requested_resize_height(rect: Rect, edge: ResizeEdge, dy: f32) -> f32 {
    match edge {
        ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => rect.h - dy,
        ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => rect.h + dy,
        ResizeEdge::Left | ResizeEdge::Right => rect.h,
    }
}

fn is_vertical_resize_edge(edge: ResizeEdge) -> bool {
    matches!(
        edge,
        ResizeEdge::Top
            | ResizeEdge::TopLeft
            | ResizeEdge::TopRight
            | ResizeEdge::Bottom
            | ResizeEdge::BottomLeft
            | ResizeEdge::BottomRight
    )
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

fn panel_layout_from_runtime(id: &str, panel: &PanelRuntime) -> PanelLayout {
    PanelLayout {
        id: id.to_string(),
        rect: panel.rect,
        visible: panel.visible,
        z_index: panel.z_index,
        collapsed: panel.collapsed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
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

    #[test]
    fn canvas_node_layout_sync_uses_owner_identity() {
        let mut tree = Tree::new();
        let first = crate::canvas::CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: crate::renderer::Rect {
                x: 10.0,
                y: 20.0,
                w: 220.0,
                h: 96.0,
            },
        };

        let layouts = tree.sync_canvas_node_layouts(std::slice::from_ref(&first));
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].owner_id, "engine_node::1");
        assert_eq!(layouts[0].rect.x, 10.0);

        tree.import_canvas_node_layouts(&[crate::canvas::CanvasNodeLayout {
            owner_id: "engine_node::1".to_string(),
            rect: crate::renderer::Rect {
                x: 80.0,
                y: 90.0,
                w: 260.0,
                h: 120.0,
            },
            z_index: 7,
            collapsed: true,
            user_min_height: Some(120.0),
        }]);

        let layouts = tree.sync_canvas_node_layouts(&[first]);
        assert_eq!(layouts[0].rect.x, 80.0);
        assert_eq!(layouts[0].rect.y, 90.0);
        assert_eq!(layouts[0].z_index, 7);
        assert!(layouts[0].collapsed);
        assert_eq!(layouts[0].user_min_height, Some(120.0));

        assert!(tree.move_canvas_node_by("engine_node::1", 10.0, -5.0));
        let layouts = tree.export_canvas_node_layouts();
        assert_eq!(layouts[0].rect.x, 90.0);
        assert_eq!(layouts[0].rect.y, 85.0);

        assert!(tree.resize_canvas_node_by("engine_node::1", ResizeEdge::Right, 80.0, 20.0));
        let layouts = tree.export_canvas_node_layouts();
        assert_eq!(layouts[0].rect.w, 340.0);
        assert_eq!(layouts[0].rect.h, 132.0);

        let stale = tree.sync_canvas_node_layouts(&[]);
        assert!(stale.is_empty());
        assert!(tree.export_canvas_node_layouts().is_empty());
    }

    #[test]
    fn canvas_port_group_toggle_is_tree_runtime_state() {
        let mut tree = Tree::new();

        assert!(
            !tree
                .canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(
            tree.toggle_canvas_port_group("engine_node::1", crate::canvas::CanvasPortSide::Input)
        );
        assert!(
            tree.canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(
            !tree.toggle_canvas_port_group("engine_node::1", crate::canvas::CanvasPortSide::Input)
        );
    }

    #[test]
    fn canvas_interaction_state_tracks_selection_and_prunes_stale_nodes() {
        let mut tree = Tree::new();
        let first = CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 80.0,
            },
        };
        let second = CanvasNodeIdentity {
            owner_id: "engine_node::2".to_string(),
            default_rect: Rect {
                x: 120.0,
                y: 0.0,
                w: 100.0,
                h: 80.0,
            },
        };

        tree.sync_canvas_node_layouts(&[first.clone(), second.clone()]);
        assert!(tree.select_canvas_node("engine_node::1"));
        assert!(tree.is_canvas_node_selected("engine_node::1"));
        assert!(!tree.is_canvas_node_selected("engine_node::2"));
        assert!(
            tree.toggle_canvas_port_group("engine_node::1", crate::canvas::CanvasPortSide::Input)
        );
        assert!(
            tree.canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(!tree.begin_pending_canvas_connection(
            "canvas_node::engine_node::1::port::input::prompt",
            [2.0, 3.0],
        ));
        assert!(tree.begin_pending_canvas_connection(
            "canvas_node::engine_node::1::port::output::image",
            [2.0, 3.0],
        ));
        assert_eq!(
            tree.pending_canvas_connection()
                .map(|pending| pending.cursor_canvas),
            Some([2.0, 3.0])
        );
        assert!(tree.update_pending_canvas_connection([4.0, 5.0]));
        assert!(
            tree.set_hovered_canvas_port(Some("canvas_node::engine_node::1::port::input::prompt"))
        );
        assert_eq!(
            tree.hovered_canvas_port_id(),
            Some("canvas_node::engine_node::1::port::input::prompt".to_string())
        );
        assert!(tree.cancel_pending_canvas_connection());
        assert!(tree.pending_canvas_connection().is_none());
        assert!(tree.hovered_canvas_port_id().is_none());
        assert!(!tree.cancel_pending_canvas_connection());
        assert!(tree.begin_pending_canvas_connection(
            "canvas_node::engine_node::1::port::output::image",
            [2.0, 3.0],
        ));
        assert!(
            tree.set_hovered_canvas_port(Some("canvas_node::engine_node::1::port::input::prompt"))
        );

        tree.sync_canvas_node_layouts(&[second]);

        assert!(!tree.is_canvas_node_selected("engine_node::1"));
        assert!(
            !tree
                .canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(tree.pending_canvas_connection().is_none());
        assert!(tree.hovered_canvas_port_id().is_none());
    }
}
