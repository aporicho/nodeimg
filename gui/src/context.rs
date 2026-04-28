use std::sync::Arc;
use std::time::Instant;

use crate::animation::{AnimationBuilder, AnimationId, AnimationStore, TimelineBuilder};
use crate::diagnostics::render_trace::{self, RectSummary, RenderTraceStage, TARGET_RENDER};
use crate::event::gesture_adapter;
use crate::event::router;
use crate::gesture::{Gesture, GestureSession, GestureSessionUpdate};
use crate::icon::{IconId, IconRegistry};
use crate::interaction::InteractionState;
use crate::paint::{DisplayList, PaintFlushStats};
#[cfg(test)]
use crate::panel::PanelDeclaration;
use crate::renderer::{Rect, RegistryDisplayResources, Renderer, TextMeasurer, TextureSize};
#[cfg(test)]
use crate::runtime::RuntimeSyncCx;
use crate::runtime::{ResourceRegistry, RuntimeEventCx, RuntimeEventResult, RuntimeSystems};
use crate::shell::AppEvent;
use crate::template::TemplateRegistry;
use crate::template::{InstanceId, SlotValue, SlotValues, TemplateId, WORKSPACE_ROOT_TEMPLATE};
use crate::theme::Theme;
#[cfg(test)]
use crate::tree::build_display_list;
use crate::tree::layout::{LayoutConstraints, LayoutFlushStats, LayoutOutput, TextureHandle};
#[cfg(test)]
use crate::tree::Desc;
use crate::tree::{
    hit_test_with_animations, resize_hit_at_screen_point, FrameStats, HitChain, Invalidation,
    MutationError, NodeId, NodeKind, PaintDirtyReason, RepaintBoundaryId, StylePatch, Tree,
    TreeMutation,
};
use crate::widget::resize_edge::ResizeEdge;

pub use crate::output::{
    FrameworkOutput, GuiEvent, OverlayEvent, PanelEvent, PlatformEffect, WidgetEvent,
};
pub use crate::overlay::{OverlayPlacement, OverlayRequest};

