use std::sync::Arc;
use std::time::Instant;

use crate::animation::{AnimationBuilder, AnimationId, AnimationStore, TimelineBuilder};
use crate::control::{ControlIntrinsic, ControlRole, ResizeEdge};
use crate::diagnostics::render_trace::{self, RectSummary, RenderTraceStage, TARGET_RENDER};
use crate::diagnostics::tree_dump::{self, TreeDumpController, TreeDumpPhase, TreeDumpPoint};
use crate::event::router;
use crate::event::signal_output;
use crate::gesture::{Gesture, GestureSession, GestureSessionUpdate};
use crate::icon::{IconId, IconRegistry};
use crate::interaction::InteractionState;
use crate::paint::{DisplayList, PaintFlushStats};
use crate::renderer::{Rect, RegistryDisplayResources, Renderer, TextMeasurer, TextureSize};
use crate::runtime::{ResourceRegistry, RuntimeEventCx, RuntimeEventResult, RuntimeSystems};
use crate::scene::SceneMutation;
use crate::shell::AppEvent;
use crate::template::TemplateRegistry;
use crate::template::{InstanceId, SlotValue, SlotValues, TemplateId, WORKSPACE_ROOT_TEMPLATE};
use crate::theme::Theme;
use crate::tree::layout::{LayoutConstraints, LayoutFlushStats, LayoutOutput, TextureHandle};
use crate::tree::{
    hit_test_with_animations, resize_hit_at_screen_point, FrameStats, HitChain, Invalidation,
    MutationError, NodeId, PaintDirtyReason, RepaintBoundaryId, StylePatch, Tree, TreeMutation,
    TreeSnapshotOptions,
};

pub use crate::output::{ControlEvent, FrameworkOutput, GuiEvent, OverlayEvent, PlatformEffect};
pub use crate::overlay::{
    DropdownOverlayContent, OverlayContent, OverlayPlacement, OverlayRequest,
};

