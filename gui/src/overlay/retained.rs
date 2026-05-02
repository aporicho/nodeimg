use super::{DropdownOverlayContent, OverlayContent};
use crate::gesture::Gesture;
use crate::icon::{names, IconSpec};
use crate::interaction::WidgetVisualState;
use crate::renderer::{Border, Rect};
use crate::template::{
    InstanceId, RetainedTemplate, TemplateError, TemplateId, TemplateMountCx, TemplatePayload,
    TemplateRevision, DROPDOWN_OVERLAY_TEMPLATE,
};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Overflow, Position, Size, TextLayout,
    TextOverflow,
};
use crate::tree::{
    NodeId, NodeKind, NodeLayoutMeta, NodeLocalRuntime, NodeMutationMeta, NodePaintMeta, NodeProps,
    RepaintBoundaryReason, RuntimeSlots, StableId, TreeNode,
};
use crate::widget::WidgetRole;

const REVISION: TemplateRevision = TemplateRevision::new(1);
const POPUP_HEIGHT: f32 = 140.0;

#[derive(Clone, Debug)]
pub struct DropdownOverlayTemplateData {
    overlay_id: String,
    x: f32,
    y: f32,
    width: Option<f32>,
    content: OverlayContent,
    theme: Theme,
}

impl DropdownOverlayTemplateData {
    pub fn new(
        overlay_id: String,
        x: f32,
        y: f32,
        width: Option<f32>,
        content: OverlayContent,
        theme: &Theme,
    ) -> Self {
        Self {
            overlay_id,
            x,
            y,
            width,
            content,
            theme: theme.clone(),
        }
    }
}

pub struct DropdownOverlayRetainedTemplate;

impl RetainedTemplate for DropdownOverlayRetainedTemplate {
    fn id(&self) -> TemplateId {
        TemplateId::from(DROPDOWN_OVERLAY_TEMPLATE)
    }

    fn revision(&self) -> TemplateRevision {
        REVISION
    }

    fn instantiate(
        &self,
        cx: &mut TemplateMountCx<'_>,
        _instance: &InstanceId,
        parent: NodeId,
        payload: TemplatePayload,
    ) -> Result<NodeId, TemplateError> {
        let TemplatePayload::DropdownOverlay(data) = payload else {
            return Err(TemplateError::UnsupportedPayload {
                template: self.id(),
                reason: "dropdown overlay template requires dropdown overlay payload",
            });
        };
        mount_dropdown_overlay(cx, parent, &data)
    }
}

fn mount_dropdown_overlay(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    data: &DropdownOverlayTemplateData,
) -> Result<NodeId, TemplateError> {
    let OverlayContent::DropdownOptions(content) = &data.content;
    let root = cx.child(
        parent,
        container(
            format!("__overlay::{}", data.overlay_id),
            BoxStyle {
                position: Position::absolute_xy(data.x, data.y),
                width: data.width.map(Size::Fixed).unwrap_or(Size::Auto),
                height: Size::Auto,
                overflow: Overflow::Visible,
                hittable: Some(true),
                z_index: 20_000,
                ..BoxStyle::default()
            },
            None,
        )
        .with_paint_boundary(RepaintBoundaryReason::Explicit),
    )?;
    mount_dropdown_group(cx, root, content, &data.theme)?;
    Ok(root)
}

