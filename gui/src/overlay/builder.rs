use super::placement::resolve_placement;
use super::runtime::OverlayState;
use crate::tree::layout::Size;
use crate::tree::{Desc, Tree};
use crate::ui::{self, StyleBuilder};

const OVERLAY_ROOT_ID: &str = "__overlay_root";

pub(crate) fn compose_overlay_desc(
    state: &mut OverlayState,
    tree: &Tree,
    base_desc: Desc,
    viewport: crate::renderer::Rect,
) -> Desc {
    refresh_overlay_layout(state, tree);

    ui::container("__context_root")
        .fixed_width(viewport.w)
        .fixed_height(viewport.h)
        .children([base_desc, overlay_desc(state, viewport)])
        .build()
}

fn refresh_overlay_layout(state: &mut OverlayState, tree: &Tree) {
    let (x, y, width) = resolve_placement(
        tree,
        &state.request,
        (state.last_x, state.last_y, state.last_width),
    );
    state.last_x = x;
    state.last_y = y;
    state.last_width = width;
}

fn overlay_desc(state: &OverlayState, viewport: crate::renderer::Rect) -> Desc {
    ui::container(OVERLAY_ROOT_ID)
        .absolute_xy(0.0, 0.0)
        .fixed_width(viewport.w)
        .fixed_height(viewport.h)
        .hittable(false)
        .child(
            ui::container(format!("__overlay::{}", state.request.id))
                .absolute_xy(state.last_x, state.last_y)
                .width(state.last_width.map(Size::Fixed).unwrap_or(Size::Auto))
                .auto_height()
                .child(clone_desc(&state.request.content)),
        )
        .build()
}

fn clone_desc(desc: &Desc) -> Desc {
    desc.clone()
}
