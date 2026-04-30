use std::borrow::Cow;

use super::{PanelConfig, PanelRuntime};
use crate::gesture::Gesture;
use crate::renderer::{Border, ImageStyle, Rect};
use crate::template::{
    InstanceId, RetainedTemplate, TemplateError, TemplateId, TemplateInstance, TemplatePayload,
    TemplateRevision, PANEL_FRAME_TEMPLATE,
};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Overflow, Position,
    RelayoutBoundaryReason, Size, TextLayout, TextOverflow, TextureHandle,
};
use crate::tree::{
    NodeId, NodeKind, NodeLayoutMeta, NodeLocalRuntime, NodeMutationMeta, NodePaintMeta, NodeProps,
    RectMoveInvalidation, RepaintBoundaryReason, RuntimeSlots, StableId, Tree, TreeNode,
};

const REVISION: TemplateRevision = TemplateRevision::new(2);

#[derive(Clone, Debug)]
pub struct PanelFrameTemplateData {
    config: PanelConfig,
    runtime: PanelRuntime,
    content: PanelContentTemplate,
    theme: Theme,
}

#[derive(Clone, Debug)]
pub enum PanelContentTemplate {
    Toolbar {
        add_graph_id: String,
        run_graph_id: String,
    },
    Preview {
        image_id: String,
        texture: TextureHandle,
        image_style: ImageStyle,
    },
    Engine {
        group_id: String,
        status: String,
        catalog: String,
        last_action: String,
    },
}

impl PanelFrameTemplateData {
    pub fn new(
        config: PanelConfig,
        runtime: PanelRuntime,
        content: PanelContentTemplate,
        theme: &Theme,
    ) -> Self {
        Self {
            config,
            runtime,
            content,
            theme: theme.clone(),
        }
    }
}

pub struct PanelFrameRetainedTemplate;

impl RetainedTemplate for PanelFrameRetainedTemplate {
    fn id(&self) -> TemplateId {
        TemplateId::from(PANEL_FRAME_TEMPLATE)
    }

    fn revision(&self) -> TemplateRevision {
        REVISION
    }

    fn instantiate(
        &self,
        tree: &mut Tree,
        instance: InstanceId,
        parent: NodeId,
        payload: TemplatePayload,
    ) -> Result<TemplateInstance, TemplateError> {
        let TemplatePayload::PanelFrame(data) = payload else {
            return Err(TemplateError::UnsupportedPayload {
                template: self.id(),
                reason: "panel frame template requires panel frame payload",
            });
        };
        let mut mount = MountCx::new(tree);
        let root = mount_panel(&mut mount, parent, &data)?;
        Ok(TemplateInstance {
            template: self.id(),
            template_revision: self.revision(),
            instance,
            root_node: root,
        })
    }
}

struct MountCx<'a> {
    tree: &'a mut Tree,
    mounted: Vec<NodeId>,
}

impl<'a> MountCx<'a> {
    fn new(tree: &'a mut Tree) -> Self {
        Self {
            tree,
            mounted: Vec::new(),
        }
    }

    fn child(&mut self, parent: NodeId, node: TreeNode) -> Result<NodeId, TemplateError> {
        let id = self.tree.insert_checked(node)?;
        self.mounted.push(id);
        if !self.tree.append_child(parent, id) {
            self.rollback();
            return Err(TemplateError::MissingParent(parent));
        }
        Ok(id)
    }

    fn rollback(&mut self) {
        for id in self.mounted.iter().rev().copied() {
            self.tree.detach_from_parent(id);
            self.tree.remove(id);
        }
        self.mounted.clear();
    }
}