/// GUI 中心对象。持有统一的控件树与框架级交互 session。
pub struct Context {
    pub(crate) tree: Tree,
    pub(crate) template_registry: TemplateRegistry,
    animations: AnimationStore,
    gesture_session: GestureSession,
    interaction: InteractionState,
    pub(crate) systems: RuntimeSystems,
    pub(crate) resources: ResourceRegistry,
    icons: IconRegistry,
    retained_display_list: Option<DisplayList>,
    last_paint_theme_revision: Option<u64>,
    current_theme: Theme,
    tree_dump: TreeDumpController,
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
    dirty_placement: usize,
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
            resources: ResourceRegistry::new(),
            icons: IconRegistry::with_builtin_icons(),
            retained_display_list: None,
            last_paint_theme_revision: None,
            current_theme: Theme::default(),
            tree_dump: TreeDumpController::from_env(),
        }
    }

    pub fn maybe_dump_tree(&mut self, stage: RenderTraceStage, phase: TreeDumpPhase, reason: &str) {
        self.tree_dump
            .maybe_dump(&self.tree, TreeDumpPoint::new(stage, phase), reason);
    }

    pub fn dump_tree_now(
        &self,
        stage: RenderTraceStage,
        phase: TreeDumpPhase,
        reason: &str,
        options: TreeSnapshotOptions,
    ) {
        let snapshot = self.tree.debug_snapshot(options);
        tree_dump::emit_snapshot(&snapshot, TreeDumpPoint::new(stage, phase), reason);
    }

    pub(crate) fn flush_layout_dirty(
        &mut self,
        root_rect: Rect,
        measurer: &mut TextMeasurer,
    ) -> LayoutFlushStats {
        self.maybe_dump_tree(
            RenderTraceStage::LayoutFlush,
            TreeDumpPhase::Before,
            "before layout dirty flush",
        );
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
        self.maybe_dump_tree(
            RenderTraceStage::LayoutFlush,
            TreeDumpPhase::After,
            "after layout dirty flush",
        );
        stats
    }

    pub(crate) fn ensure_retained_root(
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
        self.sync_overlay_tree();
        self.retained_root_ids()
            .ok_or(MutationError::MissingNode(usize::MAX))
    }

    pub(crate) fn retained_root_ids(&self) -> Option<RetainedRootIds> {
        Some(RetainedRootIds {
            root: self.tree.node_by_str("root")?,
            canvas_root: self.tree.node_by_str("canvas_root")?,
            canvas_grid: self.tree.node_by_str("canvas_grid")?,
            canvas_connections: self.tree.node_by_str("canvas_connections")?,
            panel_root: self.tree.node_by_str("panel_root")?,
            overlay_root: self.tree.node_by_str("__overlay_root")?,
        })
    }

    pub(crate) fn apply_mutation(
        &mut self,
        mutation: TreeMutation,
    ) -> Result<Invalidation, MutationError> {
        self.tree.apply_mutation(&self.template_registry, mutation)
    }

    pub(crate) fn apply_mutations(
        &mut self,
        mutations: impl IntoIterator<Item = TreeMutation>,
    ) -> Result<Vec<Invalidation>, MutationError> {
        self.tree
            .apply_mutations(&self.template_registry, mutations)
    }

    fn sync_overlay_tree(&mut self) {
        if let Err(error) = self.systems.sync_overlay_tree(
            &mut self.tree,
            &self.template_registry,
            &self.current_theme,
        ) {
            tracing::warn!(
                target: TARGET_RENDER,
                ?error,
                "failed to sync retained overlay tree"
            );
        }
    }

    pub(crate) fn update_canvas_transform(
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
    pub(crate) fn render(
        &mut self,
        renderer: &mut Renderer,
        _viewport_w: f32,
        _viewport_h: f32,
        theme: &Theme,
    ) {
        if self.tree.root().is_some() {
            self.current_theme = theme.clone();
            self.sync_overlay_tree();
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
        self.current_theme = theme.clone();
        self.sync_overlay_tree();
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

        self.maybe_dump_tree(
            RenderTraceStage::PaintFlush,
            TreeDumpPhase::Before,
            "before paint dirty flush",
        );

        let root_boundary = self
            .tree
            .nearest_repaint_boundary(root)
            .unwrap_or(RepaintBoundaryId(root));
        let dirty = self.tree.take_paint_dirty();
        let dirty_boundaries = dirty.boundaries.len();
        let dirty_paint_order = dirty.paint_order.len();
        let dirty_composite = dirty.composite.len();
        let dirty_placement = dirty.placement.len();
        let mut rebuild = dirty.boundaries;
        rebuild.extend(dirty.paint_order);
        rebuild.extend(dirty.placement);

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
                dirty_placement,
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

    pub(crate) fn register_texture(
        &mut self,
        handle: TextureHandle,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
    ) {
        self.resources.register_texture(handle, view, size);
    }

    pub(crate) fn register_svg_icon(&mut self, id: impl Into<IconId>, svg: impl AsRef<[u8]>) {
        self.icons.register_svg(id, svg);
    }

    pub(crate) fn open_overlay(&mut self, request: OverlayRequest) {
        self.systems.open_overlay(&mut self.tree, request);
        self.sync_overlay_tree();
    }

    pub(crate) fn close_overlay(&mut self) {
        self.systems
            .close_overlay(&mut self.tree, &mut self.interaction);
    }

    pub(crate) fn overlay_open(&self) -> bool {
        self.systems.overlay_open()
    }

    pub(crate) fn handle_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        let focused_before = self.focused_control_id().map(str::to_string);
        let output = router::handle_event(self, event);
        self.sync_overlay_tree();
        if matches!(
            event,
            AppEvent::MousePress { .. }
                | AppEvent::KeyPress { .. }
                | AppEvent::TextInput { .. }
                | AppEvent::ImePreedit { .. }
        ) {
            tracing::trace!(
                target: "nodeimg::render_trace::input",
                ?event,
                focused_before = focused_before.as_deref(),
                focused_after = self.focused_control_id(),
                events = output.events.len(),
                consumed = output.consumed,
                "gui input event handled"
            );
        }
        output
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

    pub(crate) fn tick_animations(&mut self, now: Instant) -> bool {
        self.animations.tick(now)
    }

    pub(crate) fn animations_active(&self) -> bool {
        self.animations.active()
    }

    pub(crate) fn ime_request(&self) -> ImeRequest {
        self.systems
            .ime_request(&self.tree, self.interaction.focused())
    }

    pub(crate) fn paste_focused_text(&mut self, text: &str) -> FrameworkOutput {
        self.systems
            .paste_focused_text(&self.tree, self.interaction.focused(), text)
    }

    pub(crate) fn handle_panel_control_event(&mut self, event: &ControlEvent) -> bool {
        crate::panel::event::apply_panel_control_event(&mut self.tree, event)
    }

    pub(crate) fn sync_canvas_node_layouts(
        &mut self,
        identities: &[crate::canvas::CanvasNodeIdentity],
    ) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.sync_canvas_node_layouts(identities)
    }

    pub(crate) fn export_canvas_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.export_canvas_node_layouts()
    }

    pub(crate) fn import_canvas_node_layouts(
        &mut self,
        layouts: &[crate::canvas::CanvasNodeLayout],
    ) {
        self.tree.import_canvas_node_layouts(layouts);
    }

    pub(crate) fn move_canvas_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        self.tree.move_canvas_node_by(owner_id, dx, dy)
    }

    pub(crate) fn resize_canvas_node_by(
        &mut self,
        owner_id: &str,
        edge: ResizeEdge,
        dx: f32,
        dy: f32,
    ) -> bool {
        self.tree.resize_canvas_node_by(owner_id, edge, dx, dy)
    }

    pub(crate) fn ensure_canvas_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        self.tree
            .ensure_canvas_node_min_size(owner_id, min_width, min_height)
    }

    pub(crate) fn apply_canvas_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        self.tree.apply_canvas_node_sizing(owner_id, request)
    }

    pub(crate) fn control_intrinsics(&self) -> Vec<ControlIntrinsic> {
        self.systems.control_intrinsics()
    }

    pub(crate) fn take_dirty_control_intrinsics(&mut self) -> Vec<ControlIntrinsic> {
        self.systems.take_dirty_control_intrinsics()
    }

    pub(crate) fn sync_canvas_text_boxes(
        &mut self,
        views: &[crate::canvas::node_template::CanvasNodeRenderView],
        measurer: &mut TextMeasurer,
        theme: &Theme,
    ) {
        let before_dirty = self.systems.dirty_control_intrinsic_ids().len();
        self.systems.sync_canvas_text_boxes(
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
                dirty_intrinsics_after: self.systems.dirty_control_intrinsic_ids().len(),
            },
        );
    }

    pub(crate) fn has_dirty_control_intrinsics(&self) -> bool {
        self.systems.has_dirty_control_intrinsics()
    }

    pub(crate) fn canvas_port_group_view(
        &self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> crate::canvas::CanvasPortGroupView {
        self.tree.canvas_port_group_view(owner_id, side)
    }

    pub(crate) fn toggle_canvas_port_group(
        &mut self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> bool {
        self.tree.toggle_canvas_port_group(owner_id, side)
    }

    pub(crate) fn select_canvas_node(&mut self, owner_id: &str) -> bool {
        self.tree.select_canvas_node(owner_id)
    }

    pub(crate) fn clear_canvas_selection(&mut self) {
        self.tree.clear_canvas_selection();
    }

    pub(crate) fn is_canvas_node_selected(&self, owner_id: &str) -> bool {
        self.tree.is_canvas_node_selected(owner_id)
    }

    pub(crate) fn pending_canvas_connection(
        &self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.pending_canvas_connection()
    }

    pub(crate) fn begin_pending_canvas_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.tree
            .begin_pending_canvas_connection(from_port_id, cursor_canvas)
    }

    pub(crate) fn update_pending_canvas_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        self.tree.update_pending_canvas_connection(cursor_canvas)
    }

    pub(crate) fn end_pending_canvas_connection(
        &mut self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.end_pending_canvas_connection()
    }

    pub(crate) fn cancel_pending_canvas_connection(&mut self) -> bool {
        self.tree.cancel_pending_canvas_connection()
    }

    pub(crate) fn hovered_canvas_port_id(&self) -> Option<String> {
        self.tree.hovered_canvas_port_id()
    }

    pub(crate) fn set_hovered_canvas_port(&mut self, port_id: Option<&str>) -> bool {
        self.tree.set_hovered_canvas_port(port_id)
    }

    pub(crate) fn export_panel_layouts(&self) -> Vec<crate::panel::PanelLayout> {
        self.tree.export_panel_layouts()
    }

    pub(crate) fn ensure_panel_runtime(
        &mut self,
        config: &crate::panel::PanelConfig,
    ) -> Option<crate::panel::PanelRuntime> {
        self.tree.ensure_panel(config);
        self.tree.panel_state(config.id.as_str()).cloned()
    }

    pub(crate) fn panel_runtime(&self, id: &str) -> Option<crate::panel::PanelRuntime> {
        self.tree.panel_state(id).cloned()
    }

    pub(crate) fn import_panel_layouts(&mut self, layouts: &[crate::panel::PanelLayout]) {
        self.tree.import_panel_layouts(layouts);
    }

    /// 命中测试，返回从叶子到根的命中链。
    pub(crate) fn hit_test(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test_with_animations(&self.tree, root, x, y, Some(&self.animations))
    }

    pub(crate) fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub(crate) fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.tree.node_by_str(id)
    }

    pub(crate) fn node_exists(&self, id: &str) -> bool {
        self.node_id_by_name(id).is_some()
    }

    pub(crate) fn node_rect(&self, id: &str) -> Option<Rect> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.node_rect_by_node(node_id))
    }

    pub(crate) fn node_rect_by_node(&self, node_id: NodeId) -> Option<Rect> {
        self.tree.get(node_id).map(|node| node.rect)
    }

    pub(crate) fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.tree.get(node_id).map(|node| node.id.as_ref())
    }

    pub(crate) fn node_scroll_offset(&self, id: &str) -> Option<f32> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.tree.get(node_id))
            .map(|node| node.scroll_offset())
    }

    pub(crate) fn node_has_gesture(&self, node_id: NodeId, gesture: Gesture) -> bool {
        self.tree
            .get(node_id)
            .map(|node| node.style.gestures.contains(&gesture))
            .unwrap_or(false)
    }

    pub(crate) fn node_is_draggable(&self, node_id: NodeId) -> bool {
        self.tree
            .get(node_id)
            .map(|node| node.style.draggable)
            .unwrap_or(false)
    }

    pub(crate) fn node_is_resizable(&self, node_id: NodeId) -> bool {
        self.tree
            .get(node_id)
            .map(|node| node.style.resizable)
            .unwrap_or(false)
    }

    pub(crate) fn resize_hit_at_screen_point(
        &self,
        x: f32,
        y: f32,
    ) -> Option<(NodeId, ResizeEdge)> {
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

    pub(crate) fn node_root_control_role(&self, node_id: NodeId) -> Option<ControlRole> {
        let mut candidate = self.node_name(node_id)?;

        loop {
            if let Some(root_id) = self.node_id_by_name(candidate) {
                if let Some(root_node) = self.tree.get(root_id) {
                    if let Some(role) = root_node.props.semantic_role {
                        return Some(role);
                    }
                }
            }

            let (prefix, _) = candidate.rsplit_once("::")?;
            candidate = prefix;
        }
    }

    pub(crate) fn node_is_text_input_field(&self, node_id: NodeId) -> bool {
        self.node_name(node_id)
            .is_some_and(|id| id.ends_with("::field"))
            && self
                .node_root_control_role(node_id)
                .is_some_and(ControlRole::is_text_input)
    }

    pub(crate) fn node_is_text_area_field(&self, node_id: NodeId) -> bool {
        self.node_name(node_id)
            .is_some_and(|id| id.ends_with("::field"))
            && self
                .node_root_control_role(node_id)
                .is_some_and(ControlRole::is_text_area)
    }

    pub(crate) fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub(crate) fn focused_control_id(&self) -> Option<&str> {
        self.node_name_for(self.focused_node())
    }

    pub(crate) fn hovered_node(&self) -> Option<NodeId> {
        self.interaction.hovered()
    }

    pub(crate) fn hovered_control_id(&self) -> Option<&str> {
        self.node_name_for(self.hovered_node())
    }

    pub(crate) fn captured_node(&self) -> Option<NodeId> {
        self.interaction.captured()
    }

    pub(crate) fn captured_control_id(&self) -> Option<&str> {
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
            .map(|signal| signal_output::gesture_signal_output(&self.tree, signal))
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

    pub fn scene(&mut self) -> SceneApi<'_> {
        SceneApi { ctx: self }
    }

    pub fn query(&self) -> QueryApi<'_> {
        QueryApi { ctx: self }
    }

    pub fn input(&mut self) -> InputApi<'_> {
        InputApi { ctx: self }
    }

    pub fn rendering(&mut self) -> RenderingApi<'_> {
        RenderingApi { ctx: self }
    }

    pub fn canvas(&self) -> CanvasApi<'_> {
        CanvasApi { ctx: self }
    }

    pub fn canvas_mut(&mut self) -> CanvasMutApi<'_> {
        CanvasMutApi { ctx: self }
    }

    pub fn panel(&self) -> PanelApi<'_> {
        PanelApi { ctx: self }
    }

    pub fn panel_mut(&mut self) -> PanelMutApi<'_> {
        PanelMutApi { ctx: self }
    }

    pub fn resources_mut(&mut self) -> ResourcesApi<'_> {
        ResourcesApi { ctx: self }
    }

    pub fn overlay(&self) -> OverlayApi<'_> {
        OverlayApi { ctx: self }
    }

    pub fn overlay_mut(&mut self) -> OverlayMutApi<'_> {
        OverlayMutApi { ctx: self }
    }

    pub fn animations(&self) -> AnimationApi<'_> {
        AnimationApi { ctx: self }
    }

    pub fn animations_mut(&mut self) -> AnimationMutApi<'_> {
        AnimationMutApi { ctx: self }
    }

    pub fn controls(&self) -> ControlsApi<'_> {
        ControlsApi { ctx: self }
    }

    pub fn controls_mut(&mut self) -> ControlsMutApi<'_> {
        ControlsMutApi { ctx: self }
    }
}