fn mount_dropdown_group(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let group_id = format!("{}::popup_group", content.dropdown_id);
    let tokens = theme.components.group;
    let group = cx.child(
        parent,
        container(
            group_id.clone(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                padding: Edges::all(tokens.padding),
                gap: tokens.title_gap,
                direction: Direction::Column,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(tokens.background),
                border: Some(Border {
                    width: tokens.border_width,
                    color: tokens.border,
                }),
                radius: [tokens.radius; 4],
                shadow: None,
            }),
        ),
    )?;
    mount_group_title(cx, group, &group_id, content.title.as_ref(), theme)?;
    let content_root = cx.child(
        group,
        container(
            format!("{group_id}::content"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                gap: tokens.gap,
                direction: Direction::Column,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    mount_hint(cx, content_root, content, theme)?;
    mount_option_list(cx, content_root, content, theme)?;
    Ok(())
}

fn mount_group_title(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    group_id: &str,
    title: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let tokens = theme.components.group;
    let titlebar = cx.child(
        parent,
        container(
            format!("{group_id}::titlebar"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                padding: Edges::symmetric(tokens.title_padding_y, tokens.title_padding_x),
                direction: Direction::Row,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let mut style = theme.text_style_label_sm();
    style.color = tokens.title_text;
    style.size = tokens.title_font_size;
    cx.child(
        titlebar,
        leaf(
            format!("{group_id}::title"),
            LeafKind::Text {
                content: title.to_string(),
                style,
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    Ok(())
}

fn mount_hint(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let mut style = theme.text_style_label_sm();
    style.color = theme.colors.text_muted;
    cx.child(
        parent,
        leaf(
            format!("{}::popup_hint", content.dropdown_id),
            LeafKind::Text {
                content: "Use mouse or Up/Down + Enter".to_string(),
                style,
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    Ok(())
}

fn mount_option_list(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let list_id = format!("{}::popup_list", content.dropdown_id);
    let scroll = cx.child(
        parent,
        container(
            format!("{list_id}::scroll"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(POPUP_HEIGHT),
                overflow: Overflow::Scroll,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let items = cx.child(
        scroll,
        container(
            format!("{list_id}::items"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                gap: theme.components.list_view.item_gap,
                direction: Direction::Column,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    for (index, option) in content.options.iter().enumerate() {
        mount_option(cx, items, content, index, option.as_ref(), theme)?;
    }
    Ok(())
}

fn mount_option(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    index: usize,
    label: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let metrics = theme.control_metrics(content.size, content.density);
    let highlighted = index == content.highlighted;
    let selected = index == content.selected;
    let visual_state = if highlighted {
        WidgetVisualState::Focused
    } else {
        WidgetVisualState::Normal
    };
    let visual = theme.button_visual(visual_state);
    let marker_color = if selected {
        theme.colors.accent
    } else {
        theme.colors.text_muted
    };
    let option_id = format!("__dropdown_option::{}::{}", content.dropdown_id, index);
    let option_root = cx.child(
        parent,
        container(
            option_id.clone(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(metrics.height),
                padding: Edges::symmetric(metrics.padding_y, metrics.padding_x),
                gap: metrics.gap,
                direction: Direction::Row,
                align_items: Align::Center,
                hittable: Some(true),
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.background),
                border: visual.border.map(|color| Border {
                    width: metrics.border_width,
                    color,
                }),
                radius: [metrics.radius; 4],
                shadow: None,
            }),
        )
        .with_semantic_role(WidgetRole::Button),
    )?;
    let marker = cx.child(
        option_root,
        container(
            format!("{option_id}::marker"),
            BoxStyle {
                width: Size::Fixed(metrics.icon_size),
                height: Size::Fixed(metrics.icon_size),
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Center,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let marker_icon = if selected {
        Some(names::CHECK)
    } else if highlighted {
        Some(names::NAV_ARROW_RIGHT)
    } else {
        None
    };
    if let Some(icon) = marker_icon {
        cx.child(
            marker,
            leaf(
                format!("{option_id}::marker_icon"),
                LeafKind::Icon {
                    spec: IconSpec::new(icon, marker_color),
                },
                BoxStyle {
                    width: Size::Fixed(metrics.icon_size),
                    height: Size::Fixed(metrics.icon_size),
                    ..BoxStyle::default()
                },
            ),
        )?;
    }
    let mut text_style = theme.text_style_body_sm();
    text_style.color = visual.text;
    text_style.size = metrics.font_size;
    cx.child(
        option_root,
        leaf(
            format!("{option_id}::label"),
            LeafKind::Text {
                content: label.to_string(),
                style: text_style,
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
    fn with_semantic_role(self, role: WidgetRole) -> Self;
    fn with_paint_boundary(self, reason: RepaintBoundaryReason) -> Self;
}

impl TreeNodeExt for TreeNode {
    fn with_semantic_role(mut self, role: WidgetRole) -> Self {
        self.props.semantic_role = Some(role);
        self
    }

    fn with_paint_boundary(mut self, reason: RepaintBoundaryReason) -> Self {
        self.paint_meta.set_boundary(reason);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::{InstanceId, TemplateRegistry};
    use crate::theme::{light_theme, ControlSize, Density};
    use crate::tree::{NodeKind, Tree};
    use std::borrow::Cow;

    #[test]
    fn dropdown_overlay_template_mounts_options_and_marker_icons() {
        let registry = TemplateRegistry::with_builtin_templates();
        let mut tree = Tree::new();
        let parent = tree.insert(container(
            "__overlay_root".to_string(),
            BoxStyle::default(),
            None,
        ));
        tree.set_root(parent);
        let theme = light_theme();

        registry
            .instantiate_payload(
                &mut tree,
                TemplateId::from(DROPDOWN_OVERLAY_TEMPLATE),
                InstanceId::from("dropdown::blend"),
                parent,
                TemplatePayload::DropdownOverlay(DropdownOverlayTemplateData::new(
                    "dropdown::blend".to_string(),
                    12.0,
                    34.0,
                    Some(180.0),
                    OverlayContent::DropdownOptions(DropdownOverlayContent {
                        dropdown_id: "blend".to_string(),
                        title: Cow::Borrowed("Mode"),
                        options: vec![Cow::Borrowed("Normal"), Cow::Borrowed("Multiply")],
                        selected: 0,
                        highlighted: 1,
                        size: ControlSize::Small,
                        density: Density::Compact,
                    }),
                    &theme,
                )),
            )
            .expect("mount dropdown overlay");

        let root = tree
            .node_by_str("__overlay::dropdown::blend")
            .expect("overlay root");
        let root_node = tree.get(root).expect("root node");
        assert_eq!(root_node.rect, zero_rect());
        assert_eq!(root_node.style.position, Position::absolute_xy(12.0, 34.0));
        assert_eq!(root_node.style.width, Size::Fixed(180.0));

        let first = tree
            .node_by_str("__dropdown_option::blend::0::marker_icon")
            .expect("selected marker icon");
        let second = tree
            .node_by_str("__dropdown_option::blend::1::marker_icon")
            .expect("highlighted marker icon");
        assert!(matches!(
            &tree.get(first).expect("first marker").kind,
            NodeKind::Leaf(LeafKind::Icon { spec }) if spec.id.as_str() == names::CHECK.as_str()
        ));
        assert!(matches!(
            &tree.get(second).expect("second marker").kind,
            NodeKind::Leaf(LeafKind::Icon { spec }) if spec.id.as_str() == names::NAV_ARROW_RIGHT.as_str()
        ));
    }
}
