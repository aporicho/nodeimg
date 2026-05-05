use crate::geometry::Rect;
use crate::paint::TextStyle;
use crate::theme::Theme;
use crate::tree::node::NodeId;
use crate::tree::paint_target::PaintTarget;
use crate::tree::Tree;

pub(crate) trait TextLeafPaintOverride {
    fn paint_text_leaf(
        &self,
        target: &mut dyn PaintTarget,
        request: TextLeafPaintRequest<'_>,
    ) -> bool;
}

#[derive(Clone, Copy)]
pub(crate) struct TextLeafPaintRequest<'a> {
    tree: &'a Tree,
    node_id: NodeId,
    node_rect: Rect,
    theme: &'a Theme,
    text_style: TextStyle,
}

impl<'a> TextLeafPaintRequest<'a> {
    pub(crate) fn new(
        tree: &'a Tree,
        node_id: NodeId,
        node_rect: Rect,
        theme: &'a Theme,
        text_style: TextStyle,
    ) -> Self {
        Self {
            tree,
            node_id,
            node_rect,
            theme,
            text_style,
        }
    }

    pub(crate) fn tree(&self) -> &'a Tree {
        self.tree
    }

    pub(crate) fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub(crate) fn node_rect(&self) -> Rect {
        self.node_rect
    }

    pub(crate) fn theme(&self) -> &'a Theme {
        self.theme
    }

    pub(crate) fn text_style(&self) -> TextStyle {
        self.text_style
    }
}