pub struct SceneApi<'a> {
    ctx: &'a mut Context,
}

impl SceneApi<'_> {
    pub fn ensure_retained_root(
        &mut self,
        viewport: Rect,
    ) -> Result<RetainedRootIds, MutationError> {
        self.ctx.ensure_retained_root(viewport)
    }

    pub fn apply(&mut self, mutation: SceneMutation) -> Result<Invalidation, MutationError> {
        self.ctx.apply_mutation(mutation)
    }

    pub fn apply_many(
        &mut self,
        mutations: impl IntoIterator<Item = SceneMutation>,
    ) -> Result<Vec<Invalidation>, MutationError> {
        self.ctx.apply_mutations(mutations)
    }

    pub fn update_canvas_transform(
        &mut self,
        transform: crate::geometry::TransformSpec,
    ) -> Result<(), MutationError> {
        self.ctx.update_canvas_transform(transform)
    }
}

pub struct QueryApi<'a> {
    ctx: &'a Context,
}

impl QueryApi<'_> {
    pub fn root(&self) -> Option<NodeId> {
        self.ctx.root()
    }

    pub fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.ctx.node_id_by_name(id)
    }

    pub fn node_exists(&self, id: &str) -> bool {
        self.ctx.node_exists(id)
    }

    pub fn node_rect(&self, id: &str) -> Option<Rect> {
        self.ctx.node_rect(id)
    }

    pub fn node_rect_by_node(&self, node_id: NodeId) -> Option<Rect> {
        self.ctx.node_rect_by_node(node_id)
    }

    pub fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.ctx.node_name(node_id)
    }

    pub fn node_scroll_offset(&self, id: &str) -> Option<f32> {
        self.ctx.node_scroll_offset(id)
    }

    pub fn node_has_gesture(&self, node_id: NodeId, gesture: Gesture) -> bool {
        self.ctx.node_has_gesture(node_id, gesture)
    }

    pub fn node_is_draggable(&self, node_id: NodeId) -> bool {
        self.ctx.node_is_draggable(node_id)
    }

    pub fn node_is_resizable(&self, node_id: NodeId) -> bool {
        self.ctx.node_is_resizable(node_id)
    }

    pub fn hit_test(&self, x: f32, y: f32) -> HitChain {
        self.ctx.hit_test(x, y)
    }

    pub fn resize_hit_at_screen_point(&self, x: f32, y: f32) -> Option<(NodeId, ResizeEdge)> {
        self.ctx.resize_hit_at_screen_point(x, y)
    }

    pub fn node_root_control_role(&self, node_id: NodeId) -> Option<ControlRole> {
        self.ctx.node_root_control_role(node_id)
    }

    pub fn node_is_text_input_field(&self, node_id: NodeId) -> bool {
        self.ctx.node_is_text_input_field(node_id)
    }

    pub fn node_is_text_area_field(&self, node_id: NodeId) -> bool {
        self.ctx.node_is_text_area_field(node_id)
    }

    pub fn focused_node(&self) -> Option<NodeId> {
        self.ctx.focused_node()
    }

    pub fn focused_control_id(&self) -> Option<&str> {
        self.ctx.focused_control_id()
    }

    pub fn hovered_node(&self) -> Option<NodeId> {
        self.ctx.hovered_node()
    }

    pub fn hovered_control_id(&self) -> Option<&str> {
        self.ctx.hovered_control_id()
    }

    pub fn captured_node(&self) -> Option<NodeId> {
        self.ctx.captured_node()
    }

    pub fn captured_control_id(&self) -> Option<&str> {
        self.ctx.captured_control_id()
    }
}

