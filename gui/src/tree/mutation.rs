use super::dirty::DirtyFlags;
use super::layout::{BoxStyle, Decoration, LeafKind, Position, Size};
use super::{NodeId, NodeKind, Tree};
use crate::diagnostics::render_trace::{self, RenderTraceStage, TextPayloadSummary};
use crate::geometry::TransformSpec;
use crate::renderer::{Point, Rect};
use crate::template::{InstanceId, TemplateError, TemplateId, TemplatePayload, TemplateRegistry};

#[derive(Clone, Debug)]
pub enum TreeMutation {
    MountTemplate {
        parent: NodeId,
        template: TemplateId,
        instance: InstanceId,
        payload: TemplatePayload,
    },
    Unmount {
        node: NodeId,
    },
    SetRect {
        node: NodeId,
        rect: Rect,
    },
    SetStyle {
        node: NodeId,
        patch: StylePatch,
    },
    SetText {
        node: NodeId,
        value: String,
    },
    SetVisible {
        node: NodeId,
        visible: bool,
    },
    SetZIndex {
        node: NodeId,
        z_index: i32,
    },
    SetConnection {
        node: NodeId,
        from_port: String,
        to_port: String,
    },
    SetPendingConnection {
        node: NodeId,
        from_port: String,
        cursor_canvas: Point,
    },
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invalidation {
    pub node: NodeId,
    pub flags: DirtyFlags,
}

#[derive(Debug)]
pub enum MutationError {
    MissingNode(NodeId),
    Template(TemplateError),
    UnsupportedNodeKind {
        node: NodeId,
        operation: &'static str,
    },
}

impl From<TemplateError> for MutationError {
    fn from(value: TemplateError) -> Self {
        Self::Template(value)
    }
}

impl Tree {
    pub fn apply_mutation(
        &mut self,
        registry: &TemplateRegistry,
        mutation: TreeMutation,
    ) -> Result<Invalidation, MutationError> {
        let trace_enabled = render_trace::is_debug_enabled();
        let mutation_kind = mutation.kind();
        let target_node = mutation.target_node();
        let target_stable_id = trace_enabled
            .then(|| self.get(target_node).map(|node| node.id.to_string()))
            .flatten();
        let text_payload = trace_enabled
            .then(|| mutation.text_payload_summary())
            .flatten();

        let result = (|| match mutation {
            TreeMutation::MountTemplate {
                parent,
                template,
                instance,
                payload,
            } => {
                registry.instantiate_payload(self, template, instance, parent, payload)?;
                let flags = DirtyFlags::STRUCTURE
                    | DirtyFlags::LAYOUT
                    | DirtyFlags::HIT
                    | DirtyFlags::PAINT;
                self.mark_dirty(parent, flags);
                Ok(Invalidation {
                    node: parent,
                    flags,
                })
            }
            TreeMutation::Unmount { node } => {
                if self.get(node).is_none() {
                    return Err(MutationError::MissingNode(node));
                }
                self.detach_from_parent(node);
                self.remove(node);
                let flags = DirtyFlags::STRUCTURE
                    | DirtyFlags::LAYOUT
                    | DirtyFlags::HIT
                    | DirtyFlags::PAINT;
                let dirty_node = self.root().unwrap_or(node);
                self.mark_dirty(dirty_node, flags);
                Ok(Invalidation {
                    node: dirty_node,
                    flags,
                })
            }
            TreeMutation::SetRect { node, rect } => {
                let old = self.get(node).ok_or(MutationError::MissingNode(node))?.rect;
                if let Some(target) = self.get_mut(node) {
                    target.rect = rect;
                    target.layout_meta.bump_explicit_rect();
                    if old.w != rect.w || old.h != rect.h {
                        target.layout_meta.bump_style();
                        target.paint_meta.bump_visual();
                    }
                }
                let flags = rect_dirty_flags(old, rect);
                self.mark_dirty(node, flags);
                Ok(Invalidation { node, flags })
            }
            TreeMutation::SetStyle { node, patch } => {
                let target = self.get_mut(node).ok_or(MutationError::MissingNode(node))?;
                let flags = apply_style_patch(target, patch);
                self.mark_dirty(node, flags);
                Ok(Invalidation { node, flags })
            }
            TreeMutation::SetText { node, value } => {
                let target = self.get_mut(node).ok_or(MutationError::MissingNode(node))?;
                let NodeKind::Leaf(LeafKind::Text { content, .. }) = &mut target.kind else {
                    return Err(MutationError::UnsupportedNodeKind {
                        node,
                        operation: "SetText",
                    });
                };
                if *content != value {
                    *content = value;
                    target.layout_meta.bump_text();
                    target.paint_meta.bump_text();
                }
                let flags = DirtyFlags::TEXT_LAYOUT | DirtyFlags::PAINT;
                self.mark_dirty(node, flags);
                Ok(Invalidation { node, flags })
            }
            TreeMutation::SetVisible { node, visible } => {
                let target = self.get_mut(node).ok_or(MutationError::MissingNode(node))?;
                target.local_runtime.visible = visible;
                let flags = DirtyFlags::HIT | DirtyFlags::PAINT;
                self.mark_dirty(node, flags);
                Ok(Invalidation { node, flags })
            }
            TreeMutation::SetZIndex { node, z_index } => {
                let target = self.get_mut(node).ok_or(MutationError::MissingNode(node))?;
                target.style.z_index = z_index;
                target.layout_meta.bump_style();
                target.paint_meta.bump_paint_order();
                let flags = DirtyFlags::PAINT_ORDER | DirtyFlags::HIT | DirtyFlags::PAINT;
                self.mark_dirty(node, flags);
                Ok(Invalidation { node, flags })
            }
            TreeMutation::SetConnection {
                node,
                from_port,
                to_port,
            } => {
                let target = self.get_mut(node).ok_or(MutationError::MissingNode(node))?;
                let NodeKind::Leaf(LeafKind::Connection {
                    from_port: target_from,
                    to_port: target_to,
                }) = &mut target.kind
                else {
                    return Err(MutationError::UnsupportedNodeKind {
                        node,
                        operation: "SetConnection",
                    });
                };
                *target_from = std::borrow::Cow::Owned(from_port);
                *target_to = std::borrow::Cow::Owned(to_port);
                target.paint_meta.bump_visual();
                let flags = DirtyFlags::PAINT | DirtyFlags::HIT;
                self.mark_dirty(node, flags);
                Ok(Invalidation { node, flags })
            }
            TreeMutation::SetPendingConnection {
                node,
                from_port,
                cursor_canvas,
            } => {
                let target = self.get_mut(node).ok_or(MutationError::MissingNode(node))?;
                let NodeKind::Leaf(LeafKind::PendingConnection {
                    from_port: target_from,
                    cursor_canvas: target_cursor,
                }) = &mut target.kind
                else {
                    return Err(MutationError::UnsupportedNodeKind {
                        node,
                        operation: "SetPendingConnection",
                    });
                };
                *target_from = std::borrow::Cow::Owned(from_port);
                *target_cursor = cursor_canvas;
                target.paint_meta.bump_visual();
                let flags = DirtyFlags::PAINT | DirtyFlags::HIT;
                self.mark_dirty(node, flags);
                Ok(Invalidation { node, flags })
            }
        })();

        if trace_enabled {
            log_mutation_trace(
                mutation_kind,
                target_node,
                target_stable_id.as_deref(),
                text_payload,
                &result,
            );
        }

        result
    }

