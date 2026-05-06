use gui::canvas::camera::Camera;
use gui::canvas::node_template::CanvasNodeRenderView;
use gui::canvas::scene_model::{
    CanvasPendingConnectionState, CanvasSceneModel, CanvasSceneNodeState,
};
use gui::canvas::{
    canvas_node_stable_id, scene_change_to_mutation, CanvasConnectionView,
    CanvasPendingConnectionView, CanvasSceneChange,
};
use gui::context::Context;
use gui::control::{format_color_hex, ControlSpec};
use gui::diagnostics::render_trace::{self, RectSummary, RenderTraceStage};
use gui::diagnostics::tree_dump::TreeDumpPhase;
use gui::geometry::TransformSpec;
use gui::layout::{Decoration, Size, TextureHandle};
use gui::panel::PanelFrameTemplateData;
use gui::renderer::{Border, Point, Rect};
use gui::scene::{MutationError, SceneMutation, StylePatch};
use gui::template::{
    InstanceId, SlotValue, SlotValues, TemplateId, TemplatePayload, NODE_PALETTE_CATEGORY_TEMPLATE,
    NODE_PALETTE_EMPTY_TEMPLATE, NODE_PALETTE_ITEM_TEMPLATE, NODE_PALETTE_TEMPLATE,
    PANEL_FRAME_TEMPLATE,
};
use gui::theme::Theme;
use std::collections::{BTreeMap, BTreeSet};

use crate::panels::{
    registered_panels, EnginePanelState, PanelAppliedSnapshot, PanelRenderInput, PanelWorkspaceMode,
};

#[cfg(test)]
use super::composition::WorkspaceUiComposition;
use super::node_palette::{node_palette_item_id, NodePaletteState};
use super::scene_state::{PanelAppliedState, WorkspaceSceneState};

const GRID_SPACING: f32 = 20.0;

#[derive(Clone, Debug, Default)]
pub(crate) struct WorkspaceSceneController {
    canvas: CanvasSceneModel,
    workspace_scene: WorkspaceSceneState,
    node_palette: Option<NodePaletteOverlay>,
}

#[derive(Clone, Debug, PartialEq)]
struct NodePaletteOverlay {
    x: f32,
    y: f32,
    state: NodePaletteState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SceneSyncStats {
    pub(crate) nodes_added: usize,
    pub(crate) nodes_removed: usize,
    pub(crate) connections_added: usize,
    pub(crate) connections_removed: usize,
    pub(crate) mutations_queued: usize,
    pub(crate) mutations_effective: usize,
}

pub(crate) struct WorkspaceSceneInput<'a> {
    pub(crate) viewport: Rect,
    pub(crate) camera: &'a Camera,
    pub(crate) canvas_nodes: &'a [CanvasNodeRenderView],
    pub(crate) canvas_connections: &'a [CanvasConnectionView],
    pub(crate) pending_connection: Option<&'a CanvasPendingConnectionView>,
    pub(crate) theme: &'a Theme,
    pub(crate) preview_image: TextureHandle,
    pub(crate) engine_panel: &'a EnginePanelState,
    pub(crate) features: WorkspaceSceneFeatures,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceSceneFeatures {
    pub(crate) composition: &'static str,
    pub(crate) panel_mode: PanelWorkspaceMode,
    pub(crate) node_palette_enabled: bool,
}

impl WorkspaceSceneFeatures {
    pub(crate) fn new(
        composition: &'static str,
        panel_mode: PanelWorkspaceMode,
        node_palette_enabled: bool,
    ) -> Self {
        Self {
            composition,
            panel_mode,
            node_palette_enabled,
        }
    }

    #[cfg(test)]
    pub(crate) fn full() -> Self {
        let composition = WorkspaceUiComposition::full();
        Self::new(
            composition.name(),
            composition.panel_mode(),
            composition.node_palette_enabled(),
        )
    }

    #[cfg(test)]
    pub(crate) fn clean_room() -> Self {
        let composition = WorkspaceUiComposition::clean_room();
        Self::new(
            composition.name(),
            composition.panel_mode(),
            composition.node_palette_enabled(),
        )
    }
}

impl WorkspaceSceneController {
    pub(crate) fn sync(
        &mut self,
        gui: &mut Context,
        input: WorkspaceSceneInput<'_>,
    ) -> Result<SceneSyncStats, MutationError> {
        let roots = gui.scene().ensure_retained_root(input.viewport)?;
        let canvas_transform =
            TransformSpec::translate_scale([input.camera.x, input.camera.y], input.camera.zoom);
        if self.canvas.set_canvas_transform(canvas_transform) {
            gui.scene().update_canvas_transform(canvas_transform)?;
        }

        let mut stats = SceneSyncStats::default();
        if input.features.node_palette_enabled {
            stats.mutations_effective +=
                self.ensure_node_palette_root(gui, roots.overlay_root, input.viewport)?;
        } else {
            self.node_palette = None;
        }
        let mut mutations = Vec::new();
        let grid_rect = canvas_grid_rect(input.viewport, input.camera);
        if self.canvas.set_grid_rect(grid_rect) {
            mutations.push(SceneMutation::SetRect {
                node: roots.canvas_grid,
                rect: grid_rect,
            });
        }

        self.sync_nodes(
            roots.canvas_root,
            input.canvas_nodes,
            gui,
            &mut mutations,
            &mut stats,
            input.theme,
        );
        self.sync_connections(
            roots.canvas_connections,
            input.canvas_connections,
            input.pending_connection,
            gui,
            &mut mutations,
            &mut stats,
        );
        self.sync_panels(
            roots.panel_root,
            gui,
            input.features.panel_mode,
            input.theme,
            PanelRenderInput {
                preview_image: input.preview_image,
                engine_panel: input.engine_panel,
            },
            &mut mutations,
        );
        if input.features.node_palette_enabled {
            self.sync_node_palette(input.viewport, gui, &mut mutations);
        } else {
            self.sync_node_palette_disabled(gui, &mut mutations);
        }

        stats.mutations_queued += mutations.len();
        let invalidations = gui.scene().apply_many(mutations)?;
        stats.mutations_effective += invalidations
            .iter()
            .filter(|invalidation| !invalidation.flags.is_empty())
            .count();
        render_trace::debug_stage(
            RenderTraceStage::SceneSync,
            SceneSyncTraceSummary {
                viewport: RectSummary::from(input.viewport),
                composition: input.features.composition,
                panel_mode: input.features.panel_mode,
                node_palette_enabled: input.features.node_palette_enabled,
                desired_nodes: input.canvas_nodes.len(),
                desired_connections: input.canvas_connections.len(),
                pending_connection: input.pending_connection.is_some(),
                mutations_queued: stats.mutations_queued,
                stats,
            },
        );
        gui.maybe_dump_tree(
            RenderTraceStage::SceneSync,
            TreeDumpPhase::After,
            "after workspace scene sync mutations",
        );
        for view in input.canvas_nodes {
            let owner_id = view.state.owner_id.as_str();
            let stable_id = canvas_node_stable_id(owner_id);
            if let Some(root) = gui.query().node_id_by_name(&stable_id) {
                self.canvas.insert_node_state(
                    owner_id.to_string(),
                    root,
                    canvas_node_state(view, input.theme),
                );
            }
        }
        Ok(stats)
    }

