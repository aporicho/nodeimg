use gui::canvas::camera::Camera;
use gui::canvas::node_template::CanvasNodeRenderView;
use gui::canvas::scene_diff::{scene_change_to_mutation, CanvasSceneChange};
use gui::canvas::scene_model::CanvasSceneModel;
use gui::canvas::{canvas_node_stable_id, CanvasConnectionView, CanvasPendingConnectionView};
use gui::context::Context;
use gui::diagnostics::render_trace::{self, RectSummary, RenderTraceStage};
use gui::geometry::TransformSpec;
use gui::panel::retained::{PanelContentTemplate, PanelFrameTemplateData};
use gui::panel::{PanelConfig, PanelId};
use gui::renderer::{Border, ImageStyle, Point, Rect};
use gui::template::{
    InstanceId, SlotValue, SlotValues, TemplateId, TemplatePayload, NODE_PALETTE_CATEGORY_TEMPLATE,
    NODE_PALETTE_EMPTY_TEMPLATE, NODE_PALETTE_ITEM_TEMPLATE, NODE_PALETTE_TEMPLATE,
    PANEL_FRAME_TEMPLATE,
};
use gui::theme::Theme;
use gui::tree::layout::{Decoration, TextureHandle};
use gui::tree::{MutationError, StylePatch, TreeMutation};
use gui::widget::mapping::ParamControlSpec;
use std::collections::BTreeSet;

use crate::image_demo::{ADD_IMAGE_DEMO_GRAPH_ID, RUN_IMAGE_DEMO_ID};
use crate::panels::EnginePanelState;

use super::node_palette::{node_palette_item_id, NodePaletteState};

const GRID_SPACING: f32 = 20.0;

#[derive(Clone, Debug, Default)]
pub(crate) struct WorkspaceSceneController {
    canvas: CanvasSceneModel,
    node_palette: Option<NodePaletteOverlay>,
    node_palette_children: BTreeSet<String>,
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
    pub(crate) mutations_applied: usize,
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
}

impl WorkspaceSceneController {
    pub(crate) fn sync(
        &mut self,
        gui: &mut Context,
        input: WorkspaceSceneInput<'_>,
    ) -> Result<SceneSyncStats, MutationError> {
        let roots = gui.ensure_retained_root(input.viewport)?;
        gui.update_canvas_transform(TransformSpec::translate_scale(
            [input.camera.x, input.camera.y],
            input.camera.zoom,
        ))?;

        let mut stats = SceneSyncStats::default();
        stats.mutations_applied +=
            self.ensure_node_palette_root(gui, roots.overlay_root, input.viewport)?;
        let mut mutations = Vec::new();
        mutations.push(TreeMutation::SetRect {
            node: roots.canvas_grid,
            rect: canvas_grid_rect(input.viewport, input.camera),
        });

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
            input.theme,
            input.preview_image,
            input.engine_panel,
            &mut mutations,
        );
        self.sync_node_palette(input.viewport, gui, &mut mutations);

