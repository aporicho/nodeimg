use super::node_card;
use super::node_template::CanvasNodeRenderView;
use crate::theme::Theme;
use crate::tree::Desc;

/// Legacy `Desc` canvas card boundary.
///
/// Owner: UI engine migration.
/// Deletion condition: `CanvasSceneModel`/`scene_diff` mounts node-card templates
/// directly for all workspace canvas nodes.
pub fn node_card_from_render_view(view: &CanvasNodeRenderView, theme: &Theme) -> Desc {
    node_card::node_card_from_render_view(view, theme)
}