pub struct InputApi<'a> {
    ctx: &'a mut Context,
}

impl InputApi<'_> {
    pub fn handle_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        self.ctx.handle_event(event)
    }

    pub fn ime_request(&self) -> ImeRequest {
        self.ctx.ime_request()
    }

    pub fn paste_focused_text(&mut self, text: &str) -> FrameworkOutput {
        self.ctx.paste_focused_text(text)
    }

    pub fn request_focus(&mut self, node_id: NodeId) {
        self.ctx.request_focus(node_id);
    }
}

pub struct RenderingApi<'a> {
    ctx: &'a mut Context,
}

impl RenderingApi<'_> {
    pub fn flush_layout_dirty(
        &mut self,
        root_rect: Rect,
        measurer: &mut TextMeasurer,
    ) -> LayoutFlushStats {
        self.ctx.flush_layout_dirty(root_rect, measurer)
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        viewport_w: f32,
        viewport_h: f32,
        theme: &Theme,
    ) {
        self.ctx.render(renderer, viewport_w, viewport_h, theme)
    }
}

pub struct CanvasApi<'a> {
    ctx: &'a Context,
}

impl CanvasApi<'_> {
    pub fn export_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.ctx.export_canvas_node_layouts()
    }

    pub fn port_group_view(
        &self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> crate::canvas::CanvasPortGroupView {
        self.ctx.canvas_port_group_view(owner_id, side)
    }

    pub fn is_node_selected(&self, owner_id: &str) -> bool {
        self.ctx.is_canvas_node_selected(owner_id)
    }

    pub fn pending_connection(&self) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.ctx.pending_canvas_connection()
    }

    pub fn hovered_port_id(&self) -> Option<String> {
        self.ctx.hovered_canvas_port_id()
    }
}