fn mount_panel(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    data: &PanelFrameTemplateData,
) -> Result<NodeId, TemplateError> {
    let id = data.config.id.as_str();
    let theme = &data.theme;
    let panel = theme.components.panel;
    let visual = theme.panel_visual();
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                position: Position::absolute_xy(data.runtime.rect.x, data.runtime.rect.y),
                width: Size::Fixed(data.runtime.rect.w),
                height: Size::Fixed(data.runtime.rect.h),
                min_width: data.runtime.min_size[0],
                min_height: data.runtime.min_size[1],
                z_index: data.runtime.z_index,
                overflow: Overflow::Hidden,
                hittable: Some(true),
                resizable: data.config.resizable,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.frame_background),
                border: Some(Border {
                    width: panel.border_width,
                    color: visual.frame_border,
                }),
                radius: [panel.radius; 4],
                shadow: None,
            }),
        )
        .with_semantic_role("Panel")
        .with_layout_boundary(RelayoutBoundaryReason::Panel)
        .with_paint_boundary(RepaintBoundaryReason::PanelFrame)
        .with_rect_move_invalidation(RectMoveInvalidation::LayoutAndBoundaryPlacement),
    )?;

    if data.config.titlebar_visible {
        mount_titlebar(cx, root, data)?;
    }
    let content = cx.child(
        root,
        container(
            format!("{id}::content"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fill,
                padding: Edges::all(panel.content_padding),
                gap: theme.spacing.sm,
                flex_grow: 1.0,
                overflow: Overflow::Hidden,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    mount_panel_content(cx, content, data)?;
    Ok(root)
}

fn mount_titlebar(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    data: &PanelFrameTemplateData,
) -> Result<(), TemplateError> {
    let id = data.config.id.as_str();
    let panel = data.theme.components.panel;
    let visual = data.theme.panel_visual();
    let titlebar = cx.child(
        parent,
        container(
            format!("{id}::titlebar"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(panel.title_bar_height),
                padding: Edges::symmetric(panel.title_padding_y, panel.title_padding_x),
                direction: Direction::Row,
                align_items: Align::Center,
                hittable: Some(true),
                draggable: data.config.draggable,
                gestures: vec![Gesture::Tap, Gesture::Drag],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.titlebar_background),
                border: None,
                radius: [panel.radius, panel.radius, 0.0, 0.0],
                shadow: None,
            }),
        )
        .with_owner(id.to_string()),
    )?;
    cx.child(
        titlebar,
        leaf(
            format!("{id}::title"),
            LeafKind::Text {
                content: data.config.title.to_string(),
                style: crate::renderer::TextStyle::new(visual.title_text, panel.title_font_size)
                    .with_family(data.theme.text.body_family)
                    .with_line_height(data.theme.text.default_line_height),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn mount_panel_content(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    data: &PanelFrameTemplateData,
) -> Result<(), TemplateError> {
    match &data.content {
        PanelContentTemplate::Toolbar {
            add_graph_id,
            run_graph_id,
        } => {
            mount_button(cx, parent, add_graph_id, "Add Image Demo", &data.theme)?;
            mount_button(cx, parent, run_graph_id, "Run Image Demo", &data.theme)?;
        }
        PanelContentTemplate::Preview {
            image_id,
            texture,
            image_style,
        } => {
            cx.child(
                parent,
                leaf(
                    image_id.clone(),
                    LeafKind::Image {
                        texture: *texture,
                        style: *image_style,
                    },
                    BoxStyle {
                        width: Size::Fill,
                        height: Size::Fill,
                        min_height: 128.0,
                        ..BoxStyle::default()
                    },
                ),
            )?;
        }
        PanelContentTemplate::Engine {
            group_id,
            status,
            catalog,
            last_action,
        } => mount_engine_group(
            cx,
            parent,
            group_id,
            status,
            catalog,
            last_action,
            &data.theme,
        )?,
    }
    Ok(())
}

fn mount_button(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    label: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let visual = theme.button_visual(crate::interaction::WidgetVisualState::Normal);
    let button = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(
                    theme.components.button.font_size + theme.components.button.padding_y * 2.0,
                ),
                padding: Edges::symmetric(
                    theme.components.button.padding_y,
                    theme.components.button.padding_x,
                ),
                direction: Direction::Row,
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Center,
                hittable: Some(true),
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.background),
                border: visual.border.map(|color| Border {
                    width: theme.components.button.border_width,
                    color,
                }),
                radius: [theme.components.button.radius; 4],
                shadow: None,
            }),
        )
        .with_semantic_role("Button"),
    )?;
    cx.child(
        button,
        leaf(
            format!("{id}::label"),
            LeafKind::Text {
                content: label.to_string(),
                style: crate::renderer::TextStyle::new(
                    visual.text,
                    theme.components.button.font_size,
                )
                .with_family(theme.text.body_family)
                .with_line_height(theme.text.default_line_height),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Auto,
                height: Size::Auto,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn mount_engine_group(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    status: &str,
    catalog: &str,
    last_action: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let group = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                padding: Edges::all(theme.components.group.padding),
                gap: theme.components.group.gap,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: theme.components.group.border_width,
                    color: theme.colors.border,
                }),
                radius: [theme.components.group.radius; 4],
                shadow: None,
            }),
        ),
    )?;
    cx.child(
        group,
        leaf(
            format!("{id}::title"),
            LeafKind::Text {
                content: "Runtime".to_string(),
                style: theme.text_style_title_sm(),
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    mount_label(cx, group, "engine_status", status, theme, false)?;
    mount_label(cx, group, "engine_catalog", catalog, theme, true)?;
    mount_label(cx, group, "engine_last_action", last_action, theme, true)?;
    Ok(())
}

fn mount_label(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    text: &str,
    theme: &Theme,
    muted: bool,
) -> Result<(), TemplateError> {
    let mut style = theme.text_style_label_sm();
    style.color = if muted {
        theme.colors.text_muted
    } else {
        theme.colors.text
    };
    cx.child(
        parent,
        leaf(
            id.to_string(),
            LeafKind::Text {
                content: text.to_string(),
                style,
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    Ok(())
}

fn label_style() -> BoxStyle {
    BoxStyle {
        width: Size::Fill,
        height: Size::Auto,
        flex_shrink: 1.0,
        ..BoxStyle::default()
    }
}

fn ellipsis_text_layout() -> TextLayout {
    TextLayout {
        overflow: TextOverflow::Ellipsis,
        ..TextLayout::default()
    }
}

fn container(id: String, style: BoxStyle, decoration: Option<Decoration>) -> TreeNode {
    TreeNode {
        id: StableId::from(id),
        props: NodeProps::default(),
        style,
        decoration,
        kind: NodeKind::Container,
        rect: zero_rect(),
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: NodeLayoutMeta::default(),
        paint_meta: NodePaintMeta::default(),
        mutation_meta: NodeMutationMeta::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

fn leaf(id: String, kind: LeafKind, style: BoxStyle) -> TreeNode {
    TreeNode {
        id: StableId::from(id),
        props: NodeProps::default(),
        style,
        decoration: None,
        kind: NodeKind::Leaf(kind),
        rect: zero_rect(),
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: NodeLayoutMeta::default(),
        paint_meta: NodePaintMeta::default(),
        mutation_meta: NodeMutationMeta::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

trait TreeNodeExt {
    fn with_semantic_role(self, role: &'static str) -> Self;
    fn with_owner(self, owner: String) -> Self;
    fn with_layout_boundary(self, reason: RelayoutBoundaryReason) -> Self;
    fn with_paint_boundary(self, reason: RepaintBoundaryReason) -> Self;
    fn with_rect_move_invalidation(self, invalidation: RectMoveInvalidation) -> Self;
}

impl TreeNodeExt for TreeNode {
    fn with_semantic_role(mut self, role: &'static str) -> Self {
        self.props.semantic_role = Some(Cow::Borrowed(role));
        self
    }

    fn with_owner(mut self, owner: String) -> Self {
        self.props.owner_id = Some(Cow::Owned(owner));
        self
    }

    fn with_layout_boundary(mut self, reason: RelayoutBoundaryReason) -> Self {
        self.layout_meta.set_boundary(reason);
        self
    }

    fn with_paint_boundary(mut self, reason: RepaintBoundaryReason) -> Self {
        self.paint_meta.set_boundary(reason);
        self
    }

    fn with_rect_move_invalidation(mut self, invalidation: RectMoveInvalidation) -> Self {
        self.mutation_meta.rect_move = invalidation;
        self
    }
}

fn zero_rect() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    }
}