    fn sync_panels(
        &mut self,
        parent: usize,
        gui: &mut Context,
        panel_mode: PanelWorkspaceMode,
        theme: &Theme,
        panel_input: PanelRenderInput<'_>,
        mutations: &mut Vec<SceneMutation>,
    ) {
        let panels = registered_panels(panel_input);
        let desired_ids = panels
            .iter()
            .filter(|panel| panel.modes.contains(panel_mode))
            .map(|panel| panel.config.id.as_str().to_string())
            .collect::<BTreeSet<_>>();
        let mut stale_ids = panels
            .iter()
            .filter(|panel| !panel.modes.contains(panel_mode))
            .map(|panel| panel.config.id.as_str().to_string())
            .collect::<BTreeSet<_>>();
        stale_ids.extend(
            self.workspace_scene
                .panel_ids()
                .filter(|id| !desired_ids.contains(*id))
                .map(str::to_string),
        );
        for id in stale_ids {
            if let Some(root) = gui.query().node_id_by_name(&id) {
                mutations.push(SceneMutation::Unmount { node: root });
            }
            self.workspace_scene.remove_panel(&id);
        }

        for panel in panels {
            if !panel.modes.contains(panel_mode) {
                continue;
            }
            let config = panel.runtime_config(theme);
            let Some(runtime) = gui.panel_mut().ensure_runtime(&config) else {
                continue;
            };
            let id = config.id.as_str().to_string();
            let root = gui.query().node_id_by_name(&id);
            let next_state = panel_applied_state(
                runtime.visible,
                runtime.rect,
                runtime.z_index,
                &panel.applied,
            );
            if !runtime.visible {
                if let Some(root) = root {
                    mutations.push(SceneMutation::Unmount { node: root });
                }
                self.workspace_scene.remove_panel(&id);
                continue;
            }
            if let Some(root) = root {
                let previous = self.workspace_scene.panel(&id);
                if previous.is_none_or(|state| state.rect != runtime.rect) {
                    mutations.push(SceneMutation::SetRect {
                        node: root,
                        rect: runtime.rect,
                    });
                }
                if previous.is_none_or(|state| state.z_index != runtime.z_index) {
                    mutations.push(SceneMutation::SetZIndex {
                        node: root,
                        z_index: runtime.z_index,
                    });
                }
                for (text_id, value) in &next_state.texts {
                    if previous
                        .and_then(|state| state.texts.get(text_id))
                        .is_none_or(|old| old != value)
                    {
                        if let Some(node) = gui.query().node_id_by_name(text_id) {
                            mutations.push(SceneMutation::SetText {
                                node,
                                value: value.clone(),
                            });
                        }
                    }
                }
                self.workspace_scene.set_panel(id, next_state);
                continue;
            }

            mutations.push(SceneMutation::MountTemplate {
                parent,
                template: TemplateId::from(PANEL_FRAME_TEMPLATE),
                instance: InstanceId::from(id.clone()),
                payload: TemplatePayload::PanelFrame(PanelFrameTemplateData::new(
                    config,
                    runtime,
                    panel.content,
                    theme,
                )),
            });
            self.workspace_scene.set_panel(id, next_state);
        }
    }

    pub(crate) fn open_node_palette(&mut self, x: f32, y: f32, state: NodePaletteState) {
        self.node_palette = Some(NodePaletteOverlay { x, y, state });
    }

    pub(crate) fn close_overlay(&mut self) {
        self.node_palette = None;
    }

    pub(crate) fn node_palette_open(&self) -> bool {
        self.node_palette.is_some()
    }

    fn sync_nodes(
        &mut self,
        canvas_root: usize,
        views: &[CanvasNodeRenderView],
        gui: &Context,
        mutations: &mut Vec<SceneMutation>,
        stats: &mut SceneSyncStats,
        theme: &Theme,
    ) {
        let desired = views
            .iter()
            .map(|view| view.state.owner_id.as_str())
            .collect::<BTreeSet<_>>();
        let stale = self
            .canvas
            .nodes()
            .filter_map(|(owner_id, node)| {
                (!desired.contains(owner_id)).then_some((owner_id.to_string(), node.root_node))
            })
            .collect::<Vec<_>>();
        for (owner_id, root) in stale {
            self.canvas.remove_node(&owner_id);
            mutations.push(SceneMutation::Unmount { node: root });
            stats.nodes_removed += 1;
        }

        for view in views {
            let owner_id = view.state.owner_id.as_str();
            let stable_id = canvas_node_stable_id(owner_id);
            let next_state = canvas_node_state(view, theme);
            if self.canvas.node(owner_id).is_none() {
                mutations.push(scene_change_to_mutation(CanvasSceneChange::AddNodeCard {
                    parent: canvas_root,
                    view: view.clone(),
                    theme: theme.clone(),
                }));
                stats.nodes_added += 1;
                continue;
            }

            if let Some(root) = gui.query().node_id_by_name(&stable_id) {
                let previous = self.canvas.node_state(owner_id);
                if previous.and_then(|state| state.rect) != next_state.rect {
                    push_canvas_node_frame_mutations(
                        gui,
                        mutations,
                        &stable_id,
                        root,
                        view.state.layout.rect,
                    );
                }
                if previous.and_then(|state| state.z_index) != next_state.z_index {
                    mutations.push(SceneMutation::SetZIndex {
                        node: root,
                        z_index: view.state.layout.z_index,
                    });
                }
            }
            if self
                .canvas
                .node_state(owner_id)
                .and_then(|state| state.label.as_deref())
                != next_state.label.as_deref()
            {
                if let Some(label) = gui
                    .query()
                    .node_id_by_name(&format!("{stable_id}::label_text"))
                {
                    mutations.push(SceneMutation::SetText {
                        node: label,
                        value: view.template.title.clone(),
                    });
                }
            }
            if self
                .canvas
                .node_state(owner_id)
                .and_then(|state| state.card_decoration.as_ref())
                != next_state.card_decoration.as_ref()
            {
                if let Some(card) = gui.query().node_id_by_name(&format!("{stable_id}::card")) {
                    mutations.push(SceneMutation::SetStyle {
                        node: card,
                        patch: StylePatch {
                            decoration: next_state.card_decoration.clone(),
                            ..StylePatch::default()
                        },
                    });
                }
            }
            sync_canvas_node_param_texts(
                &stable_id,
                &next_state,
                self.canvas.node_state(owner_id),
                gui,
                mutations,
            );
        }
    }