pub struct CanvasMutApi<'a> {
    ctx: &'a mut Context,
}

impl CanvasMutApi<'_> {
    pub fn sync_node_layouts(
        &mut self,
        identities: &[crate::canvas::CanvasNodeIdentity],
    ) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.ctx.sync_canvas_node_layouts(identities)
    }

    pub fn export_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.ctx.export_canvas_node_layouts()
    }

    pub fn import_node_layouts(&mut self, layouts: &[crate::canvas::CanvasNodeLayout]) {
        self.ctx.import_canvas_node_layouts(layouts);
    }

    pub fn move_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        self.ctx.move_canvas_node_by(owner_id, dx, dy)
    }

    pub fn resize_node_by(&mut self, owner_id: &str, edge: ResizeEdge, dx: f32, dy: f32) -> bool {
        self.ctx.resize_canvas_node_by(owner_id, edge, dx, dy)
    }

    pub fn ensure_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        self.ctx
            .ensure_canvas_node_min_size(owner_id, min_width, min_height)
    }

    pub fn apply_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        self.ctx.apply_canvas_node_sizing(owner_id, request)
    }

    pub fn toggle_port_group(
        &mut self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> bool {
        self.ctx.toggle_canvas_port_group(owner_id, side)
    }

    pub fn select_node(&mut self, owner_id: &str) -> bool {
        self.ctx.select_canvas_node(owner_id)
    }

    pub fn clear_selection(&mut self) {
        self.ctx.clear_canvas_selection();
    }

    pub fn begin_pending_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.ctx
            .begin_pending_canvas_connection(from_port_id, cursor_canvas)
    }

    pub fn update_pending_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        self.ctx.update_pending_canvas_connection(cursor_canvas)
    }

    pub fn end_pending_connection(&mut self) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.ctx.end_pending_canvas_connection()
    }

    pub fn cancel_pending_connection(&mut self) -> bool {
        self.ctx.cancel_pending_canvas_connection()
    }

    pub fn set_hovered_port(&mut self, port_id: Option<&str>) -> bool {
        self.ctx.set_hovered_canvas_port(port_id)
    }
}