/// GUI 中心对象。持有统一的控件树与框架级交互 session。
pub struct Context {
    pub(crate) tree: Tree,
    pub(crate) template_registry: TemplateRegistry,
    animations: AnimationStore,
    gesture_session: GestureSession,
    interaction: InteractionState,
    pub(crate) systems: RuntimeSystems,
    #[cfg(test)]
    pub(crate) last_theme_revision: Option<u64>,
    pub(crate) resources: ResourceRegistry,
    icons: IconRegistry,
    retained_display_list: Option<DisplayList>,
    last_paint_theme_revision: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImeRequest {
    pub allowed: bool,
    pub cursor_area: Option<Rect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedRootIds {
    pub root: NodeId,
    pub canvas_root: NodeId,
    pub canvas_grid: NodeId,
    pub canvas_connections: NodeId,
    pub panel_root: NodeId,
    pub overlay_root: NodeId,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct LayoutFlushTraceSummary {
    root_rect: RectSummary,
    dirty_boundaries: usize,
    dirty_text_nodes: usize,
    boundary_candidates: usize,
    stats: LayoutFlushStats,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct LayoutBoundaryTraceSummary {
    available: Option<RectSummary>,
    cache_hit: bool,
    skipped: &'static str,
    nodes_visited: usize,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct PaintDirtyTraceSummary {
    root: NodeId,
    root_boundary: NodeId,
    dirty_boundaries: usize,
    dirty_paint_order: usize,
    dirty_composite: usize,
    rebuild_boundaries: usize,
    retained_display_list_present: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct PaintComposeTraceSummary {
    fragments_flattened: usize,
    display_commands: usize,
    clips: usize,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct TextRuntimeTraceSummary {
    views: usize,
    dirty_intrinsics_before: usize,
    dirty_intrinsics_after: usize,
}

impl Context {
    pub fn new() -> Self {
        Self {
            tree: Tree::new(),
            template_registry: TemplateRegistry::with_builtin_templates(),
            animations: AnimationStore::new(),
            gesture_session: GestureSession::new(),
            interaction: InteractionState::new(),
            systems: RuntimeSystems::new(),
            #[cfg(test)]
            last_theme_revision: None,
            resources: ResourceRegistry::new(),
            icons: IconRegistry::with_builtin_icons(),
            retained_display_list: None,
            last_paint_theme_revision: None,
        }
    }

    /// Legacy/prototype full-tree `Desc` update.
    ///
    /// Test-only compatibility boundary. Production app code must use retained
    /// templates plus `TreeMutation` through scene controllers.
    #[cfg(test)]
    pub fn update(
        &mut self,
        desc: Desc,
        root_rect: Rect,
        measurer: &mut TextMeasurer,
        theme: &Theme,
    ) {
        let desc = self.systems.compose_desc(&self.tree, desc, root_rect);
        let force_rebuild = self.last_theme_revision != Some(theme.revision);
        crate::tree::legacy_desc::update_tree_from_desc(
            &mut self.tree,
            desc,
            root_rect,
            measurer,
            theme,
            force_rebuild,
        );
        self.last_theme_revision = Some(theme.revision);
        self.interaction.sync_with_tree(&self.tree);
        self.systems.sync_with_tree(RuntimeSyncCx {
            tree: &mut self.tree,
            interaction: &self.interaction,
            measurer,
            theme,
        });
    }

    pub fn flush_layout_dirty(
        &mut self,
        root_rect: Rect,
        measurer: &mut TextMeasurer,
    ) -> LayoutFlushStats {
        let dirty = self.tree.take_layout_dirty();
        let dirty_boundaries = dirty.boundaries.len();
        let dirty_text_nodes = dirty.text_nodes.len();
        let mut boundaries = dirty.boundaries;
        for text_node in dirty.text_nodes {
            if let Some(boundary) = self.tree.nearest_relayout_boundary(text_node) {
                boundaries.insert(boundary);
            }
        }

        let mut stats = LayoutFlushStats::default();
        let boundary_candidates = boundaries.len();
        for boundary in boundaries {
            let Some(available) = boundary_available_rect(&self.tree, boundary, root_rect) else {
                render_trace::trace_node(
                    RenderTraceStage::LayoutFlush,
                    boundary,
                    self.tree.get(boundary).map(|node| node.id.as_ref()),
                    LayoutBoundaryTraceSummary {
                        available: None,
                        cache_hit: false,
                        skipped: "missing available rect",
                        nodes_visited: 0,
                    },
                );
                continue;
            };
            let constraints = LayoutConstraints::from_available(available);
            let Some(input) = self.tree.layout_input_for(boundary, constraints) else {
                render_trace::trace_node(
                    RenderTraceStage::LayoutFlush,
                    boundary,
                    self.tree.get(boundary).map(|node| node.id.as_ref()),
                    LayoutBoundaryTraceSummary {
                        available: Some(RectSummary::from(available)),
                        cache_hit: false,
                        skipped: "missing layout input",
                        nodes_visited: 0,
                    },
                );
                continue;
            };
            if self.tree.layout_cache_get(input).is_some() {
                self.tree.record_layout_cache_hit();
                stats.boundaries_skipped_cache_hit += 1;
                render_trace::trace_node(
                    RenderTraceStage::LayoutFlush,
                    boundary,
                    self.tree.get(boundary).map(|node| node.id.as_ref()),
                    LayoutBoundaryTraceSummary {
                        available: Some(RectSummary::from(available)),
                        cache_hit: true,
                        skipped: "cache hit",
                        nodes_visited: 0,
                    },
                );
                continue;
            }

            self.tree.record_layout_cache_miss();
            let visited_before = self.tree.frame_stats_snapshot().layout_nodes_visited;
            crate::tree::layout(&mut self.tree, boundary, available, &mut |text, style| {
                measurer.measure_with_style(text, style)
            });
            let visited_after = self.tree.frame_stats_snapshot().layout_nodes_visited;
            self.tree.record_layout_boundary_flushed();
            stats.boundaries_flushed += 1;
            let nodes_visited = visited_after.saturating_sub(visited_before);
            stats.nodes_visited += nodes_visited;
            if let Some(output) = layout_output_from_tree(&self.tree, boundary) {
                self.tree.layout_cache_set(input, output);
            }
            render_trace::trace_node(
                RenderTraceStage::LayoutFlush,
                boundary,
                self.tree.get(boundary).map(|node| node.id.as_ref()),
                LayoutBoundaryTraceSummary {
                    available: Some(RectSummary::from(available)),
                    cache_hit: false,
                    skipped: "",
                    nodes_visited,
                },
            );
        }
        render_trace::debug_stage(
            RenderTraceStage::LayoutFlush,
            LayoutFlushTraceSummary {
                root_rect: RectSummary::from(root_rect),
                dirty_boundaries,
                dirty_text_nodes,
                boundary_candidates,
                stats: stats.clone(),
            },
        );
        stats
    }

    pub fn ensure_retained_root(
        &mut self,
        viewport: Rect,
    ) -> Result<RetainedRootIds, MutationError> {
        if self.tree.node_by_str("root").is_none() {
            self.template_registry.instantiate_root(
                &mut self.tree,
                TemplateId::from(WORKSPACE_ROOT_TEMPLATE),
                InstanceId::from("root"),
                retained_root_slots(viewport),
            )?;
            self.tree.mark_dirty(
                self.tree.root().expect("retained root must be set"),
                crate::tree::DirtyFlags::STRUCTURE
                    | crate::tree::DirtyFlags::LAYOUT
                    | crate::tree::DirtyFlags::HIT
                    | crate::tree::DirtyFlags::PAINT,
            );
        } else {
            let root = self.tree.node_by_str("root").expect("root");
            let canvas = self.tree.node_by_str("canvas_root").expect("canvas root");
            let grid = self.tree.node_by_str("canvas_grid").expect("canvas grid");
            let panel = self.tree.node_by_str("panel_root").expect("panel root");
            let overlay = self
                .tree
                .node_by_str("__overlay_root")
                .expect("overlay root");
            for mutation in [
                TreeMutation::SetRect {
                    node: root,
                    rect: viewport,
                },
                TreeMutation::SetRect {
                    node: canvas,
                    rect: viewport,
                },
                TreeMutation::SetRect {
                    node: grid,
                    rect: viewport,
                },
                TreeMutation::SetRect {
                    node: panel,
                    rect: viewport,
                },
                TreeMutation::SetRect {
                    node: overlay,
                    rect: viewport,
                },
            ] {
                self.tree
                    .apply_mutation(&self.template_registry, mutation)?;
            }
        }
        self.retained_root_ids()
            .ok_or(MutationError::MissingNode(usize::MAX))
    }

    pub fn retained_root_ids(&self) -> Option<RetainedRootIds> {
        Some(RetainedRootIds {
            root: self.tree.node_by_str("root")?,
            canvas_root: self.tree.node_by_str("canvas_root")?,
            canvas_grid: self.tree.node_by_str("canvas_grid")?,
            canvas_connections: self.tree.node_by_str("canvas_connections")?,
            panel_root: self.tree.node_by_str("panel_root")?,
            overlay_root: self.tree.node_by_str("__overlay_root")?,
        })
    }

    pub fn apply_mutation(
        &mut self,
        mutation: TreeMutation,
    ) -> Result<Invalidation, MutationError> {
        self.tree.apply_mutation(&self.template_registry, mutation)
    }

    pub fn apply_mutations(
        &mut self,
        mutations: impl IntoIterator<Item = TreeMutation>,
    ) -> Result<Vec<Invalidation>, MutationError> {
        self.tree
            .apply_mutations(&self.template_registry, mutations)
    }

    pub fn update_canvas_transform(
        &mut self,
        transform: crate::geometry::TransformSpec,
    ) -> Result<(), MutationError> {
        let Some(canvas_root) = self.tree.node_by_str("canvas_root") else {
            return Err(MutationError::MissingNode(usize::MAX));
        };
        self.apply_mutation(TreeMutation::SetStyle {
            node: canvas_root,
            patch: StylePatch {
                transform: Some(Some(transform)),
                ..StylePatch::default()
            },
        })?;
        Ok(())
    }

    /// 渲染整棵树。
    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        _viewport_w: f32,
        _viewport_h: f32,
        theme: &Theme,
    ) {
        if self.tree.root().is_some() {
            let flush = self.flush_paint_dirty(theme, renderer.text_measurer());
            render_trace::debug_stage(RenderTraceStage::PaintFlush, flush);
            let Some(list) = self.retained_display_list.as_ref() else {
                tracing::warn!(
                    target: TARGET_RENDER,
                    frame_id = render_trace::current_render_trace_frame().id,
                    "retained paint did not produce a display list"
                );
                return;
            };
            let resources = RegistryDisplayResources::new(self.resources.textures(), &self.icons);
            let report = renderer.draw_display_list(&list, &resources);
            if !report.unsupported.is_empty() {
                tracing::debug!(
                    target: TARGET_RENDER,
                    frame_id = render_trace::current_render_trace_frame().id,
                    unsupported = report.unsupported.len(),
                    "display list renderer skipped unsupported commands"
                );
            }
        }
    }

    pub fn flush_paint_dirty(
        &mut self,
        theme: &Theme,
        measurer: &mut TextMeasurer,
    ) -> PaintFlushStats {
        let Some(root) = self.tree.root() else {
            self.retained_display_list = None;
            return PaintFlushStats::default();
        };

        if self.retained_display_list.is_none()
            || self.last_paint_theme_revision != Some(theme.revision)
        {
            self.tree.mark_paint_dirty(root, PaintDirtyReason::Theme);
            self.last_paint_theme_revision = Some(theme.revision);
        }

        let root_boundary = self
            .tree
            .nearest_repaint_boundary(root)
            .unwrap_or(RepaintBoundaryId(root));
        let dirty = self.tree.take_paint_dirty();
        let dirty_boundaries = dirty.boundaries.len();
        let dirty_paint_order = dirty.paint_order.len();
        let dirty_composite = dirty.composite.len();
        let mut rebuild = dirty.boundaries;
        rebuild.extend(dirty.paint_order);

        if self.retained_display_list.is_none() {
            rebuild.extend(self.tree.repaint_boundaries_in_subtree(root));
        } else {
            let dirty_snapshot = rebuild.iter().copied().collect::<Vec<_>>();
            for boundary in dirty_snapshot {
                for descendant in self.tree.repaint_boundaries_in_subtree(boundary.0) {
                    if !self.tree.paint_fragment_cached(descendant) {
                        rebuild.insert(descendant);
                    }
                }
            }
        }

        let mut stats = PaintFlushStats {
            boundaries_dirty: rebuild.len(),
            ..PaintFlushStats::default()
        };
        render_trace::debug_stage(
            RenderTraceStage::PaintFlush,
            PaintDirtyTraceSummary {
                root,
                root_boundary: root_boundary.0,
                dirty_boundaries,
                dirty_paint_order,
                dirty_composite,
                rebuild_boundaries: rebuild.len(),
                retained_display_list_present: self.retained_display_list.is_some(),
            },
        );

        for boundary in rebuild.iter().copied() {
            let rebuilt = match self.tree.rebuild_paint_fragment(
                boundary,
                crate::tree::PaintCx {
                    interaction: Some(&self.interaction),
                    text_boxes: Some(self.systems.text_box_store()),
                    animations: Some(&self.animations),
                    theme,
                },
                |text, style| measurer.measure_with_style(text, style),
            ) {
                Ok(stats) => stats,
                Err(err) => {
                    tracing::warn!(
                        target: TARGET_RENDER,
                        frame_id = render_trace::current_render_trace_frame().id,
                        ?boundary,
                        ?err,
                        "failed to rebuild paint fragment"
                    );
                    continue;
                }
            };
            stats.fragments_rebuilt += rebuilt.fragments_rebuilt;
            stats.commands_recorded += rebuilt.commands_recorded;
            render_trace::trace_node(
                RenderTraceStage::PaintFragment,
                boundary.0,
                self.tree.get(boundary.0).map(|node| node.id.as_ref()),
                rebuilt,
            );
        }

        if stats.fragments_rebuilt == 0 && self.retained_display_list.is_some() {
            self.tree.record_paint_fragments_reused(1);
            stats.fragments_reused = 1;
            return stats;
        }

        let (display_list, compose_stats) = self.tree.compose_retained_display_list(root_boundary);
        stats.fragments_flattened += compose_stats.fragments_flattened;
        render_trace::debug_stage(
            RenderTraceStage::PaintFlush,
            PaintComposeTraceSummary {
                fragments_flattened: compose_stats.fragments_flattened,
                display_commands: display_list.commands.len(),
                clips: display_list.clips.len(),
            },
        );
        self.retained_display_list = Some(display_list);
        stats
    }

    pub fn register_texture(
        &mut self,
        handle: TextureHandle,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
    ) {
        self.resources.register_texture(handle, view, size);
    }

    pub fn register_svg_icon(&mut self, id: impl Into<IconId>, svg: impl AsRef<[u8]>) {
        self.icons.register_svg(id, svg);
    }

    pub fn open_overlay(&mut self, request: OverlayRequest) {
        self.systems.open_overlay(&self.tree, request);
    }

    pub fn close_overlay(&mut self) {
        self.systems
            .close_overlay(&self.tree, &mut self.interaction);
    }

    pub fn overlay_open(&self) -> bool {
        self.systems.overlay_open()
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        router::handle_event(self, event)
    }

    pub fn animate(&mut self, id: impl Into<String>) -> AnimationBuilder<'_> {
        self.animations.animate(id)
    }

    pub fn timeline(&mut self) -> TimelineBuilder<'_> {
        self.animations.timeline()
    }

    pub fn cancel_animation(&mut self, id: AnimationId) -> bool {
        self.animations.cancel(id)
    }

    pub fn clear_animation_visual(&mut self, id: &str) -> bool {
        self.animations.clear_visual(id)
    }

    pub fn tick_animations(&mut self, now: Instant) -> bool {
        self.animations.tick(now)
    }

    pub fn animations_active(&self) -> bool {
        self.animations.active()
    }

    pub fn ime_request(&self) -> ImeRequest {
        self.systems
            .ime_request(&self.tree, self.interaction.focused())
    }

    pub fn paste_focused_text(&mut self, text: &str) -> FrameworkOutput {
        self.systems
            .paste_focused_text(&self.tree, self.interaction.focused(), text)
    }

    #[cfg(test)]
    pub fn panel_root(&mut self, viewport: Rect, panels: Vec<PanelDeclaration>) -> Desc {
        crate::panel::panel_root(&mut self.tree, viewport, panels)
    }

    pub fn handle_panel_event(&mut self, event: &PanelEvent) -> bool {
        crate::panel::event::apply_panel_event(&mut self.tree, event)
    }

    pub fn sync_canvas_node_layouts(
        &mut self,
        identities: &[crate::canvas::CanvasNodeIdentity],
    ) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.sync_canvas_node_layouts(identities)
    }

    pub fn export_canvas_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.export_canvas_node_layouts()
    }

    pub fn import_canvas_node_layouts(&mut self, layouts: &[crate::canvas::CanvasNodeLayout]) {
        self.tree.import_canvas_node_layouts(layouts);
    }

    pub fn move_canvas_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        self.tree.move_canvas_node_by(owner_id, dx, dy)
    }

    pub fn resize_canvas_node_by(
        &mut self,
        owner_id: &str,
        edge: crate::widget::resize_edge::ResizeEdge,
        dx: f32,
        dy: f32,
    ) -> bool {
        self.tree.resize_canvas_node_by(owner_id, edge, dx, dy)
    }

    pub fn ensure_canvas_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        self.tree
            .ensure_canvas_node_min_size(owner_id, min_width, min_height)
    }

