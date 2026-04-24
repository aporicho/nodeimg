use crate::workspace::view::build_workspace_tree;
use gui::canvas::camera::Camera;
use gui::canvas::node_template::CanvasNodeRenderView;
use gui::canvas::{CanvasConnectionView, CanvasPendingConnectionView};
use gui::renderer::Rect;
use gui::theme::Theme;
use gui::tree::Desc;

pub(crate) struct UserModeBuildContext<'a> {
    pub(crate) viewport: Rect,
    pub(crate) camera: &'a Camera,
    pub(crate) theme: &'a Theme,
    pub(crate) canvas_nodes: &'a [CanvasNodeRenderView],
    pub(crate) canvas_connections: &'a [CanvasConnectionView],
    pub(crate) pending_connection: Option<&'a CanvasPendingConnectionView>,
    pub(crate) panel_root: Desc,
}

pub(crate) fn build_user_page(ctx: UserModeBuildContext<'_>) -> Desc {
    build_workspace_tree(
        ctx.viewport,
        ctx.camera,
        ctx.theme,
        ctx.canvas_nodes,
        ctx.canvas_connections,
        ctx.pending_connection,
        ctx.panel_root,
    )
}