    pub fn apply_mutations(
        &mut self,
        registry: &TemplateRegistry,
        mutations: impl IntoIterator<Item = TreeMutation>,
    ) -> Result<Vec<Invalidation>, MutationError> {
        mutations
            .into_iter()
            .map(|mutation| self.apply_mutation(registry, mutation))
            .collect()
    }
}

impl TreeMutation {
    fn kind(&self) -> &'static str {
        match self {
            Self::MountTemplate { .. } => "MountTemplate",
            Self::Unmount { .. } => "Unmount",
            Self::SetRect { .. } => "SetRect",
            Self::SetStyle { .. } => "SetStyle",
            Self::SetText { .. } => "SetText",
            Self::SetVisible { .. } => "SetVisible",
            Self::SetZIndex { .. } => "SetZIndex",
            Self::SetConnection { .. } => "SetConnection",
            Self::SetPendingConnection { .. } => "SetPendingConnection",
        }
    }

    fn target_node(&self) -> NodeId {
        match self {
            Self::MountTemplate { parent, .. } => *parent,
            Self::Unmount { node }
            | Self::SetRect { node, .. }
            | Self::SetStyle { node, .. }
            | Self::SetText { node, .. }
            | Self::SetVisible { node, .. }
            | Self::SetZIndex { node, .. }
            | Self::SetConnection { node, .. }
            | Self::SetPendingConnection { node, .. } => *node,
        }
    }