    fn sync_connections(
        &mut self,
        parent: usize,
        connections: &[CanvasConnectionView],
        pending: Option<&CanvasPendingConnectionView>,
        gui: &Context,
        mutations: &mut Vec<SceneMutation>,
        stats: &mut SceneSyncStats,
    ) {
        let desired = connections
            .iter()
            .enumerate()
            .map(|(index, _)| format!("canvas_connection::{index}"))
            .collect::<BTreeSet<_>>();
        let stale = self
            .canvas
            .connections()
            .filter(|id| !desired.contains(*id))
            .map(str::to_string)
            .collect::<Vec<_>>();
        for id in stale {
            self.canvas.remove_connection(&id);
            if let Some(root) = gui.query().node_id_by_name(&id) {
                mutations.push(scene_change_to_mutation(
                    CanvasSceneChange::RemoveConnection { root },
                ));
                stats.connections_removed += 1;
            }
        }

        for (index, connection) in connections.iter().enumerate() {
            let id = format!("canvas_connection::{index}");
            if let Some(previous) = self.canvas.connection(&id) {
                if let Some(node) = gui.query().node_id_by_name(&id) {
                    if previous.from_port != connection.from_port_id
                        || previous.to_port != connection.to_port_id
                    {
                        mutations.push(SceneMutation::SetConnection {
                            node,
                            from_port: connection.from_port_id.clone(),
                            to_port: connection.to_port_id.clone(),
                        });
                    }
                }
                self.canvas.insert_connection_state(
                    id,
                    connection.from_port_id.clone(),
                    connection.to_port_id.clone(),
                );
            } else {
                mutations.push(scene_change_to_mutation(CanvasSceneChange::AddConnection {
                    parent,
                    id: id.clone(),
                    from_port: connection.from_port_id.clone(),
                    to_port: connection.to_port_id.clone(),
                }));
                self.canvas.insert_connection_state(
                    id,
                    connection.from_port_id.clone(),
                    connection.to_port_id.clone(),
                );
                stats.connections_added += 1;
            }
        }

        let pending_id = "canvas_connection::pending";
        match (pending, gui.query().node_id_by_name(pending_id)) {
            (Some(pending), Some(node)) => {
                let state = CanvasPendingConnectionState {
                    from_port: pending.from_port_id.clone(),
                    cursor_canvas: Point {
                        x: pending.cursor_canvas[0],
                        y: pending.cursor_canvas[1],
                    },
                };
                if self.canvas.set_pending_connection(Some(state.clone())) {
                    mutations.push(scene_change_to_mutation(
                        CanvasSceneChange::UpdatePendingConnection {
                            node,
                            from_port: state.from_port,
                            cursor_canvas: state.cursor_canvas,
                        },
                    ));
                }
            }
            (Some(pending), None) => {
                let state = CanvasPendingConnectionState {
                    from_port: pending.from_port_id.clone(),
                    cursor_canvas: Point {
                        x: pending.cursor_canvas[0],
                        y: pending.cursor_canvas[1],
                    },
                };
                self.canvas.set_pending_connection(Some(state.clone()));
                mutations.push(scene_change_to_mutation(
                    CanvasSceneChange::AddPendingConnection {
                        parent,
                        from_port: state.from_port,
                        cursor_canvas: state.cursor_canvas,
                    },
                ));
            }
            (None, Some(root)) => {
                if self.canvas.set_pending_connection(None) {
                    mutations.push(scene_change_to_mutation(
                        CanvasSceneChange::RemovePendingConnection { root },
                    ));
                }
            }
            (None, None) => {
                self.canvas.set_pending_connection(None);
            }
        }
    }

    fn ensure_node_palette_root(
        &mut self,
        gui: &mut Context,
        parent: usize,
        viewport: Rect,
    ) -> Result<usize, MutationError> {
        let Some(overlay) = &self.node_palette else {
            return Ok(0);
        };
        if gui.query().node_id_by_name("node_palette").is_some() {
            return Ok(0);
        }
        let rect = node_palette_rect(overlay.x, overlay.y, viewport);

        gui.scene().apply(SceneMutation::MountTemplate {
            parent,
            template: TemplateId::from(NODE_PALETTE_TEMPLATE),
            instance: InstanceId::from("node_palette"),
            payload: TemplatePayload::from(SlotValues::new().with("rect", SlotValue::Rect(rect))),
        })?;
        self.workspace_scene.set_palette_root_rect(Some(rect));
        Ok(1)
    }

