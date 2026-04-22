use super::placement::resolve_placement;
use super::runtime::OverlayState;
use crate::tree::layout::{BoxStyle, Position, Size};
use crate::tree::{Desc, Tree};
use std::borrow::Cow;

const OVERLAY_ROOT_ID: &str = "__overlay_root";

pub(crate) fn compose_overlay_desc(
    state: &mut OverlayState,
    tree: &Tree,
    base_desc: Desc,
    viewport: crate::renderer::Rect,
) -> Desc {
    refresh_overlay_layout(state, tree);

    Desc::Container {
        id: Cow::Borrowed("__context_root"),
        style: BoxStyle {
            width: Size::Fixed(viewport.w),
            height: Size::Fixed(viewport.h),
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![base_desc, overlay_desc(state, viewport)],
    }
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
    Desc::Container {
        id: Cow::Borrowed(OVERLAY_ROOT_ID),
        style: BoxStyle {
            position: Position::absolute_xy(0.0, 0.0),
            width: Size::Fixed(viewport.w),
            height: Size::Fixed(viewport.h),
            hittable: Some(false),
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![Desc::Container {
            id: Cow::Owned(format!("__overlay::{}", state.request.id)),
            style: BoxStyle {
                position: Position::absolute_xy(state.last_x, state.last_y),
                width: state.last_width.map(Size::Fixed).unwrap_or(Size::Auto),
                height: Size::Auto,
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![clone_desc(&state.request.content)],
        }],
    }
}

fn clone_desc(desc: &Desc) -> Desc {
    match desc {
        Desc::Container {
            id,
            style,
            decoration,
            children,
        } => Desc::Container {
            id: id.clone(),
            style: style.clone(),
            decoration: decoration.clone(),
            children: children.iter().map(clone_desc).collect(),
        },
        Desc::Leaf { id, style, kind } => Desc::Leaf {
            id: id.clone(),
            style: style.clone(),
            kind: kind.clone(),
        },
        Desc::Widget { id, props } => Desc::Widget {
            id: id.clone(),
            props: props.clone_box(),
        },
        Desc::WidgetContainer {
            id,
            props,
            children,
        } => Desc::WidgetContainer {
            id: id.clone(),
            props: props.clone_box(),
            children: children.iter().map(clone_desc).collect(),
        },
    }
}
