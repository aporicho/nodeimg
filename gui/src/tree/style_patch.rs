use super::dirty::DirtyFlags;
use super::layout::{
    BoxStyle, Decoration, LayoutDependencyKind, LayoutDependencyScope, Position, Size,
};
use super::TreeNode;
use crate::geometry::TransformSpec;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StylePatch {
    pub position: Option<Position>,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub z_index: Option<i32>,
    pub transform: Option<Option<TransformSpec>>,
    pub decoration: Option<Option<Decoration>>,
    pub replace_box_style: Option<BoxStyle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StylePatchEffect {
    pub flags: DirtyFlags,
    pub layout_dependency: Option<(LayoutDependencyKind, LayoutDependencyScope)>,
}

pub(crate) fn apply_style_patch(node: &mut TreeNode, patch: StylePatch) -> StylePatchEffect {
    if let Some(style) = patch.replace_box_style {
        if node.style == style {
            return StylePatchEffect::default();
        }
        node.style = style;
        node.paint_meta.bump_visual();
        return StylePatchEffect {
            flags: DirtyFlags::STYLE | DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT,
            layout_dependency: Some((
                LayoutDependencyKind::Style,
                LayoutDependencyScope::RelayoutBoundary,
            )),
        };
    }

    let mut flags = DirtyFlags::NONE;
    let mut layout_dependency = None;

    if let Some(position) = patch.position {
        if node.style.position != position {
            node.style.position = position;
            node.paint_meta.bump_visual();
            flags |= DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT;
            layout_dependency = Some(LayoutDependencyScope::RelayoutBoundary);
        }
    }
    if let Some(width) = patch.width {
        if node.style.width != width {
            node.style.width = width;
            node.paint_meta.bump_visual();
            flags |= DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT;
            layout_dependency = Some(LayoutDependencyScope::RelayoutBoundary);
        }
    }
    if let Some(height) = patch.height {
        if node.style.height != height {
            node.style.height = height;
            node.paint_meta.bump_visual();
            flags |= DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT;
            layout_dependency = Some(LayoutDependencyScope::RelayoutBoundary);
        }
    }
    if let Some(z_index) = patch.z_index {
        if node.style.z_index != z_index {
            node.style.z_index = z_index;
            node.paint_meta.bump_paint_order();
            flags |= DirtyFlags::PAINT_ORDER | DirtyFlags::HIT | DirtyFlags::PAINT;
            layout_dependency = Some(layout_dependency.unwrap_or(LayoutDependencyScope::LocalNode));
        }
    }
    if let Some(transform) = patch.transform {
        if node.style.transform != transform {
            node.style.transform = transform;
            node.paint_meta.bump_visual();
            flags |= DirtyFlags::PAINT | DirtyFlags::HIT;
            layout_dependency = Some(layout_dependency.unwrap_or(LayoutDependencyScope::LocalNode));
        }
    }
    if let Some(decoration) = patch.decoration {
        if node.decoration != decoration {
            node.decoration = decoration;
            node.paint_meta.bump_visual();
            flags |= DirtyFlags::PAINT;
            layout_dependency = Some(layout_dependency.unwrap_or(LayoutDependencyScope::LocalNode));
        }
    }

    StylePatchEffect {
        flags,
        layout_dependency: layout_dependency.map(|scope| (LayoutDependencyKind::Style, scope)),
    }
}