    fn sync_node_palette(
        &mut self,
        viewport: Rect,
        gui: &Context,
        mutations: &mut Vec<SceneMutation>,
    ) {
        let root = gui.query().node_id_by_name("node_palette");
        let Some(overlay) = &self.node_palette else {
            if let Some(root) = root {
                mutations.push(SceneMutation::Unmount { node: root });
            }
            self.workspace_scene.clear_palette();
            return;
        };
        let Some(root) = root else {
            return;
        };
        let Some(items_parent) = gui.query().node_id_by_name("node_palette::items") else {
            return;
        };

        let root_rect = node_palette_rect(overlay.x, overlay.y, viewport);
        if self.workspace_scene.palette_root_rect() != Some(root_rect) {
            mutations.push(SceneMutation::SetRect {
                node: root,
                rect: root_rect,
            });
            self.workspace_scene.set_palette_root_rect(Some(root_rect));
        }

        let mut desired = BTreeMap::new();
        if overlay.state.items.is_empty() {
            let id = "node_palette_empty".to_string();
            desired.insert(id.clone(), "No nodes available".to_string());
            if gui.query().node_id_by_name(&id).is_none() {
                mutations.push(SceneMutation::MountTemplate {
                    parent: items_parent,
                    template: TemplateId::from(NODE_PALETTE_EMPTY_TEMPLATE),
                    instance: InstanceId::from(id),
                    payload: TemplatePayload::from(
                        SlotValues::new()
                            .with("label", SlotValue::Text("No nodes available".to_string())),
                    ),
                });
                self.workspace_scene
                    .set_palette_child_label("node_palette_empty", "No nodes available");
            }
        } else {
            let mut seen_categories = BTreeSet::new();
            for item in &overlay.state.items {
                if seen_categories.insert(item.category.as_str()) {
                    let category_id = format!("node_palette::category::{}", item.category);
                    desired.insert(category_id.clone(), item.category.clone());
                    if gui.query().node_id_by_name(&category_id).is_none() {
                        mutations.push(SceneMutation::MountTemplate {
                            parent: items_parent,
                            template: TemplateId::from(NODE_PALETTE_CATEGORY_TEMPLATE),
                            instance: InstanceId::from(category_id.clone()),
                            payload: TemplatePayload::from(
                                SlotValues::new()
                                    .with("label", SlotValue::Text(item.category.clone())),
                            ),
                        });
                        self.workspace_scene
                            .set_palette_child_label(category_id, item.category.clone());
                    } else if let Some(label) = gui
                        .query()
                        .node_id_by_name(&format!("{category_id}::label"))
                    {
                        if self.workspace_scene.palette_child_label(&category_id)
                            != Some(item.category.as_str())
                        {
                            mutations.push(SceneMutation::SetText {
                                node: label,
                                value: item.category.clone(),
                            });
                            self.workspace_scene
                                .set_palette_child_label(category_id, item.category.clone());
                        }
                    }
                }

                let item_id = node_palette_item_id(&item.type_id);
                let label = format!("{}  [{}]", item.name, item.source);
                desired.insert(item_id.clone(), label.clone());
                if gui.query().node_id_by_name(&item_id).is_none() {
                    mutations.push(SceneMutation::MountTemplate {
                        parent: items_parent,
                        template: TemplateId::from(NODE_PALETTE_ITEM_TEMPLATE),
                        instance: InstanceId::from(item_id.clone()),
                        payload: TemplatePayload::from(
                            SlotValues::new().with("label", SlotValue::Text(label)),
                        ),
                    });
                    self.workspace_scene.set_palette_child_label(
                        item_id,
                        format!("{}  [{}]", item.name, item.source),
                    );
                } else if let Some(label_node) =
                    gui.query().node_id_by_name(&format!("{item_id}::label"))
                {
                    if self.workspace_scene.palette_child_label(&item_id) != Some(label.as_str()) {
                        mutations.push(SceneMutation::SetText {
                            node: label_node,
                            value: label.clone(),
                        });
                        self.workspace_scene.set_palette_child_label(item_id, label);
                    }
                }
            }
        }

        for id in self
            .workspace_scene
            .palette_child_ids()
            .filter(|id| !desired.contains_key(*id))
            .map(str::to_string)
            .collect::<Vec<_>>()
        {
            if let Some(node) = gui.query().node_id_by_name(&id) {
                mutations.push(SceneMutation::Unmount { node });
            }
            self.workspace_scene.remove_palette_child(&id);
        }
        for (id, label) in desired {
            self.workspace_scene.set_palette_child_label(id, label);
        }
    }

    fn sync_node_palette_disabled(&mut self, gui: &Context, mutations: &mut Vec<SceneMutation>) {
        self.node_palette = None;
        if let Some(root) = gui.query().node_id_by_name("node_palette") {
            mutations.push(SceneMutation::Unmount { node: root });
        }
        self.workspace_scene.clear_palette();
    }
}

fn push_canvas_node_frame_mutations(
    gui: &Context,
    mutations: &mut Vec<SceneMutation>,
    stable_id: &str,
    root: usize,
    rect: Rect,
) {
    mutations.push(SceneMutation::SetRect { node: root, rect });
    push_node_frame_size_patch(mutations, root, rect);
    if let Some(card) = gui.query().node_id_by_name(&format!("{stable_id}::card")) {
        push_node_frame_size_patch(mutations, card, rect);
    }
    for suffix in ["::pin_column::input", "::pin_column::output"] {
        if let Some(node) = gui.query().node_id_by_name(&format!("{stable_id}{suffix}")) {
            push_pin_column_height_patch(mutations, node, rect.h);
        }
    }
}

fn push_node_frame_size_patch(mutations: &mut Vec<SceneMutation>, node: usize, rect: Rect) {
    mutations.push(SceneMutation::SetStyle {
        node,
        patch: StylePatch {
            width: Some(Size::Fixed(rect.w)),
            height: Some(Size::Fixed(rect.h)),
            ..StylePatch::default()
        },
    });
}

fn push_pin_column_height_patch(mutations: &mut Vec<SceneMutation>, node: usize, height: f32) {
    mutations.push(SceneMutation::SetStyle {
        node,
        patch: StylePatch {
            height: Some(Size::Fixed(height)),
            ..StylePatch::default()
        },
    });
}

#[allow(dead_code)]
#[derive(Debug)]
struct SceneSyncTraceSummary {
    viewport: RectSummary,
    composition: &'static str,
    panel_mode: PanelWorkspaceMode,
    node_palette_enabled: bool,
    desired_nodes: usize,
    desired_connections: usize,
    pending_connection: bool,
    mutations_queued: usize,
    stats: SceneSyncStats,
}

fn canvas_grid_rect(viewport: Rect, camera: &Camera) -> Rect {
    let (canvas_min_x, canvas_min_y) = camera.screen_to_canvas(0.0, 0.0);
    let (canvas_max_x, canvas_max_y) = camera.screen_to_canvas(viewport.w, viewport.h);
    let x = align_grid_start(canvas_min_x, GRID_SPACING);
    let y = align_grid_start(canvas_min_y, GRID_SPACING);
    Rect {
        x,
        y,
        w: ((canvas_max_x - canvas_min_x).abs() + GRID_SPACING * 4.0).max(GRID_SPACING),
        h: ((canvas_max_y - canvas_min_y).abs() + GRID_SPACING * 4.0).max(GRID_SPACING),
    }
}

fn align_grid_start(min_canvas: f32, spacing: f32) -> f32 {
    (min_canvas / spacing).floor() * spacing - spacing * 2.0
}

fn node_palette_rect(x: f32, y: f32, viewport: Rect) -> Rect {
    const WIDTH: f32 = 320.0;
    const HEIGHT: f32 = 372.0;
    const MARGIN: f32 = 8.0;
    Rect {
        x: x.clamp(MARGIN, (viewport.w - WIDTH - MARGIN).max(MARGIN)),
        y: y.clamp(MARGIN, (viewport.h - HEIGHT - MARGIN).max(MARGIN)),
        w: WIDTH,
        h: HEIGHT,
    }
}

fn canvas_node_state(view: &CanvasNodeRenderView, theme: &Theme) -> CanvasSceneNodeState {
    CanvasSceneNodeState {
        rect: Some(view.state.layout.rect),
        z_index: Some(view.state.layout.z_index),
        label: Some(view.template.title.clone()),
        card_decoration: Some(Some(canvas_node_card_decoration(view, theme))),
        param_texts: canvas_node_param_texts(&canvas_node_stable_id(&view.state.owner_id), view),
    }
}

fn canvas_node_card_decoration(view: &CanvasNodeRenderView, theme: &Theme) -> Decoration {
    Decoration {
        background: Some(theme.colors.surface),
        border: Some(Border {
            width: if view.state.selected { 2.0 } else { 1.0 },
            color: if view.state.selected {
                theme.colors.text
            } else {
                theme.colors.border
            },
        }),
        radius: [theme.radii.md; 4],
        shadow: None,
    }
}

fn sync_canvas_node_param_texts(
    _stable_id: &str,
    next_state: &CanvasSceneNodeState,
    previous: Option<&CanvasSceneNodeState>,
    gui: &Context,
    mutations: &mut Vec<SceneMutation>,
) {
    for (target_id, value) in &next_state.param_texts {
        if previous
            .and_then(|state| state.param_texts.get(target_id))
            .is_some_and(|old| old == value)
        {
            continue;
        }
        if let Some(node) = gui.query().node_id_by_name(target_id) {
            mutations.push(SceneMutation::SetText {
                node,
                value: value.clone(),
            });
        }
    }
}

fn canvas_node_param_texts(
    stable_id: &str,
    view: &CanvasNodeRenderView,
) -> BTreeMap<String, String> {
    let mut texts = BTreeMap::new();
    for param in &view.template.params {
        let control_id = format!("{stable_id}::body::param::{}::control::content", param.key);
        let (target_id, value) = match &param.control {
            ControlSpec::Text { value } | ControlSpec::TextArea { value, .. } => {
                (format!("{control_id}::value"), value.clone())
            }
            ControlSpec::Number {
                value, precision, ..
            } => (
                format!("{control_id}::value"),
                format!("{value:.precision$}"),
            ),
            ControlSpec::ReadOnly { value } => (control_id, value.clone()),
            ControlSpec::Select { options, selected } => (
                control_id,
                options.get(*selected).cloned().unwrap_or_default(),
            ),
            ControlSpec::FilePath { path, .. } => (control_id, path.clone()),
            ControlSpec::Color { rgba } => {
                (format!("{control_id}::value"), format_color_hex(*rgba))
            }
            ControlSpec::Button { label } => (format!("{control_id}::label"), label.clone()),
            ControlSpec::Image { .. }
            | ControlSpec::Group { .. }
            | ControlSpec::Label { .. }
            | ControlSpec::Slider { .. }
            | ControlSpec::Toggle { .. } => continue,
        };
        texts.insert(target_id, value);
    }
    texts
}

fn panel_applied_state(
    visible: bool,
    rect: Rect,
    z_index: i32,
    applied: &PanelAppliedSnapshot,
) -> PanelAppliedState {
    PanelAppliedState {
        visible,
        rect,
        z_index,
        texts: applied.texts.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::composition::WorkspaceUiComposition;
    use crate::workspace::control_sync::canvas_text_box_sync_items;
    use crate::workspace::controller::WorkspaceController;
    use crate::workspace::node_palette::NodePaletteItem;
    use crate::workspace::showcase_node;
    use crate::workspace::showcase_state::ShowcaseState;
    use crate::workspace::ui_control_test_node::UI_CONTROL_TEST_TYPE_ID;
    use gui::canvas::CanvasNodeLayout;
    use gui::control::ControlValue;
    use gui::cursor::CursorKind;
    use gui::geometry::ResizeEdge;
    use gui::layout::TextureHandle;
    use gui::output::{ControlEvent, GuiEvent};
    use gui::renderer::TextMeasurer;
    use gui::shell::{AppEvent, Key, Modifiers, MouseButton};
    use gui::theme::light_theme;

    fn viewport() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: 900.0,
            h: 700.0,
        }
    }