        stats.mutations_applied += mutations.len();
        render_trace::debug_stage(
            RenderTraceStage::SceneSync,
            SceneSyncTraceSummary {
                viewport: RectSummary::from(input.viewport),
                desired_nodes: input.canvas_nodes.len(),
                desired_connections: input.canvas_connections.len(),
                pending_connection: input.pending_connection.is_some(),
                mutations_queued: mutations.len(),
                stats,
            },
        );
        gui.apply_mutations(mutations)?;
        for view in input.canvas_nodes {
            let owner_id = view.state.owner_id.as_str();
            let stable_id = canvas_node_stable_id(owner_id);
            if let Some(root) = gui.node_id_by_name(&stable_id) {
                self.canvas.insert_node(owner_id.to_string(), root);
            }
        }
        Ok(stats)
    }

    fn sync_panels(
        &mut self,
        parent: usize,
        gui: &mut Context,
        theme: &Theme,
        preview_image: TextureHandle,
        engine: &EnginePanelState,
        mutations: &mut Vec<TreeMutation>,
    ) {
        for panel in retained_panels(preview_image, engine) {
            let Some(runtime) = gui.ensure_panel_runtime(&panel.config) else {
                continue;
            };
            let id = panel.config.id.as_str();
            let root = gui.node_id_by_name(id);
            if !runtime.visible {
                if let Some(root) = root {
                    mutations.push(TreeMutation::Unmount { node: root });
                }
                continue;
            }
            if let Some(root) = root {
                mutations.push(TreeMutation::SetRect {
                    node: root,
                    rect: runtime.rect,
                });
                mutations.push(TreeMutation::SetZIndex {
                    node: root,
                    z_index: runtime.z_index,
                });
                if let PanelContentTemplate::Engine {
                    status,
                    catalog,
                    last_action,
                    ..
                } = &panel.content
                {
                    if let Some(node) = gui.node_id_by_name("engine_status") {
                        mutations.push(TreeMutation::SetText {
                            node,
                            value: status.clone(),
                        });
                    }
                    if let Some(node) = gui.node_id_by_name("engine_catalog") {
                        mutations.push(TreeMutation::SetText {
                            node,
                            value: catalog.clone(),
                        });
                    }
                    if let Some(node) = gui.node_id_by_name("engine_last_action") {
                        mutations.push(TreeMutation::SetText {
                            node,
                            value: last_action.clone(),
                        });
                    }
                }
                continue;
            }

            mutations.push(TreeMutation::MountTemplate {
                parent,
                template: TemplateId::from(PANEL_FRAME_TEMPLATE),
                instance: InstanceId::from(id.to_string()),
                payload: TemplatePayload::PanelFrame(PanelFrameTemplateData::new(
                    panel.config,
                    runtime,
                    panel.content,
                    theme,
                )),
            });
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
        mutations: &mut Vec<TreeMutation>,
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
            mutations.push(TreeMutation::Unmount { node: root });
            stats.nodes_removed += 1;
        }

        for view in views {
            let owner_id = view.state.owner_id.as_str();
            let stable_id = canvas_node_stable_id(owner_id);
            if self.canvas.node(owner_id).is_none() {
                mutations.push(scene_change_to_mutation(CanvasSceneChange::AddNodeCard {
                    parent: canvas_root,
                    view: view.clone(),
                    theme: theme.clone(),
                }));
                stats.nodes_added += 1;
                continue;
            }

            if let Some(root) = gui.node_id_by_name(&stable_id) {
                mutations.push(TreeMutation::SetRect {
                    node: root,
                    rect: view.state.layout.rect,
                });
                mutations.push(TreeMutation::SetZIndex {
                    node: root,
                    z_index: view.state.layout.z_index,
                });
            }
            if let Some(label) = gui.node_id_by_name(&format!("{stable_id}::label_text")) {
                mutations.push(TreeMutation::SetText {
                    node: label,
                    value: view.template.title.clone(),
                });
            }
            if let Some(card) = gui.node_id_by_name(&format!("{stable_id}::card")) {
                mutations.push(TreeMutation::SetStyle {
                    node: card,
                    patch: StylePatch {
                        decoration: Some(Some(Decoration {
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
                        })),
                        ..StylePatch::default()
                    },
                });
            }
            sync_canvas_node_param_texts(&stable_id, view, gui, mutations);
        }
    }

    fn sync_connections(
        &mut self,
        parent: usize,
        connections: &[CanvasConnectionView],
        pending: Option<&CanvasPendingConnectionView>,
        gui: &Context,
        mutations: &mut Vec<TreeMutation>,
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
            if let Some(root) = gui.node_id_by_name(&id) {
                mutations.push(scene_change_to_mutation(
                    CanvasSceneChange::RemoveConnection { root },
                ));
                stats.connections_removed += 1;
            }
        }

        for (index, connection) in connections.iter().enumerate() {
            let id = format!("canvas_connection::{index}");
            if self.canvas.has_connection(&id) {
                if let Some(node) = gui.node_id_by_name(&id) {
                    mutations.push(TreeMutation::SetConnection {
                        node,
                        from_port: connection.from_port_id.clone(),
                        to_port: connection.to_port_id.clone(),
                    });
                }
            } else {
                self.canvas.insert_connection(id.clone());
                mutations.push(scene_change_to_mutation(CanvasSceneChange::AddConnection {
                    parent,
                    id,
                    from_port: connection.from_port_id.clone(),
                    to_port: connection.to_port_id.clone(),
                }));
                stats.connections_added += 1;
            }
        }

        let pending_id = "canvas_connection::pending";
        match (pending, gui.node_id_by_name(pending_id)) {
            (Some(pending), Some(node)) => {
                mutations.push(scene_change_to_mutation(
                    CanvasSceneChange::UpdatePendingConnection {
                        node,
                        from_port: pending.from_port_id.clone(),
                        cursor_canvas: Point {
                            x: pending.cursor_canvas[0],
                            y: pending.cursor_canvas[1],
                        },
                    },
                ));
            }
            (Some(pending), None) => {
                mutations.push(scene_change_to_mutation(
                    CanvasSceneChange::AddPendingConnection {
                        parent,
                        from_port: pending.from_port_id.clone(),
                        cursor_canvas: Point {
                            x: pending.cursor_canvas[0],
                            y: pending.cursor_canvas[1],
                        },
                    },
                ));
            }
            (None, Some(root)) => {
                mutations.push(scene_change_to_mutation(
                    CanvasSceneChange::RemovePendingConnection { root },
                ));
            }
            (None, None) => {}
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
        if gui.node_id_by_name("node_palette").is_some() {
            return Ok(0);
        }

        gui.apply_mutation(TreeMutation::MountTemplate {
            parent,
            template: TemplateId::from(NODE_PALETTE_TEMPLATE),
            instance: InstanceId::from("node_palette"),
            payload: TemplatePayload::from(SlotValues::new().with(
                "rect",
                SlotValue::Rect(node_palette_rect(overlay.x, overlay.y, viewport)),
            )),
        })?;
        Ok(1)
    }

    fn sync_node_palette(
        &mut self,
        viewport: Rect,
        gui: &Context,
        mutations: &mut Vec<TreeMutation>,
    ) {
        let root = gui.node_id_by_name("node_palette");
        let Some(overlay) = &self.node_palette else {
            if let Some(root) = root {
                mutations.push(TreeMutation::Unmount { node: root });
            }
            self.node_palette_children.clear();
            return;
        };
        let Some(root) = root else {
            return;
        };
        let Some(items_parent) = gui.node_id_by_name("node_palette::items") else {
            return;
        };

        mutations.push(TreeMutation::SetRect {
            node: root,
            rect: node_palette_rect(overlay.x, overlay.y, viewport),
        });

        let mut desired = BTreeSet::new();
        if overlay.state.items.is_empty() {
            let id = "node_palette_empty".to_string();
            desired.insert(id.clone());
            if gui.node_id_by_name(&id).is_none() {
                mutations.push(TreeMutation::MountTemplate {
                    parent: items_parent,
                    template: TemplateId::from(NODE_PALETTE_EMPTY_TEMPLATE),
                    instance: InstanceId::from(id),
                    payload: TemplatePayload::from(
                        SlotValues::new()
                            .with("label", SlotValue::Text("No nodes available".to_string())),
                    ),
                });
            }
        } else {
            let mut seen_categories = BTreeSet::new();
            for item in &overlay.state.items {
                if seen_categories.insert(item.category.as_str()) {
                    let category_id = format!("node_palette::category::{}", item.category);
                    desired.insert(category_id.clone());
                    if gui.node_id_by_name(&category_id).is_none() {
                        mutations.push(TreeMutation::MountTemplate {
                            parent: items_parent,
                            template: TemplateId::from(NODE_PALETTE_CATEGORY_TEMPLATE),
                            instance: InstanceId::from(category_id),
                            payload: TemplatePayload::from(
                                SlotValues::new()
                                    .with("label", SlotValue::Text(item.category.clone())),
                            ),
                        });
                    } else if let Some(label) =
                        gui.node_id_by_name(&format!("{category_id}::label"))
                    {
                        mutations.push(TreeMutation::SetText {
                            node: label,
                            value: item.category.clone(),
                        });
                    }
                }

                let item_id = node_palette_item_id(&item.type_id);
                desired.insert(item_id.clone());
                let label = format!("{}  [{}]", item.name, item.source);
                if gui.node_id_by_name(&item_id).is_none() {
                    mutations.push(TreeMutation::MountTemplate {
                        parent: items_parent,
                        template: TemplateId::from(NODE_PALETTE_ITEM_TEMPLATE),
                        instance: InstanceId::from(item_id),
                        payload: TemplatePayload::from(
                            SlotValues::new().with("label", SlotValue::Text(label)),
                        ),
                    });
                } else if let Some(label_node) = gui.node_id_by_name(&format!("{item_id}::label")) {
                    mutations.push(TreeMutation::SetText {
                        node: label_node,
                        value: label,
                    });
                }
            }
        }

        for id in self
            .node_palette_children
            .difference(&desired)
            .cloned()
            .collect::<Vec<_>>()
        {
            if let Some(node) = gui.node_id_by_name(&id) {
                mutations.push(TreeMutation::Unmount { node });
            }
        }
        self.node_palette_children = desired;
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct SceneSyncTraceSummary {
    viewport: RectSummary,
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

fn sync_canvas_node_param_texts(
    stable_id: &str,
    view: &CanvasNodeRenderView,
    gui: &Context,
    mutations: &mut Vec<TreeMutation>,
) {
    for (index, param) in view.template.params.iter().enumerate() {
        let widget_id = format!("{stable_id}::body::param::{index}::control::widget");
        let (target_id, value) = match &param.control {
            ParamControlSpec::Text { value } | ParamControlSpec::TextArea { value, .. } => {
                (format!("{widget_id}::value"), value.clone())
            }
            ParamControlSpec::Number {
                value, precision, ..
            } => (
                format!("{widget_id}::value"),
                format!("{value:.precision$}"),
            ),
            ParamControlSpec::ReadOnly { value } => (widget_id, value.clone()),
            ParamControlSpec::Select { options, selected } => (
                widget_id,
                options.get(*selected).cloned().unwrap_or_default(),
            ),
            ParamControlSpec::FilePath { path, .. } => (widget_id, path.clone()),
            ParamControlSpec::Color { rgba } => (
                format!("{widget_id}::value"),
                format!(
                    "#{:02X}{:02X}{:02X}",
                    (rgba[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (rgba[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (rgba[2].clamp(0.0, 1.0) * 255.0) as u8
                ),
            ),
            ParamControlSpec::Slider { .. } | ParamControlSpec::Toggle { .. } => continue,
        };
        if let Some(node) = gui.node_id_by_name(&target_id) {
            mutations.push(TreeMutation::SetText { node, value });
        }
    }
}

struct RetainedPanelSpec {
    config: PanelConfig,
    content: PanelContentTemplate,
}

fn retained_panels(
    preview_image: TextureHandle,
    engine: &EnginePanelState,
) -> Vec<RetainedPanelSpec> {
    vec![
        RetainedPanelSpec {
            config: PanelConfig {
                id: PanelId::new("toolbar"),
                title: std::borrow::Cow::Borrowed("Toolbar"),
                default_rect: Rect {
                    x: 28.0,
                    y: 28.0,
                    w: 180.0,
                    h: 82.0,
                },
                min_size: [156.0, 72.0],
                titlebar_visible: true,
                draggable: true,
                resizable: true,
                closable: false,
                initially_visible: true,
            },
            content: PanelContentTemplate::Toolbar {
                add_graph_id: ADD_IMAGE_DEMO_GRAPH_ID.to_string(),
                run_graph_id: RUN_IMAGE_DEMO_ID.to_string(),
            },
        },
        RetainedPanelSpec {
            config: PanelConfig {
                id: PanelId::new("preview"),
                title: std::borrow::Cow::Borrowed("Preview"),
                default_rect: Rect {
                    x: 180.0,
                    y: 28.0,
                    w: 360.0,
                    h: 224.0,
                },
                min_size: [260.0, 180.0],
                titlebar_visible: true,
                draggable: true,
                resizable: true,
                closable: false,
                initially_visible: true,
            },
            content: PanelContentTemplate::Preview {
                image_id: "preview_image".to_string(),
                texture: preview_image,
                image_style: ImageStyle::default(),
            },
        },
        RetainedPanelSpec {
            config: PanelConfig {
                id: PanelId::new("engine"),
                title: std::borrow::Cow::Borrowed("Engine"),
                default_rect: Rect {
                    x: 28.0,
                    y: 96.0,
                    w: 300.0,
                    h: 150.0,
                },
                min_size: [260.0, 132.0],
                titlebar_visible: true,
                draggable: true,
                resizable: true,
                closable: false,
                initially_visible: true,
            },
            content: PanelContentTemplate::Engine {
                group_id: "engine_status_group".to_string(),
                status: format!(
                    "Status: {} | Nodes: {} | Connections: {}",
                    engine.execution_status, engine.node_count, engine.connection_count
                ),
                catalog: format!(
                    "Catalog: {} definitions | Graph v{}{}",
                    engine.node_def_count,
                    engine.graph_version,
                    if engine.dirty { " *" } else { "" }
                ),
                last_action: format!("Last: {}", engine.last_action),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::node_palette::NodePaletteItem;
    use gui::renderer::TextMeasurer;
    use gui::theme::light_theme;
    use gui::tree::layout::TextureHandle;

    fn viewport() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: 900.0,
            h: 700.0,
        }
    }

    #[test]
    fn node_palette_open_mounts_retained_templates_without_widget_builds() {
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
                },
            )
            .expect("sync");
        gui.flush_layout_dirty(viewport(), &mut TextMeasurer::new());

        assert!(gui.node_exists("node_palette"));
        assert!(gui.node_exists("node_palette::category::Generators"));
        assert!(gui.node_exists("node_library::add::image_gen"));
        assert_eq!(gui.last_frame_stats().widget_build_calls, 0);
        assert_eq!(gui.last_frame_stats().full_tree_scans, 0);
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
                },
            )
            .expect("close sync");

        assert!(!gui.node_exists("node_palette"));
        assert!(!gui.node_exists("node_palette_empty"));
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