pub struct PanelApi<'a> {
    ctx: &'a Context,
}

impl PanelApi<'_> {
    pub fn export_layouts(&self) -> Vec<crate::panel::PanelLayout> {
        self.ctx.export_panel_layouts()
    }

    pub fn runtime(&self, id: &str) -> Option<crate::panel::PanelRuntime> {
        self.ctx.panel_runtime(id)
    }
}

pub struct PanelMutApi<'a> {
    ctx: &'a mut Context,
}

impl PanelMutApi<'_> {
    pub fn handle_control_event(&mut self, event: &ControlEvent) -> bool {
        self.ctx.handle_panel_control_event(event)
    }

    pub fn ensure_runtime(
        &mut self,
        config: &crate::panel::PanelConfig,
    ) -> Option<crate::panel::PanelRuntime> {
        self.ctx.ensure_panel_runtime(config)
    }

    pub fn import_layouts(&mut self, layouts: &[crate::panel::PanelLayout]) {
        self.ctx.import_panel_layouts(layouts);
    }
}

pub struct ResourcesApi<'a> {
    ctx: &'a mut Context,
}

impl ResourcesApi<'_> {
    pub fn register_texture(
        &mut self,
        handle: TextureHandle,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
    ) {
        self.ctx.register_texture(handle, view, size);
    }

    pub fn register_svg_icon(&mut self, id: impl Into<IconId>, svg: impl AsRef<[u8]>) {
        self.ctx.register_svg_icon(id, svg);
    }
}