    fn text_payload_summary(&self) -> Option<TextPayloadSummary> {
        match self {
            Self::SetText { value, .. } => Some(TextPayloadSummary::new(value)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct MutationTraceSummary<'a> {
    kind: &'static str,
    target_node: NodeId,
    target_stable_id: Option<&'a str>,
    text_payload: Option<TextPayloadSummary>,
    invalidated_node: Option<NodeId>,
    flags: Option<String>,
    error: Option<&'a str>,
}

fn log_mutation_trace(
    kind: &'static str,
    target_node: NodeId,
    target_stable_id: Option<&str>,
    text_payload: Option<TextPayloadSummary>,
    result: &Result<Invalidation, MutationError>,
) {
    let (invalidated_node, flags, error) = match result {
        Ok(invalidation) => (
            Some(invalidation.node),
            Some(invalidation.flags.to_string()),
            None,
        ),
        Err(MutationError::MissingNode(_)) => (None, None, Some("missing node")),
        Err(MutationError::Template(_)) => (None, None, Some("template error")),
        Err(MutationError::UnsupportedNodeKind { .. }) => {
            (None, None, Some("unsupported node kind"))
        }
    };
    render_trace::debug_stage(
        RenderTraceStage::TreeMutation,
        MutationTraceSummary {
            kind,
            target_node,
            target_stable_id,
            text_payload,
            invalidated_node,
            flags,
            error,
        },
    );
}

fn rect_dirty_flags(old: Rect, new: Rect) -> DirtyFlags {
    let size_changed = old.w != new.w || old.h != new.h;
    let position_changed = old.x != new.x || old.y != new.y;
    if size_changed {
        DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT
    } else if position_changed {
        DirtyFlags::COMPOSITE | DirtyFlags::HIT
    } else {
        DirtyFlags::NONE
    }
}

fn apply_style_patch(node: &mut super::TreeNode, patch: StylePatch) -> DirtyFlags {
    if let Some(style) = patch.replace_box_style {
        node.style = style;
        node.layout_meta.bump_style();
        node.paint_meta.bump_visual();
        return DirtyFlags::STYLE | DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT;
    }

    let mut flags = DirtyFlags::STYLE;

    if let Some(position) = patch.position {
        node.style.position = position;
        node.layout_meta.bump_style();
        node.paint_meta.bump_visual();
        flags |= DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT;
    }
    if let Some(width) = patch.width {
        node.style.width = width;
        node.layout_meta.bump_style();
        node.paint_meta.bump_visual();
        flags |= DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT;
    }
    if let Some(height) = patch.height {
        node.style.height = height;
        node.layout_meta.bump_style();
        node.paint_meta.bump_visual();
        flags |= DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT;
    }
    if let Some(z_index) = patch.z_index {
        node.style.z_index = z_index;
        node.layout_meta.bump_style();
        node.paint_meta.bump_paint_order();
        flags |= DirtyFlags::PAINT_ORDER | DirtyFlags::HIT | DirtyFlags::PAINT;
    }
    if let Some(transform) = patch.transform {
        node.style.transform = transform;
        node.layout_meta.bump_style();
        node.paint_meta.bump_visual();
        flags |= DirtyFlags::COMPOSITE | DirtyFlags::HIT;
    }
    if let Some(decoration) = patch.decoration {
        node.decoration = decoration;
        node.layout_meta.bump_style();
        node.paint_meta.bump_visual();
        flags |= DirtyFlags::PAINT;
    }

    flags
}