    fn clean_room_canvas_nodes(
        workspace: &mut WorkspaceController,
        gui: &mut Context,
        theme: &Theme,
    ) -> Vec<CanvasNodeRenderView> {
        workspace.canvas_node_render_views(gui, theme, WorkspaceUiComposition::clean_room())
    }

    #[test]
    fn node_palette_open_mounts_retained_templates() {
        let mut gui = Context::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let state = NodePaletteState {
            items: vec![NodePaletteItem {
                type_id: "image_gen".to_string(),
                name: "Image Generator".to_string(),
                category: "Generators".to_string(),
                source: "built-in".to_string(),
            }],
        };

        controller.open_node_palette(40.0, 50.0, state);
        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &[],
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &light_theme(),
                    preview_image: TextureHandle(1),
                    engine_panel: &empty_engine_panel(),
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        assert!(gui.query().node_exists("node_palette"));
        assert!(gui
            .query()
            .node_exists("node_palette::category::Generators"));
        assert!(gui.query().node_exists("node_library::add::image_gen"));
        assert_eq!(gui.last_frame_stats().parent_lookup_fallback_scans, 0);
    }

    #[test]
    fn node_palette_close_unmounts_retained_overlay() {
        let mut gui = Context::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        controller.open_node_palette(40.0, 50.0, NodePaletteState { items: Vec::new() });
        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &[],
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &light_theme(),
                    preview_image: TextureHandle(1),
                    engine_panel: &empty_engine_panel(),
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("open sync");

        controller.close_overlay();
        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &[],
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &light_theme(),
                    preview_image: TextureHandle(1),
                    engine_panel: &empty_engine_panel(),
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("close sync");

        assert!(!gui.query().node_exists("node_palette"));
        assert!(!gui.query().node_exists("node_palette_empty"));
    }

    #[test]
    fn workspace_scene_second_sync_queues_zero_mutations() {
        let mut gui = Context::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();

        let first = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &[],
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("first sync");
        let second = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &[],
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("second sync");

        assert!(first.mutations_queued > 0);
        assert_eq!(second.mutations_queued, 0);
        assert_eq!(second.mutations_effective, 0);
    }

    #[test]
    fn retained_root_does_not_overwrite_canvas_grid_rect() {
        let mut gui = Context::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let expected_grid = canvas_grid_rect(viewport(), &camera);

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &[],
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("first sync");
        assert_eq!(gui.query().node_rect("canvas_grid"), Some(expected_grid));

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &[],
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("second sync");

        assert_eq!(gui.query().node_rect("canvas_grid"), Some(expected_grid));
    }

