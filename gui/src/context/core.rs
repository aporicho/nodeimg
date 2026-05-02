use super::api::{
    AnimationApi, AnimationMutApi, CanvasApi, CanvasMutApi, ControlsApi, ControlsMutApi, InputApi,
    OverlayApi, OverlayMutApi, PanelApi, PanelMutApi, QueryApi, RenderingApi, ResourcesApi,
    SceneApi,
};
use crate::animation::AnimationStore;
use crate::diagnostics::tree_dump::TreeDumpController;
use crate::gesture::GestureSession;
use crate::icon::IconRegistry;
use crate::interaction::InteractionState;
use crate::paint::DisplayList;
use crate::runtime::{ResourceRegistry, RuntimeSystems};
use crate::template::TemplateRegistry;
use crate::theme::Theme;
use crate::tree::{FrameStats, Tree};

pub struct Context {
    pub(crate) tree: Tree,
    pub(in crate::context) template_registry: TemplateRegistry,
    pub(in crate::context) animations: AnimationStore,
    pub(in crate::context) gesture_session: GestureSession,
    pub(in crate::context) interaction: InteractionState,
    pub(in crate::context) systems: RuntimeSystems,
    pub(in crate::context) resources: ResourceRegistry,
    pub(in crate::context) icons: IconRegistry,
    pub(in crate::context) retained_display_list: Option<DisplayList>,
    pub(in crate::context) last_paint_theme_revision: Option<u64>,
    pub(in crate::context) current_theme: Theme,
    pub(in crate::context) tree_dump: TreeDumpController,
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

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