    pub fn apply_canvas_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        self.tree.apply_canvas_node_sizing(owner_id, request)
    }

    pub fn control_intrinsics(&self) -> Vec<crate::runtime::ControlIntrinsic> {
        self.systems.control_intrinsics()
    }

    pub fn retained_control_intrinsics_snapshot(&self) -> Vec<crate::runtime::ControlIntrinsic> {
        self.systems.control_intrinsics()
    }

    pub fn take_dirty_control_intrinsics(&mut self) -> Vec<crate::runtime::ControlIntrinsic> {
        self.systems.take_dirty_control_intrinsics()
    }

    pub fn take_text_box_dirty_intrinsics(&mut self) -> std::collections::BTreeSet<String> {
        self.systems.take_text_box_dirty_intrinsics()
    }

    pub fn sync_retained_canvas_text_boxes(
        &mut self,
        views: &[crate::canvas::node_template::CanvasNodeRenderView],
        measurer: &mut TextMeasurer,
        theme: &Theme,
    ) {
        let before_dirty = self.systems.text_box_dirty_intrinsics().len();
        self.systems.sync_retained_canvas_text_boxes(
            &self.tree,
            views,
            measurer,
            theme,
            self.interaction.focused(),
        );
        render_trace::debug_stage(
            RenderTraceStage::TextRuntimeSync,
            TextRuntimeTraceSummary {
                views: views.len(),
                dirty_intrinsics_before: before_dirty,
                dirty_intrinsics_after: self.systems.text_box_dirty_intrinsics().len(),
            },
        );
    }

    pub fn text_box_dirty_intrinsics(&self) -> Vec<String> {
        self.systems.text_box_dirty_intrinsics()
    }

    pub fn canvas_port_group_view(
        &self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> crate::canvas::CanvasPortGroupView {
        self.tree.canvas_port_group_view(owner_id, side)
    }

    pub fn toggle_canvas_port_group(
        &mut self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> bool {
        self.tree.toggle_canvas_port_group(owner_id, side)
    }

    pub fn select_canvas_node(&mut self, owner_id: &str) -> bool {
        self.tree.select_canvas_node(owner_id)
    }

    pub fn clear_canvas_selection(&mut self) {
        self.tree.clear_canvas_selection();
    }

    pub fn is_canvas_node_selected(&self, owner_id: &str) -> bool {
        self.tree.is_canvas_node_selected(owner_id)
    }

    pub fn pending_canvas_connection(&self) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.pending_canvas_connection()
    }

    pub fn begin_pending_canvas_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.tree
            .begin_pending_canvas_connection(from_port_id, cursor_canvas)
    }

    pub fn update_pending_canvas_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        self.tree.update_pending_canvas_connection(cursor_canvas)
    }

    pub fn end_pending_canvas_connection(
        &mut self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.end_pending_canvas_connection()
    }

    pub fn cancel_pending_canvas_connection(&mut self) -> bool {
        self.tree.cancel_pending_canvas_connection()
    }

    pub fn hovered_canvas_port_id(&self) -> Option<String> {
        self.tree.hovered_canvas_port_id()
    }

    pub fn set_hovered_canvas_port(&mut self, port_id: Option<&str>) -> bool {
        self.tree.set_hovered_canvas_port(port_id)
    }

    pub fn export_panel_layouts(&self) -> Vec<crate::panel::PanelLayout> {
        self.tree.export_panel_layouts()
    }

    pub fn ensure_panel_runtime(
        &mut self,
        config: &crate::panel::PanelConfig,
    ) -> Option<crate::panel::PanelRuntime> {
        self.tree.ensure_panel(config);
        self.tree.panel_state(config.id.as_str()).cloned()
    }

    pub fn panel_runtime(&self, id: &str) -> Option<crate::panel::PanelRuntime> {
        self.tree.panel_state(id).cloned()
    }

    pub fn import_panel_layouts(&mut self, layouts: &[crate::panel::PanelLayout]) {
        self.tree.import_panel_layouts(layouts);
    }

    /// 命中测试，返回从叶子到根的命中链。
    pub fn hit_test(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test_with_animations(&self.tree, root, x, y, Some(&self.animations))
    }

    pub fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.tree.node_by_str(id)
    }

    pub fn node_exists(&self, id: &str) -> bool {
        self.node_id_by_name(id).is_some()
    }

    pub fn node_rect(&self, id: &str) -> Option<Rect> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.node_rect_by_node(node_id))
    }

    pub fn node_rect_by_node(&self, node_id: NodeId) -> Option<Rect> {
        self.tree.get(node_id).map(|node| node.rect)
    }

    pub fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.tree.get(node_id).map(|node| node.id.as_ref())
    }

    pub fn node_scroll_offset(&self, id: &str) -> Option<f32> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.tree.get(node_id))
            .map(|node| node.scroll_offset())
    }

    pub fn node_has_gesture(&self, node_id: NodeId, gesture: Gesture) -> bool {
        self.tree
            .get(node_id)
            .map(|node| node.style.gestures.contains(&gesture))
            .unwrap_or(false)
    }

    pub fn node_is_draggable(&self, node_id: NodeId) -> bool {
        self.tree
            .get(node_id)
            .map(|node| node.style.draggable)
            .unwrap_or(false)
    }

    pub fn node_is_resizable(&self, node_id: NodeId) -> bool {
        self.tree
            .get(node_id)
            .map(|node| node.style.resizable)
            .unwrap_or(false)
    }

    pub fn resize_hit_at_screen_point(&self, x: f32, y: f32) -> Option<(NodeId, ResizeEdge)> {
        let Some(root) = self.tree.root() else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                x,
                y,
                "query resize edge at screen point: root missing"
            );
            return None;
        };
        let hit = resize_hit_at_screen_point(&self.tree, root, x, y, Some(&self.animations));
        let root_node = self.tree.get(root).map(|node| node.id.to_string());
        let hit_node =
            hit.and_then(|hit| self.tree.get(hit.node_id).map(|node| node.id.to_string()));
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            x,
            y,
            root_node = root_node.as_deref(),
            hit_node = hit_node.as_deref(),
            edge = ?hit.map(|hit| hit.edge),
            "query resize edge at screen point"
        );
        hit.map(|hit| (hit.node_id, hit.edge))
    }

    pub fn node_root_widget_type(&self, node_id: NodeId) -> Option<&str> {
        let mut candidate = self.node_name(node_id)?;

        loop {
            if let Some(root_id) = self.node_id_by_name(candidate) {
                if let Some(root_node) = self.tree.get(root_id) {
                    match &root_node.kind {
                        NodeKind::Widget(props) => return Some(props.widget_type()),
                        _ => {
                            if let Some(role) = root_node.props.semantic_role.as_deref() {
                                return Some(role);
                            }
                        }
                    }
                }
            }

            let (prefix, _) = candidate.rsplit_once("::")?;
            candidate = prefix;
        }
    }

    pub fn node_is_text_input_field(&self, node_id: NodeId) -> bool {
        self.node_name(node_id)
            .is_some_and(|id| id.ends_with("::field"))
            && self.node_root_widget_type(node_id) == Some("TextInput")
    }

    pub fn node_is_text_area_field(&self, node_id: NodeId) -> bool {
        self.node_name(node_id)
            .is_some_and(|id| id.ends_with("::field"))
            && self.node_root_widget_type(node_id) == Some("TextArea")
    }

    pub fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub fn focused_widget_id(&self) -> Option<&str> {
        self.node_name_for(self.focused_node())
    }

    pub fn hovered_node(&self) -> Option<NodeId> {
        self.interaction.hovered()
    }

    pub fn hovered_widget_id(&self) -> Option<&str> {
        self.node_name_for(self.hovered_node())
    }

    pub fn captured_node(&self) -> Option<NodeId> {
        self.interaction.captured()
    }

    pub fn captured_widget_id(&self) -> Option<&str> {
        self.node_name_for(self.captured_node())
    }

    pub fn request_focus(&mut self, node_id: NodeId) {
        self.interaction.focus(node_id);
    }

    pub fn clear_focus(&mut self) {
        self.interaction.blur();
    }

    pub(crate) fn handle_interaction_event(&mut self, event: &AppEvent) {
        self.interaction
            .handle_event(&self.tree, Some(&self.animations), event);
    }

    pub(crate) fn handle_runtime_pre_gesture_event(
        &mut self,
        event: &AppEvent,
    ) -> RuntimeEventResult {
        self.systems.handle_pre_gesture_event(
            RuntimeEventCx {
                tree: &mut self.tree,
                interaction: &mut self.interaction,
                animations: Some(&self.animations),
            },
            event,
        )
    }

    pub(crate) fn handle_gesture_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        let update = self.handle_gesture_session_event(event);
        let output = update
            .signal
            .as_ref()
            .map(|signal| gesture_adapter::gesture_signal_output(&self.tree, signal))
            .unwrap_or_default();
        output.with_consumed(update.consumed)
    }

    pub(crate) fn handle_gesture_session_event(
        &mut self,
        event: &AppEvent,
    ) -> GestureSessionUpdate {
        self.gesture_session
            .handle_event(&self.tree, Some(&self.animations), event)
    }

    pub(crate) fn cancel_gesture(&mut self) {
        self.gesture_session.cancel();
    }

    fn node_name_for(&self, node_id: Option<NodeId>) -> Option<&str> {
        node_id.and_then(|id| self.node_name(id))
    }

    pub fn last_frame_stats(&self) -> FrameStats {
        self.tree.frame_stats_snapshot()
    }

    pub fn template_registry(&self) -> &TemplateRegistry {
        &self.template_registry
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

fn boundary_available_rect(tree: &Tree, boundary: NodeId, root_rect: Rect) -> Option<Rect> {
    if tree.root() == Some(boundary) {
        return Some(root_rect);
    }
    tree.get(boundary).map(|node| {
        if node.rect.w > 0.0 || node.rect.h > 0.0 {
            node.rect
        } else {
            root_rect
        }
    })
}

fn retained_root_slots(viewport: Rect) -> SlotValues {
    SlotValues::new()
        .with("root_rect", SlotValue::Rect(viewport))
        .with("canvas_rect", SlotValue::Rect(viewport))
        .with("grid_rect", SlotValue::Rect(viewport))
        .with("panel_rect", SlotValue::Rect(viewport))
        .with("overlay_rect", SlotValue::Rect(viewport))
}

fn layout_output_from_tree(tree: &Tree, node: NodeId) -> Option<LayoutOutput> {
    let tree_node = tree.get(node)?;
    let padding = tree_node.style.padding;
    let content_rect = Rect {
        x: tree_node.rect.x + padding.left,
        y: tree_node.rect.y + padding.top,
        w: (tree_node.rect.w - padding.horizontal()).max(0.0),
        h: (tree_node.rect.h - padding.vertical()).max(0.0),
    };
    Some(LayoutOutput {
        rect: tree_node.rect,
        content_rect,
        intrinsic_width: tree_node.rect.w,
        intrinsic_height: tree_node.rect.h,
        baseline: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::TextMeasurer;
    use crate::shell::{Key, Modifiers, MouseButton};
    use crate::theme::{dark_theme, Theme};
    use crate::ui::{self, StyleBuilder};
    use crate::widget::atoms::button::ButtonProps;
    use crate::widget::atoms::dropdown::DropdownProps;
    use crate::widget::atoms::label::{LabelProps, LabelVariant};
    use crate::widget::atoms::number_input::NumberInputProps;
    use crate::widget::atoms::slider::SliderProps;
    use crate::widget::atoms::text_area::TextAreaProps;
    use crate::widget::atoms::text_input::TextInputProps;
    use crate::widget::atoms::toggle::ToggleProps;
    use crate::widget::frameworks::panel::PanelProps;
    use crate::widget::frameworks::scroll_area::ScrollAreaProps;
    use crate::widget::props::WidgetProps;
    use std::borrow::Cow;
    use std::time::{Duration, Instant};

    fn root_desc(height: f32, child: impl Into<Desc>) -> Desc {
        ui::container("root")
            .fixed_width(320.0)
            .fixed_height(height)
            .child(child)
            .build()
    }

    fn widget(id: impl Into<Cow<'static, str>>, props: impl WidgetProps) -> Desc {
        ui::widget(id, props).build()
    }

    #[test]
    fn flush_layout_dirty_reuses_boundary_layout_cache() {
        let theme = dark_theme();
        let mut cx = Context::new();
        let mut measurer = TextMeasurer::new();
        let root_rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 120.0,
        };
        cx.update(
            root_desc(
                120.0,
                ui::container("child")
                    .fixed_width(40.0)
                    .fixed_height(24.0)
                    .build(),
            ),
            root_rect,
            &mut measurer,
            &theme,
        );
        let root = cx.root().expect("root");
        cx.tree.clear_frame_stats();

        cx.tree.mark_dirty(root, crate::tree::DirtyFlags::LAYOUT);
        let first = cx.flush_layout_dirty(root_rect, &mut measurer);
        cx.tree.mark_dirty(root, crate::tree::DirtyFlags::LAYOUT);
        let second = cx.flush_layout_dirty(root_rect, &mut measurer);

        assert_eq!(first.boundaries_flushed, 1);
        assert_eq!(first.boundaries_skipped_cache_hit, 0);
        assert_eq!(second.boundaries_flushed, 0);
        assert_eq!(second.boundaries_skipped_cache_hit, 1);
        assert_eq!(cx.last_frame_stats().layout_cache_hits, 1);
    }

    fn test_desc(value: &str) -> Desc {
        root_desc(
            120.0,
            widget(
                "input",
                TextInputProps {
                    label: Some(Cow::Borrowed("Prompt")),
                    value: Cow::Owned(value.to_string()),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ),
        )
    }

    fn nested_text_input_desc(value: &str) -> Desc {
        root_desc(
            120.0,
            widget(
                "canvas_node::engine_node::7::body::param::0::control::widget",
                TextInputProps {
                    label: None,
                    value: Cow::Owned(value.to_string()),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ),
        )
    }

    fn canvas_node_text_area_desc(value: &str) -> Desc {
        root_desc(
            180.0,
            ui::container("canvas_node")
                .fixed_width(300.0)
                .fixed_height(160.0)
                .hittable(true)
                .draggable(true)
                .resizable(true)
                .child(widget(
                    "canvas_node::body::control",
                    TextAreaProps {
                        label: None,
                        value: Cow::Owned(value.to_string()),
                        disabled: false,
                        size: Default::default(),
                        density: Default::default(),
                        min_rows: 5,
                    },
                ))
                .build(),
        )
    }

    fn real_node_card_text_area_desc(value: &str, theme: &Theme) -> Desc {
        real_node_card_text_area_canvas_desc(value, theme, None, 180.0)
    }

    fn real_node_card_text_area_desc_with_height(
        value: &str,
        theme: &Theme,
        card_height: f32,
    ) -> Desc {
        real_node_card_text_area_canvas_desc(value, theme, None, card_height)
    }

    fn transformed_real_node_card_text_area_desc(value: &str, theme: &Theme) -> Desc {
        use crate::geometry::TransformSpec;

        real_node_card_text_area_canvas_desc(
            value,
            theme,
            Some(TransformSpec::translate_scale([100.0, -40.0], 2.0)),
            180.0,
        )
    }

    fn real_node_card_text_area_canvas_desc(
        value: &str,
        theme: &Theme,
        canvas_transform: Option<crate::geometry::TransformSpec>,
        card_height: f32,
    ) -> Desc {
        use crate::canvas::node_card::node_card_from_render_view;
        use crate::canvas::node_template::{
            CanvasNodeInstanceState, CanvasNodeParamTemplate, CanvasNodeRenderView,
            CanvasNodeTemplate,
        };
        use crate::canvas::{CanvasNodeLayout, CanvasPortGroupView};
        use crate::widget::mapping::ParamControlSpec;

        let owner_id = "showcase_node::text_area_control".to_string();
        let template = CanvasNodeTemplate {
            type_id: owner_id.clone(),
            title: "Text Area".to_string(),
            subtitle: "single text area control".to_string(),
            category: "ui/showcase".to_string(),
            params: vec![CanvasNodeParamTemplate::new(
                "Prompt",
                "",
                "",
                "",
                ParamControlSpec::TextArea {
                    value: value.to_string(),
                    min_rows: 5,
                },
            )],
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        let state = CanvasNodeInstanceState {
            owner_id: owner_id.clone(),
            layout: CanvasNodeLayout {
                owner_id,
                rect: Rect {
                    x: 20.0,
                    y: 20.0,
                    w: 304.0,
                    h: card_height,
                },
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            selected: false,
            input_group: CanvasPortGroupView::default(),
            output_group: CanvasPortGroupView::default(),
            port_states: Vec::new(),
        };

        let card = node_card_from_render_view(&CanvasNodeRenderView { template, state }, theme);
        let mut canvas_root = ui::container("canvas_root")
            .absolute_xy(0.0, 0.0)
            .fixed_width(520.0)
            .fixed_height(320.0);
        if let Some(transform) = canvas_transform {
            canvas_root = canvas_root.transform(transform);
        }

        ui::container("root")
            .fixed_width(720.0)
            .fixed_height(520.0)
            .child(canvas_root.child(card).build())
            .build()
    }

    fn button_desc() -> Desc {
        root_desc(
            120.0,
            widget(
                "button",
                ButtonProps {
                    label: Cow::Borrowed("Run"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ),
        )
    }

    fn toggle_desc() -> Desc {
        root_desc(
            120.0,
            widget(
                "toggle",
                ToggleProps {
                    label: Some(Cow::Borrowed("Grid")),
                    value: true,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ),
        )
    }

    fn slider_desc() -> Desc {
        root_desc(
            120.0,
            widget(
                "slider",
                SliderProps {
                    label: Some(Cow::Borrowed("Radius")),
                    min: 0.0,
                    max: 10.0,
                    step: 1.0,
                    value: 5.0,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ),
        )
    }

    fn panel_desc() -> Desc {
        root_desc(
            180.0,
            widget(
                "panel",
                PanelProps {
                    title: Cow::Borrowed("Panel"),
                    rect: Rect {
                        x: 20.0,
                        y: 20.0,
                        w: 180.0,
                        h: 100.0,
                    },
                    z_index: 0,
                    min_size: [120.0, 80.0],
                    titlebar_visible: true,
                    draggable: true,
                    resizable: true,
                    closable: false,
                    content: vec![],
                },
            ),
        )
    }

    fn panel_with_input_desc() -> Desc {
        root_desc(
            240.0,
            widget(
                "panel",
                PanelProps {
                    title: Cow::Borrowed("Panel"),
                    rect: Rect {
                        x: 20.0,
                        y: 20.0,
                        w: 240.0,
                        h: 160.0,
                    },
                    z_index: 0,
                    min_size: [120.0, 80.0],
                    titlebar_visible: true,
                    draggable: true,
                    resizable: true,
                    closable: false,
                    content: vec![widget(
                        "input",
                        TextInputProps {
                            label: Some(Cow::Borrowed("Prompt")),
                            value: Cow::Borrowed("hello"),
                            disabled: false,
                            size: Default::default(),
                            density: Default::default(),
                        },
                    )],
                },
            ),
        )
    }

    fn popup_content_desc() -> Desc {
        widget(
            "popup_button",
            ButtonProps {
                label: Cow::Borrowed("Overlay"),
                icon: None,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        )
    }

    fn scroll_desc() -> Desc {
        let items = (0..20)
            .map(|index| {
                widget(
                    format!("item_{index}"),
                    LabelProps {
                        text: Cow::Owned(format!("Item {index}")),
                        variant: LabelVariant::Body,
                        muted: false,
                    },
                )
            })
            .collect();

        root_desc(
            200.0,
            widget(
                "scroll",
                ScrollAreaProps {
                    height: 80.0,
                    content: items,
                },
            ),
        )
    }

    fn number_desc(value: f32) -> Desc {
        root_desc(
            120.0,
            widget(
                "number",
                NumberInputProps {
                    label: Some(Cow::Borrowed("Radius")),
                    value,
                    min: 0.0,
                    max: 10.0,
                    step: 0.5,
                    precision: 2,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ),
        )
    }

    fn dropdown_desc(selected: usize) -> Desc {
        root_desc(
            180.0,
            widget(
                "dropdown",
                DropdownProps {
                    label: Some(Cow::Borrowed("Mode")),
                    options: vec![
                        Cow::Borrowed("Normal"),
                        Cow::Borrowed("Multiply"),
                        Cow::Borrowed("Screen"),
                    ],
                    selected,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            ),
        )
    }

    fn test_context(value: &str) -> Context {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            test_desc(value),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx
    }

    fn field_rect(ctx: &Context) -> Rect {
        ctx.node_rect("input::field")
            .expect("text input field rect")
    }

    fn button_rect(ctx: &Context) -> Rect {
        ctx.node_rect("button").expect("button rect")
    }

    fn node_rect(ctx: &Context, node_name: &str) -> Rect {
        ctx.node_rect(node_name).expect("node rect")
    }

    fn focus_number(ctx: &mut Context) {
        let rect = node_rect(ctx, "number::field");
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
    }

    fn dropdown_field_rect(ctx: &Context) -> Rect {
        node_rect(ctx, "dropdown::field")
    }

    fn focus_input(ctx: &mut Context) {
        let rect = field_rect(ctx);
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
    }

    #[test]
    fn context_query_api_finds_nodes_without_exposing_tree() {
        let ctx = test_context("hello");

        let field_id = ctx
            .node_id_by_name("input::field")
            .expect("text input field id");

        assert!(ctx.node_exists("input::field"));
        assert!(!ctx.node_exists("missing"));
        assert_eq!(ctx.node_name(field_id), Some("input::field"));
        let by_name = ctx.node_rect("input::field").expect("rect by name");
        let by_node = ctx.node_rect_by_node(field_id).expect("rect by node");
        assert_eq!(by_name.x, by_node.x);
        assert_eq!(by_name.y, by_node.y);
        assert_eq!(by_name.w, by_node.w);
        assert_eq!(by_name.h, by_node.h);
        assert_eq!(ctx.node_root_widget_type(field_id), Some("TextInput"));
        assert!(ctx.node_is_text_input_field(field_id));
        assert!(ctx.node_rect("missing").is_none());
    }

    #[test]
    fn context_query_api_uses_node_resize_threshold() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            ui::container("resize_box")
                .fixed_width(100.0)
                .fixed_height(60.0)
                .hittable(true)
                .resizable(true)
                .resize_edge_threshold(14.0)
                .build(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let node_id = ctx.node_id_by_name("resize_box").expect("resize node");
        let rect = ctx.node_rect_by_node(node_id).expect("resize rect");

        assert_eq!(
            ctx.resize_hit_at_screen_point(rect.x + rect.w - 12.0, rect.y + 30.0),
            Some((node_id, ResizeEdge::Right))
        );
        assert_eq!(
            ctx.resize_hit_at_screen_point(rect.x + rect.w - 16.0, rect.y + 30.0),
            None
        );
    }

    #[test]
    fn context_query_api_resolves_nested_text_input_root() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id = "canvas_node::engine_node::7::body::param::0::control::widget";
        let field_id = "canvas_node::engine_node::7::body::param::0::control::widget::field";
        ctx.update(
            nested_text_input_desc("hello"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let field_node = ctx.node_id_by_name(field_id).expect("nested field id");

        assert_eq!(ctx.node_root_widget_type(field_node), Some("TextInput"));
        assert!(ctx.node_is_text_input_field(field_node));
        assert!(ctx.node_exists(widget_id));
    }

    #[test]
    fn nested_text_input_accepts_pointer_focus_and_text_input() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id = "canvas_node::engine_node::7::body::param::0::control::widget";
        let field_id = "canvas_node::engine_node::7::body::param::0::control::widget::field";
        ctx.update(
            nested_text_input_desc("hello"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let field = node_rect(&ctx, field_id);
        let x = field.x + field.w * 0.5;
        let y = field.y + field.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::End,
            modifiers: Modifiers::default(),
        });
        let output = ctx.handle_event(&AppEvent::TextInput {
            text: "!".to_string(),
        });

        assert_eq!(ctx.focused_widget_id(), Some(widget_id));
        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::TextChanged { id, value })]
                if id == widget_id && value == "hello!"
        ));
    }

    #[test]
    fn canvas_node_text_area_accepts_pointer_focus_and_text_input() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id = "canvas_node::body::control";
        let field_id = "canvas_node::body::control::field";
        ctx.update(
            canvas_node_text_area_desc("hello"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let field = node_rect(&ctx, field_id);
        let x = field.x + 8.0;
        let y = field.y + field.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::End,
            modifiers: Modifiers::default(),
        });
        let output = ctx.handle_event(&AppEvent::TextInput {
            text: "!".to_string(),
        });

        assert_eq!(ctx.focused_widget_id(), Some(widget_id));
        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::TextChanged { id, value })]
                if id == widget_id && value == "hello!"
        ));
    }

    #[test]
    fn canvas_node_text_area_drag_selection_can_be_copied_and_painted() {
        use crate::paint::PaintCommand;

        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id = "canvas_node::body::control";
        let field_id = "canvas_node::body::control::field";
        ctx.update(
            canvas_node_text_area_desc("hello world"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let field = node_rect(&ctx, field_id);
        let y = field.y + 8.0;
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: field.x + 4.0,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseMove {
            x: field.x + 64.0,
            y,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: field.x + 64.0,
            y,
            button: MouseButton::Left,
        });

        assert_eq!(ctx.focused_widget_id(), Some(widget_id));
        let copy_output = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('C'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        assert!(matches!(
            copy_output.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if !text.is_empty()
        ));

        let list = build_display_list(
            &ctx.tree,
            ctx.root().expect("root"),
            crate::tree::PaintCx {
                interaction: Some(&ctx.interaction),
                text_boxes: Some(ctx.systems.text_box_store()),
                animations: None,
                theme: &theme,
            },
            |text, style| {
                (
                    text.chars().count() as f32 * style.size * 0.5,
                    style.size * style.line_height,
                )
            },
        )
        .expect("display list");

        assert!(list.commands.iter().any(|command| matches!(
            &command.command,
            PaintCommand::Rect(rect) if rect.style.color == theme.selection_color()
        )));
    }

    #[test]
    fn real_node_card_text_area_drag_selection_beats_card_drag() {
        use crate::paint::PaintCommand;

        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget";
        let field_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget::field";
        ctx.update(
            real_node_card_text_area_desc("hello world", &theme),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 520.0,
                h: 320.0,
            },
            &mut measurer,
            &theme,
        );

        let field = node_rect(&ctx, field_id);
        let y = field.y + 8.0;
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: field.x + 4.0,
            y,
            button: MouseButton::Left,
        });
        let move_output = ctx.handle_event(&AppEvent::MouseMove {
            x: field.x + 64.0,
            y,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: field.x + 64.0,
            y,
            button: MouseButton::Left,
        });

        assert!(move_output.consumed);
        assert_eq!(ctx.focused_widget_id(), Some(widget_id));
        let copy_output = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('C'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        assert!(matches!(
            copy_output.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if !text.is_empty()
        ));

        let list = build_display_list(
            &ctx.tree,
            ctx.root().expect("root"),
            crate::tree::PaintCx {
                interaction: Some(&ctx.interaction),
                text_boxes: Some(ctx.systems.text_box_store()),
                animations: None,
                theme: &theme,
            },
            |text, style| {
                (
                    text.chars().count() as f32 * style.size * 0.5,
                    style.size * style.line_height,
                )
            },
        )
        .expect("display list");
        assert!(list.commands.iter().any(|command| matches!(
            &command.command,
            PaintCommand::Rect(rect) if rect.style.color == theme.selection_color()
        )));
    }

    #[test]
    fn real_node_card_text_area_wraps_to_card_width() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget";
        let field_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget::field";
        let card_id = "canvas_node::showcase_node::text_area_control::card";
        let value = "A compact text field".repeat(8);
        ctx.update(
            real_node_card_text_area_desc(&value, &theme),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 720.0,
                h: 520.0,
            },
            &mut measurer,
            &theme,
        );

        let card = node_rect(&ctx, card_id);
        let field = node_rect(&ctx, field_id);
        assert!(field.x >= card.x);
        assert!(field.x + field.w <= card.x + card.w + 0.001);

        let runtime = ctx
            .systems
            .text_box_store()
            .runtime(widget_id)
            .expect("text area runtime");
        assert!(runtime.layout().lines.len() > 1);
        assert!(runtime.clip_rect().w <= field.w);
    }

    #[test]
    fn real_node_card_text_area_field_stretches_to_card_height() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget";
        let field_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget::field";
        let value = "A compact text field".repeat(8);
        ctx.update(
            real_node_card_text_area_desc_with_height(&value, &theme, 300.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 720.0,
                h: 520.0,
            },
            &mut measurer,
            &theme,
        );

        let field = node_rect(&ctx, field_id);
        let runtime = ctx
            .systems
            .text_box_store()
            .runtime(widget_id)
            .expect("text area runtime");
        assert_eq!(runtime.current_size()[1], field.h);
        assert!(field.h > runtime.min_size()[1]);
    }

    #[test]
    fn real_node_card_text_area_paints_wrapped_lines() {
        use crate::paint::PaintCommand;

        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let value = "A compact text field".repeat(8);
        ctx.update(
            real_node_card_text_area_desc(&value, &theme),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 720.0,
                h: 520.0,
            },
            &mut measurer,
            &theme,
        );

        let list = build_display_list(
            &ctx.tree,
            ctx.root().expect("root"),
            crate::tree::PaintCx {
                interaction: Some(&ctx.interaction),
                text_boxes: Some(ctx.systems.text_box_store()),
                animations: None,
                theme: &theme,
            },
            |text, style| {
                (
                    text.chars().count() as f32 * style.size * 0.5,
                    style.size * style.line_height,
                )
            },
        )
        .expect("display list");

        let text_y = list
            .commands
            .iter()
            .filter_map(|command| match &command.command {
                PaintCommand::Text(text)
                    if !text.text.is_empty() && value.contains(text.text.as_str()) =>
                {
                    Some(text.pos.y)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(text_y.len() > 1);
        assert!(text_y.windows(2).all(|pair| pair[1] > pair[0]));
    }

    #[test]
    fn transformed_canvas_text_area_selection_uses_screen_to_layout_coordinates() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let widget_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget";
        let field_id =
            "canvas_node::showcase_node::text_area_control::body::param::0::control::widget::field";
        ctx.update(
            transformed_real_node_card_text_area_desc("hello world", &theme),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 720.0,
                h: 520.0,
            },
            &mut measurer,
            &theme,
        );

        let field = node_rect(&ctx, field_id);
        let to_screen = |x: f32, y: f32| (x * 2.0 + 100.0, y * 2.0 - 40.0);
        let (press_x, press_y) = to_screen(field.x + 4.0, field.y + 8.0);
        let (move_x, move_y) = to_screen(field.x + 64.0, field.y + 8.0);

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: press_x,
            y: press_y,
            button: MouseButton::Left,
        });
        let move_output = ctx.handle_event(&AppEvent::MouseMove {
            x: move_x,
            y: move_y,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: move_x,
            y: move_y,
            button: MouseButton::Left,
        });

        assert!(move_output.consumed);
        assert_eq!(ctx.focused_widget_id(), Some(widget_id));
        let copy_output = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('C'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        assert!(matches!(
            copy_output.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if !text.is_empty()
        ));
    }

    #[test]
    fn nested_text_input_paints_runtime_text_before_external_sync() {
        use crate::paint::PaintCommand;

        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let field_id = "canvas_node::engine_node::7::body::param::0::control::widget::field";
        ctx.update(
            nested_text_input_desc("A compact text field"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let field = node_rect(&ctx, field_id);
        let x = field.x + field.w * 0.5;
        let y = field.y + field.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::End,
            modifiers: Modifiers::default(),
        });
        let _ = ctx.handle_event(&AppEvent::TextInput {
            text: " extended".to_string(),
        });
        let _ = ctx.handle_event(&AppEvent::ImePreedit {
            text: "中".to_string(),
            caret: None,
        });

        let list = build_display_list(
            &ctx.tree,
            ctx.root().expect("root"),
            crate::tree::PaintCx {
                interaction: Some(&ctx.interaction),
                text_boxes: Some(ctx.systems.text_box_store()),
                animations: None,
                theme: &theme,
            },
            |text, style| {
                (
                    text.chars().count() as f32 * style.size * 0.5,
                    style.size * style.line_height,
                )
            },
        )
        .expect("display list");
        let texts = list
            .commands
            .iter()
            .filter_map(|command| match &command.command {
                PaintCommand::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(texts.contains(&"A compact text field extended"));
        assert!(texts.contains(&"中"));
    }

    #[test]
    fn context_animation_transform_participates_in_hit_testing() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = button_rect(&ctx);
        let now = Instant::now();
        ctx.animate("button")
            .to(crate::animation::AnimationProps::new().translate([120.0, 0.0]))
            .duration_ms(100)
            .ease(crate::animation::Ease::Linear)
            .play_at(now);
        ctx.tick_animations(now + Duration::from_millis(100));

        let chain = ctx.hit_test(rect.x + rect.w * 0.5 + 120.0, rect.y + rect.h * 0.5);

        assert!(chain.iter().any(|node_id| {
            ctx.node_name(node_id)
                .map(|name| name == "button")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn ctrl_c_requests_clipboard_copy_without_text_change() {
        let mut ctx = test_context("hello");
        focus_input(&mut ctx);

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        let outcome = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('C'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        assert!(matches!(
            outcome.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if text == "hello"
        ));
    }

    #[test]
    fn ctrl_x_requests_cut_and_returns_text_change() {
        let mut ctx = test_context("hello");
        focus_input(&mut ctx);

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        let outcome = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('X'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        assert!(matches!(
            outcome.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if text == "hello"
        ));
        assert!(matches!(
            outcome.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::TextChanged { id, value })]
                if id == "input" && value.is_empty()
        ));
        assert!(outcome.consumed);
    }

    #[test]
    fn drag_selection_can_be_copied() {
        let mut ctx = test_context("hello world");
        let rect = field_rect(&ctx);
        let y = rect.y + rect.h * 0.5;
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + 4.0,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseMove {
            x: rect.x + 48.0,
            y,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + 48.0,
            y,
            button: MouseButton::Left,
        });

        let outcome = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('C'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        assert!(matches!(
            outcome.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if !text.is_empty()
        ));
    }

    #[test]
    fn theme_revision_forces_widget_rebuild() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let dark = dark_theme();
        let mut updated = dark_theme();
        updated.revision = 99;
        updated.controls.medium_regular.height = 52.0;

        ctx.update(
            test_desc("hello"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &dark,
        );

        let before_height = ctx.node_rect("input::field").expect("text input field").h;

        ctx.update(
            test_desc("hello"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &updated,
        );

        let after_height = ctx.node_rect("input::field").expect("text input field").h;

        assert_eq!(before_height, dark.controls.medium_regular.height);
        assert_eq!(after_height, updated.controls.medium_regular.height);
    }

    #[test]
    fn context_emits_click_through_gesture_session() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = button_rect(&ctx);

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        let output = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });

        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::Click { id })] if id == "button"
        ));
        assert!(output.consumed);
    }

    #[test]
    fn mouse_move_updates_hovered_widget_through_context_facade() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = button_rect(&ctx);

        let _ = ctx.handle_event(&AppEvent::MouseMove {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
        });

        assert_eq!(ctx.hovered_widget_id(), Some("button"));
    }

    #[test]
    fn mouse_press_and_release_updates_capture_through_context_facade() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = button_rect(&ctx);
        let x = rect.x + rect.w * 0.5;
        let y = rect.y + rect.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        assert_eq!(ctx.captured_widget_id(), Some("button"));

        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        assert_eq!(ctx.captured_widget_id(), None);
    }

    #[test]
    fn text_input_consumed_pointer_press_cancels_active_gesture_session() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_with_input_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 240.0,
            },
            &mut measurer,
            &theme,
        );
        let titlebar = node_rect(&ctx, "panel::titlebar");
        let field = node_rect(&ctx, "input::field");

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: titlebar.x + titlebar.w * 0.5,
            y: titlebar.y + titlebar.h * 0.5,
            button: MouseButton::Left,
        });
        assert!(ctx.gesture_session.is_active());

        let output = ctx.handle_event(&AppEvent::MousePress {
            x: field.x + field.w * 0.5,
            y: field.y + field.h * 0.5,
            button: MouseButton::Left,
        });

        assert!(output.consumed);
        assert!(!ctx.gesture_session.is_active());
        assert_eq!(ctx.focused_widget_id(), Some("input"));
    }

    #[test]
    fn unfocused_cancels_active_gesture_session() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let titlebar = node_rect(&ctx, "panel::titlebar");

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: titlebar.x + titlebar.w * 0.5,
            y: titlebar.y + titlebar.h * 0.5,
            button: MouseButton::Left,
        });
        assert!(ctx.gesture_session.is_active());

        let output = ctx.handle_event(&AppEvent::Unfocused);

        assert!(!output.consumed);
        assert!(!ctx.gesture_session.is_active());
    }

    #[test]
    fn context_tracks_double_click_without_demo_tap_state() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = button_rect(&ctx);
        let x = rect.x + rect.w * 0.5;
        let y = rect.y + rect.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let output = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });

        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DoubleClick { id })] if id == "button"
        ));
        assert!(output.consumed);
    }

    #[test]
    fn widget_event_canonicalizes_child_target_to_widget_id() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            toggle_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "toggle::track");

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        let output = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });

        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::Click { id })] if id == "toggle"
        ));
    }

    #[test]
    fn context_emits_widget_drag_events_from_gesture_signal() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            slider_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "slider::track");
        let x = rect.x + rect.w * 0.5;
        let y = rect.y + rect.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let start = ctx.handle_event(&AppEvent::MouseMove { x: x + 20.0, y });
        let move_output = ctx.handle_event(&AppEvent::MouseMove { x: x + 30.0, y });
        let end = ctx.handle_event(&AppEvent::MouseRelease {
            x: x + 30.0,
            y,
            button: MouseButton::Left,
        });

        assert!(matches!(
            start.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DragStart { id, .. })] if id == "slider"
        ));
        assert!(matches!(
            move_output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DragMove { id, .. })] if id == "slider"
        ));
        assert!(matches!(
            end.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DragEnd { id, .. })] if id == "slider"
        ));
    }

    #[test]
    fn context_emits_panel_drag_events_from_gesture_signal() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "panel::titlebar");
        let x = rect.x + rect.w * 0.5;
        let y = rect.y + rect.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let start = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 20.0,
            y: y + 4.0,
        });
        let move_output = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 30.0,
            y: y + 8.0,
        });
        let end = ctx.handle_event(&AppEvent::MouseRelease {
            x: x + 30.0,
            y: y + 8.0,
            button: MouseButton::Left,
        });

        assert!(matches!(
            start.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::DragStart { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            move_output.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::DragMove { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            end.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::DragEnd { id, .. })] if id == "panel"
        ));
    }

    #[test]
    fn context_emits_panel_resize_events_from_gesture_signal() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "panel");
        let x = rect.x + rect.w - 1.0;
        let y = rect.y + rect.h - 1.0;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let start = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 12.0,
            y: y + 12.0,
        });
        let move_output = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 20.0,
            y: y + 20.0,
        });
        let end = ctx.handle_event(&AppEvent::MouseRelease {
            x: x + 20.0,
            y: y + 20.0,
            button: MouseButton::Left,
        });

        assert!(matches!(
            start.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::ResizeStart { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            move_output.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::ResizeMove { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            end.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::ResizeEnd { id, .. })] if id == "panel"
        ));
    }

    #[test]
    fn overlay_open_is_composed_into_tree_and_hit_chain() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.open_overlay(OverlayRequest {
            id: "test_popup".to_string(),
            anchor_id: "button".to_string(),
            restore_focus_id: Some("button".to_string()),
            placement: OverlayPlacement::BelowStart,
            content: popup_content_desc(),
            offset_x: 0.0,
            offset_y: 8.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: true,
        });
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let popup_rect = node_rect(&ctx, "__overlay::test_popup");
        let chain = ctx.hit_test(popup_rect.x + 2.0, popup_rect.y + 2.0);
        assert!(chain.iter().any(|node_id| {
            ctx.node_name(node_id)
                .is_some_and(|id| id.starts_with("__overlay::test_popup"))
        }));
    }

    #[test]
    fn overlay_can_be_placed_at_pointer_position() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.open_overlay(OverlayRequest {
            id: "point_popup".to_string(),
            anchor_id: "canvas_root".to_string(),
            restore_focus_id: None,
            placement: OverlayPlacement::AtPoint { x: 44.0, y: 52.0 },
            content: popup_content_desc(),
            offset_x: 3.0,
            offset_y: 4.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: false,
        });
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let popup_rect = node_rect(&ctx, "__overlay::point_popup");
        assert_eq!(popup_rect.x, 47.0);
        assert_eq!(popup_rect.y, 56.0);
    }

    #[test]
    fn outside_click_dismisses_overlay() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.open_overlay(OverlayRequest {
            id: "test_popup".to_string(),
            anchor_id: "button".to_string(),
            restore_focus_id: Some("button".to_string()),
            placement: OverlayPlacement::BelowStart,
            content: popup_content_desc(),
            offset_x: 0.0,
            offset_y: 8.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: true,
        });
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: 300.0,
            y: 100.0,
            button: MouseButton::Left,
        });

        assert!(!ctx.overlay_open());
    }

    #[test]
    fn escape_dismisses_overlay_and_restores_focus() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.open_overlay(OverlayRequest {
            id: "test_popup".to_string(),
            anchor_id: "button".to_string(),
            restore_focus_id: Some("button".to_string()),
            placement: OverlayPlacement::BelowStart,
            content: popup_content_desc(),
            offset_x: 0.0,
            offset_y: 8.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: true,
        });
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.clear_focus();

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Escape,
            modifiers: Modifiers::default(),
        });

        assert!(!ctx.overlay_open());
        assert_eq!(ctx.focused_widget_id(), Some("button"));
    }

    #[test]
    fn scroll_events_update_scroll_area_offset() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            scroll_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 200.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "scroll");

        let output = ctx.handle_event(&AppEvent::ScrollPixel {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            delta_x: 0.0,
            delta_y: -24.0,
        });

        assert!(ctx.node_scroll_offset("scroll").expect("scroll node") > 0.0);
        assert!(output.consumed);
        assert!(output.events.is_empty());
    }

    #[test]
    fn unhandled_pointer_event_is_not_consumed() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let output = ctx.handle_event(&AppEvent::ScrollPixel {
            x: 300.0,
            y: 100.0,
            delta_x: 0.0,
            delta_y: -24.0,
        });

        assert!(!output.consumed);
        assert!(output.events.is_empty());
        assert!(output.effects.is_empty());
    }

    #[test]
    fn number_input_emits_number_change_for_valid_text() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            number_desc(1.5),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        focus_number(&mut ctx);
        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        let output = ctx.handle_event(&AppEvent::TextInput {
            text: "2".to_string(),
        });

        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::NumberChanged { id, value })]
                if id == "number" && (*value - 2.0).abs() < 0.0001
        ));
    }

    #[test]
    fn number_input_reverts_invalid_text_on_blur() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            number_desc(1.5),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        focus_number(&mut ctx);

        let output = ctx.handle_event(&AppEvent::TextInput {
            text: "a".to_string(),
        });
        assert!(output.events.is_empty());
        assert_eq!(
            ctx.systems
                .text_box_store()
                .runtime("number")
                .unwrap()
                .editor()
                .text(),
            "a1.50"
        );

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: 300.0,
            y: 100.0,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: 300.0,
            y: 100.0,
            button: MouseButton::Left,
        });

        assert_eq!(
            ctx.systems
                .text_box_store()
                .runtime("number")
                .unwrap()
                .editor()
                .text(),
            "1.50"
        );
    }

    #[test]
    fn number_input_step_uses_current_edited_value() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 120.0,
        };
        ctx.update(number_desc(1.5), viewport, &mut measurer, &theme);
        focus_number(&mut ctx);
        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        let _ = ctx.handle_event(&AppEvent::TextInput {
            text: "2.5".to_string(),
        });

        let output = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Up,
            modifiers: Modifiers::default(),
        });

        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::NumberChanged { id, value })]
                if id == "number" && (*value - 3.0).abs() < 0.0001
        ));
    }

    #[test]
    fn focused_number_input_does_not_clobber_dirty_editor_on_external_sync() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 120.0,
        };
        ctx.update(number_desc(1.5), viewport, &mut measurer, &theme);
        focus_number(&mut ctx);
        let _ = ctx.handle_event(&AppEvent::TextInput {
            text: "a".to_string(),
        });

        ctx.update(number_desc(2.0), viewport, &mut measurer, &theme);

        assert_eq!(
            ctx.systems
                .text_box_store()
                .runtime("number")
                .unwrap()
                .editor()
                .text(),
            "a1.50"
        );
        assert_eq!(
            ctx.systems
                .text_box_store()
                .runtime("number")
                .unwrap()
                .external_text(),
            "2.00"
        );
    }

    #[test]
    fn dropdown_click_open_and_select_emits_select_change() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 360.0,
        };
        ctx.update(dropdown_desc(0), viewport, &mut measurer, &theme);
        let rect = dropdown_field_rect(&ctx);

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        assert!(ctx.overlay_open());

        ctx.update(dropdown_desc(0), viewport, &mut measurer, &theme);
        let option_rect = node_rect(&ctx, "__dropdown_option::dropdown::1");
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: option_rect.x + 4.0,
            y: option_rect.y + option_rect.h * 0.5,
            button: MouseButton::Left,
        });
        let output = ctx.handle_event(&AppEvent::MouseRelease {
            x: option_rect.x + 4.0,
            y: option_rect.y + option_rect.h * 0.5,
            button: MouseButton::Left,
        });
        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::SelectionChanged { id, selected })]
                if id == "dropdown" && *selected == 1
        ));
        assert!(!ctx.overlay_open());
    }

    #[test]
    fn dropdown_keyboard_open_and_select_uses_highlighted_option() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 360.0,
        };
        ctx.update(dropdown_desc(0), viewport, &mut measurer, &theme);
        let dropdown_id = ctx.node_id_by_name("dropdown").expect("dropdown id");
        ctx.request_focus(dropdown_id);

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Enter,
            modifiers: Modifiers::default(),
        });
        assert!(ctx.overlay_open());

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Down,
            modifiers: Modifiers::default(),
        });
        let output = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Enter,
            modifiers: Modifiers::default(),
        });

        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::SelectionChanged { id, selected })]
                if id == "dropdown" && *selected == 1
        ));
    }
}