    #[test]
    fn clean_room_scene_mounts_grid_toolbar_and_one_node() {
        let mut gui = Context::new();
        let mut workspace = WorkspaceController::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let nodes = clean_room_canvas_nodes(&mut workspace, &mut gui, &theme);

        let first = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("first clean room sync");
        let second = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("second clean room sync");

        assert!(first.mutations_queued > 0);
        assert_eq!(second.mutations_queued, 0);
        assert!(gui.query().node_exists("canvas_grid"));
        assert!(gui.query().node_exists("canvas_node::engine_node::0"));
        assert_eq!(nodes[0].template.type_id, UI_CONTROL_TEST_TYPE_ID);
        assert!(gui.query().node_exists("toolbar"));
        assert!(gui.query().node_exists("preview"));
        assert!(!gui.query().node_exists("engine"));
        assert!(!gui.query().node_exists("node_palette"));

        let toolbar = registered_panels(PanelRenderInput {
            preview_image: TextureHandle(1),
            engine_panel: &engine_panel,
        })
        .into_iter()
        .find(|panel| panel.config.id.as_str() == "toolbar")
        .expect("toolbar panel should be registered");
        let toolbar_runtime = gui.panel().runtime("toolbar").expect("toolbar runtime");
        assert!(toolbar_runtime.rect.h >= toolbar.runtime_config(&theme).min_size[1]);
    }

    #[test]
    fn clean_room_engine_text_param_updates_without_remounting_node() {
        let mut gui = Context::new();
        let mut workspace = WorkspaceController::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let nodes = clean_room_canvas_nodes(&mut workspace, &mut gui, &theme);

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("initial clean room sync");
        assert!(workspace.update_control_value(
            "canvas_node::engine_node::0::body::param::prompt::control::content",
            ControlValue::Text("changed".to_string()),
            WorkspaceUiComposition::clean_room(),
        ));
        let updated_nodes = clean_room_canvas_nodes(&mut workspace, &mut gui, &theme);

        let update = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &updated_nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("updated clean room sync");

        assert_eq!(update.nodes_added, 0);
        assert_eq!(update.nodes_removed, 0);
        assert_eq!(update.mutations_queued, 1);
    }