pub struct OverlayApi<'a> {
    ctx: &'a Context,
}

impl OverlayApi<'_> {
    pub fn is_open(&self) -> bool {
        self.ctx.overlay_open()
    }
}

pub struct OverlayMutApi<'a> {
    ctx: &'a mut Context,
}

impl OverlayMutApi<'_> {
    pub fn open(&mut self, request: OverlayRequest) {
        self.ctx.open_overlay(request);
    }

    pub fn close(&mut self) {
        self.ctx.close_overlay();
    }
}

pub struct AnimationApi<'a> {
    ctx: &'a Context,
}

impl AnimationApi<'_> {
    pub fn active(&self) -> bool {
        self.ctx.animations_active()
    }
}

pub struct AnimationMutApi<'a> {
    ctx: &'a mut Context,
}

impl AnimationMutApi<'_> {
    pub fn tick(&mut self, now: Instant) -> bool {
        self.ctx.tick_animations(now)
    }
}

pub struct ControlsApi<'a> {
    ctx: &'a Context,
}

impl ControlsApi<'_> {
    pub fn intrinsics(&self) -> Vec<ControlIntrinsic> {
        self.ctx.control_intrinsics()
    }

    pub fn has_dirty_intrinsics(&self) -> bool {
        self.ctx.has_dirty_control_intrinsics()
    }
}

pub struct ControlsMutApi<'a> {
    ctx: &'a mut Context,
}

impl ControlsMutApi<'_> {
    pub fn take_dirty_intrinsics(&mut self) -> Vec<ControlIntrinsic> {
        self.ctx.take_dirty_control_intrinsics()
    }

    pub fn sync_canvas_text_boxes(
        &mut self,
        views: &[crate::canvas::node_template::CanvasNodeRenderView],
        measurer: &mut TextMeasurer,
        theme: &Theme,
    ) {
        self.ctx.sync_canvas_text_boxes(views, measurer, theme);
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