    #[test]
    fn clean_room_canvas_node_resize_updates_retained_card_without_remounting() {
        let mut gui = Context::new();
        let mut workspace = WorkspaceController::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let nodes = clean_room_canvas_nodes(&mut workspace, &mut gui, &theme);

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("initial clean room sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let card_id = "canvas_node::engine_node::0::card";
        let before_card = gui.query().node_rect(card_id).expect("node card rect");
        let start_x = before_card.x + before_card.w;
        let start_y = before_card.y + before_card.h;

        assert!(workspace.start_canvas_node_resize(
            &mut gui,
            &camera,
            card_id,
            ResizeEdge::BottomRight,
            start_x,
            start_y,
        ));
        assert!(workspace.resize_canvas_node(
            &mut gui,
            &camera,
            card_id,
            ResizeEdge::BottomRight,
            start_x + 70.0,
            start_y + 40.0,
        ));
        assert!(!workspace.end_canvas_node_resize(
            &mut gui,
            &camera,
            card_id,
            ResizeEdge::BottomRight,
            start_x + 70.0,
            start_y + 40.0,
        ));

        let resized_nodes = clean_room_canvas_nodes(&mut workspace, &mut gui, &theme);
        let update = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &resized_nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("resized clean room sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let after_card = gui.query().node_rect(card_id).expect("resized node card");
        assert_eq!(update.nodes_added, 0);
        assert_eq!(update.nodes_removed, 0);
        assert!(update.mutations_queued > 0);
        assert_eq!(after_card.w, before_card.w + 70.0);
        assert_eq!(after_card.h, before_card.h + 40.0);

        let right_edge_hit = gui.query().pointer_hit_at(
            after_card.x + after_card.w,
            after_card.y + after_card.h * 0.5,
        );
        assert_eq!(
            gui.query().cursor_for_hit(&right_edge_hit),
            CursorKind::Resize(ResizeEdge::Right)
        );
    }

    #[test]
    fn full_canvas_node_top_left_resize_keeps_card_frame_as_runtime_rect() {
        let mut gui = Context::new();
        let mut workspace = WorkspaceController::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let composition = WorkspaceUiComposition::full();
        let nodes = workspace.canvas_node_render_views(&mut gui, &theme, composition);

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("initial full sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let node_id = format!("canvas_node::{}", showcase_node::SHOWCASE_OWNER_ID);
        let card_id = format!("{node_id}::card");
        let input_column_id = format!("{node_id}::pin_column::input");
        let output_column_id = format!("{node_id}::pin_column::output");
        let before_root = gui.query().node_rect(&node_id).expect("node root rect");
        let before_card = gui.query().node_rect(&card_id).expect("node card rect");
        let before_input = gui
            .query()
            .node_rect(&input_column_id)
            .expect("input pin column rect");
        let before_output = gui
            .query()
            .node_rect(&output_column_id)
            .expect("output pin column rect");

        assert_eq!(before_root, before_card);
        assert!(before_input.x + before_input.w < before_card.x);
        assert!(before_output.x > before_card.x + before_card.w);

        let start_x = before_card.x;
        let start_y = before_card.y;
        let dx = -48.0;
        let dy = -36.0;
        assert!(workspace.start_canvas_node_resize(
            &mut gui,
            &camera,
            &card_id,
            ResizeEdge::TopLeft,
            start_x,
            start_y,
        ));
        assert!(workspace.resize_canvas_node(
            &mut gui,
            &camera,
            &card_id,
            ResizeEdge::TopLeft,
            start_x + dx,
            start_y + dy,
        ));
        assert!(!workspace.end_canvas_node_resize(
            &mut gui,
            &camera,
            &card_id,
            ResizeEdge::TopLeft,
            start_x + dx,
            start_y + dy,
        ));

        let resized_nodes = workspace.canvas_node_render_views(&mut gui, &theme, composition);
        let update = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &resized_nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("resized full sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let after_root = gui.query().node_rect(&node_id).expect("node root rect");
        let after_card = gui.query().node_rect(&card_id).expect("resized node card");
        let after_input = gui
            .query()
            .node_rect(&input_column_id)
            .expect("input pin column rect");
        let after_output = gui
            .query()
            .node_rect(&output_column_id)
            .expect("output pin column rect");

        assert_eq!(update.nodes_added, 0);
        assert_eq!(update.nodes_removed, 0);
        assert!(update.mutations_queued > 0);
        assert_eq!(after_root, after_card);
        assert_eq!(after_card.x, before_card.x + dx);
        assert_eq!(after_card.y, before_card.y + dy);
        assert_eq!(after_card.w, before_card.w - dx);
        assert_eq!(after_card.h, before_card.h - dy);
        assert_eq!(after_card.x + after_card.w, before_card.x + before_card.w);
        assert_eq!(after_card.y + after_card.h, before_card.y + before_card.h);
        assert!(after_input.x + after_input.w < after_card.x);
        assert!(after_output.x > after_card.x + after_card.w);
        assert_eq!(after_input.h, after_card.h);
        assert_eq!(after_output.h, after_card.h);
    }

    #[test]
    fn clean_room_panel_drag_moves_toolbar_root_and_hit_targets() {
        let mut gui = Context::new();
        let mut workspace = WorkspaceController::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let nodes = clean_room_canvas_nodes(&mut workspace, &mut gui, &theme);

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("initial clean room sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let toolbar = gui.query().node_id_by_name("toolbar").expect("toolbar");
        let titlebar = gui
            .query()
            .node_id_by_name("toolbar::titlebar")
            .expect("toolbar titlebar");
        let before_root = gui.query().node_rect("toolbar").expect("toolbar rect");
        let before_titlebar = gui
            .query()
            .node_rect("toolbar::titlebar")
            .expect("toolbar titlebar rect");
        let drag_start_x = before_titlebar.x + before_titlebar.w * 0.5;
        let drag_start_y = before_titlebar.y + before_titlebar.h - 4.0;
        let dx = 260.0;
        let dy = 120.0;
        let titlebar_hit = gui.query().pointer_hit_at(drag_start_x, drag_start_y);
        let resize_hit = gui.query().pointer_hit_at(
            before_root.x + before_root.w,
            before_root.y + before_root.h * 0.5,
        );

        assert!(titlebar_hit.chain().contains(titlebar));
        assert_eq!(gui.query().cursor_for_hit(&titlebar_hit), CursorKind::Move);
        assert_eq!(
            gui.query().cursor_for_hit(&resize_hit),
            CursorKind::Resize(ResizeEdge::Right)
        );

        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::DragStart {
                id: "toolbar".to_string(),
                x: drag_start_x,
                y: drag_start_y,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::DragMove {
                id: "toolbar".to_string(),
                x: drag_start_x + dx,
                y: drag_start_y + dy,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::DragEnd {
                id: "toolbar".to_string(),
                x: drag_start_x + dx,
                y: drag_start_y + dy,
            }));

        let sync = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("drag sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let after_root = gui
            .query()
            .node_rect("toolbar")
            .expect("moved toolbar rect");
        let after_titlebar = gui
            .query()
            .node_rect("toolbar::titlebar")
            .expect("moved toolbar titlebar rect");

        assert!(sync.mutations_queued > 0);
        assert_eq!(after_root.x, before_root.x + dx);
        assert_eq!(after_root.y, before_root.y + dy);
        assert_eq!(after_titlebar.x, before_titlebar.x + dx);
        assert_eq!(after_titlebar.y, before_titlebar.y + dy);
        assert!(gui
            .query()
            .hit_test(after_titlebar.x + 12.0, after_titlebar.y + 10.0)
            .contains(titlebar));
        assert!(!gui
            .query()
            .hit_test(before_titlebar.x + 12.0, before_titlebar.y + 10.0)
            .contains(toolbar));

        let second_drag_start_x = after_titlebar.x + 12.0;
        let second_drag_start_y = after_titlebar.y + 10.0;
        let second_dx = -90.0;
        let second_dy = 44.0;
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::DragStart {
                id: "toolbar".to_string(),
                x: second_drag_start_x,
                y: second_drag_start_y,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::DragMove {
                id: "toolbar".to_string(),
                x: second_drag_start_x + second_dx,
                y: second_drag_start_y + second_dy,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::DragEnd {
                id: "toolbar".to_string(),
                x: second_drag_start_x + second_dx,
                y: second_drag_start_y + second_dy,
            }));

        let second_sync = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("second drag sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let final_root = gui
            .query()
            .node_rect("toolbar")
            .expect("final toolbar rect");
        let final_titlebar = gui
            .query()
            .node_rect("toolbar::titlebar")
            .expect("final toolbar titlebar rect");

        assert!(second_sync.mutations_queued > 0);
        assert_eq!(final_root.x, after_root.x + second_dx);
        assert_eq!(final_root.y, after_root.y + second_dy);
        assert_eq!(final_titlebar.x, after_titlebar.x + second_dx);
        assert_eq!(final_titlebar.y, after_titlebar.y + second_dy);
        assert!(gui
            .query()
            .hit_test(final_titlebar.x + 12.0, final_titlebar.y + 10.0)
            .contains(titlebar));
        assert!(!gui
            .query()
            .hit_test(after_titlebar.x + 12.0, after_titlebar.y + 10.0)
            .contains(toolbar));
        assert!(gui.query().node_exists("preview"));
        assert!(!gui.query().node_exists("engine"));
        assert!(!gui.query().node_exists("node_palette"));
    }

    #[test]
    fn clean_room_panel_top_left_resize_clamps_and_updates_hit_targets() {
        let mut gui = Context::new();
        let mut workspace = WorkspaceController::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let nodes = clean_room_canvas_nodes(&mut workspace, &mut gui, &theme);

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("initial clean room sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let toolbar = gui.query().node_id_by_name("toolbar").expect("toolbar");
        let before_root = gui.query().node_rect("toolbar").expect("toolbar rect");
        let start_x = before_root.x + 2.0;
        let start_y = before_root.y + 2.0;
        let before_hit = gui.query().pointer_hit_at(start_x, start_y);

        assert!(before_hit.chain().contains(toolbar));
        assert_eq!(
            gui.query().cursor_for_hit(&before_hit),
            CursorKind::Resize(ResizeEdge::TopLeft)
        );

        let end_x = start_x + 80.0;
        let end_y = start_y + 40.0;
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::ResizeStart {
                id: "toolbar".to_string(),
                edge: ResizeEdge::TopLeft,
                x: start_x,
                y: start_y,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::ResizeMove {
                id: "toolbar".to_string(),
                edge: ResizeEdge::TopLeft,
                x: end_x,
                y: end_y,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::ResizeEnd {
                id: "toolbar".to_string(),
                edge: ResizeEdge::TopLeft,
                x: end_x,
                y: end_y,
            }));

        let sync = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("resize sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let after_root = gui.query().node_rect("toolbar").expect("toolbar rect");
        let expected_right = before_root.x + before_root.w;
        let expected_bottom = before_root.y + before_root.h;
        let toolbar_min_size = registered_panels(PanelRenderInput {
            preview_image: TextureHandle(1),
            engine_panel: &engine_panel,
        })
        .into_iter()
        .find(|panel| panel.config.id.as_str() == "toolbar")
        .expect("toolbar panel should be registered")
        .runtime_config(&theme)
        .min_size;

        assert!(sync.mutations_queued > 0);
        assert_eq!(after_root.w, toolbar_min_size[0]);
        assert_eq!(after_root.h, toolbar_min_size[1]);
        assert_eq!(after_root.x + after_root.w, expected_right);
        assert_eq!(after_root.y + after_root.h, expected_bottom);

        let after_hit = gui
            .query()
            .pointer_hit_at(after_root.x + 2.0, after_root.y + 2.0);
        assert!(after_hit.chain().contains(toolbar));
        assert_eq!(
            gui.query().cursor_for_hit(&after_hit),
            CursorKind::Resize(ResizeEdge::TopLeft)
        );
        assert!(!gui
            .query()
            .hit_test(before_root.x + 2.0, before_root.y + 2.0)
            .contains(toolbar));

        let expand_start_x = after_root.x + 2.0;
        let expand_start_y = after_root.y + 2.0;
        let expand_end_x = expand_start_x - 40.0;
        let expand_end_y = expand_start_y - 30.0;
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::ResizeStart {
                id: "toolbar".to_string(),
                edge: ResizeEdge::TopLeft,
                x: expand_start_x,
                y: expand_start_y,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::ResizeMove {
                id: "toolbar".to_string(),
                edge: ResizeEdge::TopLeft,
                x: expand_end_x,
                y: expand_end_y,
            }));
        assert!(gui
            .panel_mut()
            .handle_control_event(&ControlEvent::ResizeEnd {
                id: "toolbar".to_string(),
                edge: ResizeEdge::TopLeft,
                x: expand_end_x,
                y: expand_end_y,
            }));

        let expand_sync = controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("expand resize sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        let expanded_root = gui.query().node_rect("toolbar").expect("toolbar rect");
        let expected_expand_right = after_root.x + after_root.w;
        let expected_expand_bottom = after_root.y + after_root.h;

        assert!(expand_sync.mutations_queued > 0);
        assert_eq!(expanded_root.x, after_root.x - 40.0);
        assert_eq!(expanded_root.y, after_root.y - 30.0);
        assert_eq!(expanded_root.w, after_root.w + 40.0);
        assert_eq!(expanded_root.h, after_root.h + 30.0);
        assert_eq!(expanded_root.x + expanded_root.w, expected_expand_right);
        assert_eq!(expanded_root.y + expanded_root.h, expected_expand_bottom);
    }

    #[test]
    fn switch_full_to_clean_room_unmounts_stale_scene_elements() {
        let mut gui = Context::new();
        let mut controller = WorkspaceSceneController::default();
        let camera = Camera::new();
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let palette = NodePaletteState {
            items: vec![NodePaletteItem {
                type_id: "image_gen".to_string(),
                name: "Image Generator".to_string(),
                category: "Generators".to_string(),
                source: "built-in".to_string(),
            }],
        };
        let full_identity = showcase_node::solo_node_identity();
        let showcase_state = ShowcaseState::default();
        let full_node = showcase_node::showcase_render_view_for_layout(
            CanvasNodeLayout {
                owner_id: full_identity.owner_id.clone(),
                rect: full_identity.default_rect,
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            &showcase_state,
        )
        .expect("showcase node");
        let full_nodes = vec![full_node];
        controller.open_node_palette(40.0, 50.0, palette);
        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &full_nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("full sync");
        assert!(gui
            .query()
            .node_exists("canvas_node::showcase_node::solo_control"));
        assert!(gui.query().node_exists("toolbar"));
        assert!(gui.query().node_exists("node_palette"));

        let mut clean_workspace = WorkspaceController::new();
        let clean_nodes = clean_room_canvas_nodes(&mut clean_workspace, &mut gui, &theme);
        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &clean_nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::clean_room(),
                },
            )
            .expect("clean room sync");

        assert!(gui.query().node_exists("canvas_node::engine_node::0"));
        assert!(!gui
            .query()
            .node_exists("canvas_node::showcase_node::solo_control"));
        assert!(gui.query().node_exists("toolbar"));
        assert!(gui.query().node_exists("preview"));
        assert!(!gui.query().node_exists("engine"));
        assert!(!gui.query().node_exists("node_palette"));
        assert!(!controller.node_palette_open());
    }

    #[test]
    fn retained_text_area_under_canvas_transform_hits_field_and_accepts_text() {
        let mut gui = Context::new();
        let mut controller = WorkspaceSceneController::default();
        let mut camera = Camera::new();
        camera.x = 1_560.0;
        camera.y = -100.0;
        camera.zoom = 1.0;
        let theme = light_theme();
        let engine_panel = empty_engine_panel();
        let identity = showcase_node::text_area_node_identity();
        let mut showcase_state = ShowcaseState::default();
        showcase_state.update_control_value(
            "canvas_node::showcase_node::text_area_control::body::param::Prompt::control::content",
            ControlValue::Text("hello".to_string()),
        );
        let view = showcase_node::showcase_render_view_for_layout(
            CanvasNodeLayout {
                owner_id: identity.owner_id.clone(),
                rect: identity.default_rect,
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            &showcase_state,
        )
        .expect("text area view");
        let nodes = vec![view];
        let mut measurer = TextMeasurer::new();

        controller
            .sync(
                &mut gui,
                WorkspaceSceneInput {
                    viewport: viewport(),
                    camera: &camera,
                    canvas_nodes: &nodes,
                    canvas_connections: &[],
                    pending_connection: None,
                    theme: &theme,
                    preview_image: TextureHandle(1),
                    engine_panel: &engine_panel,
                    features: WorkspaceSceneFeatures::full(),
                },
            )
            .expect("sync");
        gui.rendering()
            .flush_layout_dirty(viewport(), &mut measurer);
        let text_box_sync_items = canvas_text_box_sync_items(&nodes);
        gui.controls_mut()
            .sync_text_boxes(&text_box_sync_items, &mut measurer, &theme);

        let control_id =
            "canvas_node::showcase_node::text_area_control::body::param::Prompt::control::content";
        let field_id = format!("{control_id}::field");
        let field = gui.query().node_rect(&field_id).expect("field rect");
        let (x, y) = camera.canvas_to_screen(field.x + 8.0, field.y + 8.0);
        let query = gui.query();
        let chain = query.hit_test(x, y);
        assert_eq!(
            chain.leaf().and_then(|node| query.node_name(node)),
            Some(field_id.as_str())
        );

        let click = gui.input().handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        assert!(click.consumed);
        let _ = gui.input().handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = gui.input().handle_event(&AppEvent::KeyPress {
            key: Key::End,
            modifiers: Modifiers::default(),
        });
        let input = gui.input().handle_event(&AppEvent::TextInput {
            text: "!".to_string(),
        });

        assert!(input.events.iter().any(|event| matches!(
            event,
            GuiEvent::Control(ControlEvent::ValueChanged {
                id,
                value: ControlValue::Text(value)
            }) if id == control_id && value == "hello!"
        )));
    }

    fn empty_engine_panel() -> EnginePanelState {
        EnginePanelState {
            node_count: 0,
            connection_count: 0,
            node_def_count: 0,
            graph_version: 0,
            dirty: false,
            execution_status: "Idle".to_string(),
            last_action: "Ready".to_string(),
        }
    }
}
