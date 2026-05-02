use super::{
    BoundaryDeclarations, CompiledNode, CompiledTemplate, SlotBinding, SlotTarget, TemplateError,
    TemplateRegistry, TemplateRevision, TemplateSlots,
};
use crate::gesture::Gesture;
use crate::renderer::{Color, Point, TextStyle};
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Inset, Justify, LeafKind, Overflow, Position,
    RelayoutBoundaryReason, Size, TextLayout,
};
use crate::tree::{
    NodeLayoutMeta, NodeMutationMeta, NodePaintMeta, RectMoveInvalidation, RepaintBoundaryReason,
};

pub const TEXT_BOX_TEMPLATE: &str = "builtin::text_box";
pub const CANVAS_ROOT_TEMPLATE: &str = "builtin::canvas_root";
pub const CANVAS_GRID_TEMPLATE: &str = "builtin::canvas_grid";
pub const CANVAS_NODE_CARD_TEMPLATE: &str = "builtin::canvas_node_card";
pub const WORKSPACE_ROOT_TEMPLATE: &str = "builtin::workspace_root";
pub const CANVAS_CONNECTION_TEMPLATE: &str = "builtin::canvas_connection";
pub const CANVAS_PENDING_CONNECTION_TEMPLATE: &str = "builtin::canvas_pending_connection";
pub const PARAM_CONTROL_TEMPLATE: &str = "builtin::param_control";
pub const PANEL_FRAME_TEMPLATE: &str = "builtin::panel_frame";
pub const DROPDOWN_OVERLAY_TEMPLATE: &str = "builtin::dropdown_overlay";
pub const NODE_PALETTE_TEMPLATE: &str = "builtin::node_palette";
pub const NODE_PALETTE_CATEGORY_TEMPLATE: &str = "builtin::node_palette_category";
pub const NODE_PALETTE_ITEM_TEMPLATE: &str = "builtin::node_palette_item";
pub const NODE_PALETTE_EMPTY_TEMPLATE: &str = "builtin::node_palette_empty";

const BUILTIN_TEMPLATE_REVISION: TemplateRevision = TemplateRevision::new(1);

pub(crate) fn register_builtin_templates(
    registry: &mut TemplateRegistry,
) -> Result<(), TemplateError> {
    registry.register(text_box_template())?;
    registry.register(canvas_root_template())?;
    registry.register(canvas_grid_template())?;
    registry.register(canvas_node_card_template())?;
    registry.register(workspace_root_template())?;
    registry.register(canvas_connection_template())?;
    registry.register(canvas_pending_connection_template())?;
    registry.register(param_control_template())?;
    registry.register(panel_frame_template())?;
    registry.register(node_palette_template())?;
    registry.register(node_palette_category_template())?;
    registry.register(node_palette_item_template())?;
    registry.register(node_palette_empty_template())?;
    registry
        .register_retained(crate::canvas::retained_node_card::CanvasNodeCardRetainedTemplate)?;
    registry.register_retained(crate::panel::retained::PanelFrameRetainedTemplate)?;
    registry.register_retained(crate::overlay::retained::DropdownOverlayRetainedTemplate)?;
    Ok(())
}

pub(crate) fn node_palette_template() -> CompiledTemplate {
    CompiledTemplate::new(
        NODE_PALETTE_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container(
            "",
            node_palette_root_style(),
            Some(node_palette_surface_decoration()),
        )
        .with_children(vec![
            text_leaf("::title", "Node Library", node_palette_title_text_style()),
            container("::items", node_palette_items_style(), None),
        ]),
    )
    .with_slots(TemplateSlots::new(vec![SlotBinding::new(
        "rect",
        "",
        SlotTarget::Rect,
    )]))
    .with_boundaries(BoundaryDeclarations {
        relayout: vec![""],
        repaint: vec![""],
    })
}

pub(crate) fn node_palette_category_template() -> CompiledTemplate {
    CompiledTemplate::new(
        NODE_PALETTE_CATEGORY_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container("", node_palette_category_style(), None).with_children(vec![text_leaf(
            "::label",
            "",
            node_palette_category_text_style(),
        )]),
    )
    .with_slots(TemplateSlots::new(vec![SlotBinding::new(
        "label",
        "::label",
        SlotTarget::TextContent,
    )]))
}

pub(crate) fn node_palette_item_template() -> CompiledTemplate {
    CompiledTemplate::new(
        NODE_PALETTE_ITEM_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container(
            "",
            node_palette_item_style(),
            Some(node_palette_item_decoration()),
        )
        .with_children(vec![text_leaf(
            "::label",
            "",
            node_palette_item_text_style(),
        )]),
    )
    .with_slots(TemplateSlots::new(vec![SlotBinding::new(
        "label",
        "::label",
        SlotTarget::TextContent,
    )]))
}

pub(crate) fn node_palette_empty_template() -> CompiledTemplate {
    CompiledTemplate::new(
        NODE_PALETTE_EMPTY_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container("", node_palette_empty_style(), None).with_children(vec![text_leaf(
            "::label",
            "No nodes available",
            node_palette_category_text_style(),
        )]),
    )
    .with_slots(TemplateSlots::new(vec![SlotBinding::new(
        "label",
        "::label",
        SlotTarget::TextContent,
    )]))
}

pub(crate) fn canvas_connection_template() -> CompiledTemplate {
    CompiledTemplate::new(
        CANVAS_CONNECTION_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        connection_leaf(
            "",
            LeafKind::Connection {
                from_port: std::borrow::Cow::Borrowed(""),
                to_port: std::borrow::Cow::Borrowed(""),
            },
        ),
    )
    .with_slots(TemplateSlots::new(vec![SlotBinding::new(
        "endpoints",
        "",
        SlotTarget::ConnectionEndpoints,
    )]))
}

pub(crate) fn canvas_pending_connection_template() -> CompiledTemplate {
    CompiledTemplate::new(
        CANVAS_PENDING_CONNECTION_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        connection_leaf(
            "",
            LeafKind::PendingConnection {
                from_port: std::borrow::Cow::Borrowed(""),
                cursor_canvas: Point { x: 0.0, y: 0.0 },
            },
        ),
    )
    .with_slots(TemplateSlots::new(vec![SlotBinding::new(
        "pending",
        "",
        SlotTarget::PendingConnection,
    )]))
}

pub(crate) fn workspace_root_template() -> CompiledTemplate {
    CompiledTemplate::new(
        WORKSPACE_ROOT_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container_with_boundaries(
            "",
            workspace_root_style(),
            None,
            RelayoutBoundaryReason::Root,
            RepaintBoundaryReason::Root,
        )
        .with_absolute_id("root")
        .with_children(vec![
            container_with_boundaries(
                "::canvas",
                workspace_canvas_root_style(),
                None,
                RelayoutBoundaryReason::CanvasRoot,
                RepaintBoundaryReason::CanvasRoot,
            )
            .with_absolute_id("canvas_root")
            .with_children(vec![
                workspace_grid_leaf("::grid").with_absolute_id("canvas_grid"),
                container_with_boundaries(
                    "::connections",
                    canvas_connections_style(),
                    None,
                    RelayoutBoundaryReason::Explicit,
                    RepaintBoundaryReason::CanvasConnectionLayer,
                )
                .with_absolute_id("canvas_connections"),
            ]),
            container("::panel_root", panel_root_style(), None).with_absolute_id("panel_root"),
            container("::overlay_root", overlay_root_style(), None)
                .with_absolute_id("__overlay_root"),
        ]),
    )
    .with_slots(TemplateSlots::new(vec![
        SlotBinding::new("root_rect", "", SlotTarget::Rect),
        SlotBinding::new("canvas_rect", "::canvas", SlotTarget::Rect),
        SlotBinding::new("canvas_style", "::canvas", SlotTarget::Style),
        SlotBinding::new("grid_rect", "::grid", SlotTarget::Rect),
        SlotBinding::new("panel_rect", "::panel_root", SlotTarget::Rect),
        SlotBinding::new("overlay_rect", "::overlay_root", SlotTarget::Rect),
    ]))
    .with_boundaries(BoundaryDeclarations {
        relayout: vec!["", "::canvas"],
        repaint: vec!["", "::canvas", "::connections"],
    })
}

pub(crate) fn text_box_template() -> CompiledTemplate {
    CompiledTemplate::new(
        TEXT_BOX_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container("", text_box_root_style(), None).with_children(vec![container_with_boundaries(
            "::field",
            text_box_field_style(),
            None,
            RelayoutBoundaryReason::FixedConstraintTextField,
            RepaintBoundaryReason::ActiveTextEditor,
        )
        .with_children(vec![
            container("::selection", absolute_fill_style(), None),
            text_leaf("::value", "", default_text_style()),
            container("::caret", caret_style(), Some(caret_decoration())),
        ])]),
    )
    .with_slots(TemplateSlots::new(vec![
        SlotBinding::new("value", "::value", SlotTarget::TextContent),
        SlotBinding::new("rect", "", SlotTarget::Rect),
        SlotBinding::new("visible", "", SlotTarget::Visible),
    ]))
    .with_boundaries(BoundaryDeclarations {
        relayout: vec!["::field"],
        repaint: vec!["::field"],
    })
}

pub(crate) fn canvas_root_template() -> CompiledTemplate {
    CompiledTemplate::new(
        CANVAS_ROOT_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container_with_boundaries(
            "",
            canvas_root_style(),
            None,
            RelayoutBoundaryReason::CanvasRoot,
            RepaintBoundaryReason::CanvasRoot,
        )
        .with_children(vec![grid_leaf_with_boundary(
            "::grid",
            RepaintBoundaryReason::CanvasGridLayer,
        )]),
    )
    .with_slots(TemplateSlots::new(vec![
        SlotBinding::new("rect", "", SlotTarget::Rect),
        SlotBinding::new("visible", "", SlotTarget::Visible),
    ]))
    .with_boundaries(BoundaryDeclarations {
        relayout: vec![""],
        repaint: vec!["", "::grid"],
    })
}

pub(crate) fn canvas_grid_template() -> CompiledTemplate {
    CompiledTemplate::new(
        CANVAS_GRID_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        grid_leaf_with_boundary("", RepaintBoundaryReason::CanvasGridLayer),
    )
    .with_slots(TemplateSlots::new(vec![
        SlotBinding::new("rect", "", SlotTarget::Rect),
        SlotBinding::new("visible", "", SlotTarget::Visible),
    ]))
    .with_boundaries(BoundaryDeclarations {
        relayout: Vec::new(),
        repaint: vec![""],
    })
}

pub(crate) fn canvas_node_card_template() -> CompiledTemplate {
    CompiledTemplate::new(
        CANVAS_NODE_CARD_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container_with_boundaries(
            "",
            canvas_node_root_style(),
            None,
            RelayoutBoundaryReason::CanvasNodeCard,
            RepaintBoundaryReason::CanvasNodeCard,
        )
        .with_children(vec![container("::card", canvas_node_card_style(), None)
            .with_children(vec![
                container("::body", canvas_node_body_style(), None),
                container("::label", canvas_node_label_style(), None)
                    .with_children(vec![text_leaf("::label_text", "", default_text_style())]),
            ])]),
    )
    .with_slots(TemplateSlots::new(vec![
        SlotBinding::new("label", "::label_text", SlotTarget::TextContent),
        SlotBinding::new("rect", "", SlotTarget::Rect),
        SlotBinding::new("style", "", SlotTarget::Style),
        SlotBinding::new("visible", "", SlotTarget::Visible),
        SlotBinding::new("z_index", "", SlotTarget::ZIndex),
    ]))
    .with_boundaries(BoundaryDeclarations {
        relayout: vec![""],
        repaint: vec![""],
    })
}

pub(crate) fn param_control_template() -> CompiledTemplate {
    CompiledTemplate::new(
        PARAM_CONTROL_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container("", param_control_style(), None).with_children(vec![
            text_leaf("::label", "", default_text_style()),
            text_leaf("::value", "", default_text_style()),
        ]),
    )
    .with_slots(TemplateSlots::new(vec![
        SlotBinding::new("label", "::label", SlotTarget::TextContent),
        SlotBinding::new("value", "::value", SlotTarget::TextContent),
        SlotBinding::new("visible", "", SlotTarget::Visible),
    ]))
}

pub(crate) fn panel_frame_template() -> CompiledTemplate {
    CompiledTemplate::new(
        PANEL_FRAME_TEMPLATE,
        BUILTIN_TEMPLATE_REVISION,
        container_with_boundaries(
            "",
            panel_frame_style(),
            None,
            RelayoutBoundaryReason::Panel,
            RepaintBoundaryReason::PanelFrame,
        )
        .with_children(vec![container("::content", BoxStyle::default(), None)]),
    )
    .with_slots(TemplateSlots::new(vec![
        SlotBinding::new("rect", "", SlotTarget::Rect),
        SlotBinding::new("style", "", SlotTarget::Style),
        SlotBinding::new("visible", "", SlotTarget::Visible),
        SlotBinding::new("z_index", "", SlotTarget::ZIndex),
    ]))
    .with_boundaries(BoundaryDeclarations {
        relayout: vec![""],
        repaint: vec![""],
    })
}

fn container(
    id_suffix: &'static str,
    style: BoxStyle,
    decoration: Option<Decoration>,
) -> CompiledNode {
    let node = CompiledNode::container(id_suffix, style);
    match decoration {
        Some(decoration) => node.with_decoration(decoration),
        None => node,
    }
}

fn container_with_boundaries(
    id_suffix: &'static str,
    style: BoxStyle,
    decoration: Option<Decoration>,
    layout_boundary: RelayoutBoundaryReason,
    paint_boundary: RepaintBoundaryReason,
) -> CompiledNode {
    container(id_suffix, style, decoration)
        .with_layout_meta(NodeLayoutMeta::boundary(layout_boundary))
        .with_paint_meta(NodePaintMeta::boundary(paint_boundary))
}

fn text_leaf(id_suffix: &'static str, content: &str, text_style: TextStyle) -> CompiledNode {
    CompiledNode::leaf(
        id_suffix,
        LeafKind::Text {
            content: content.to_string(),
            style: text_style,
            layout: TextLayout::default(),
        },
        BoxStyle {
            width: Size::Auto,
            height: Size::Auto,
            ..BoxStyle::default()
        },
    )
}

fn grid_leaf_with_boundary(
    id_suffix: &'static str,
    boundary: RepaintBoundaryReason,
) -> CompiledNode {
    CompiledNode::leaf(
        id_suffix,
        LeafKind::Grid {
            spacing: 24.0,
            dot_color: Color {
                r: 0.45,
                g: 0.45,
                b: 0.45,
                a: 0.35,
            },
            dot_size: 1.5,
        },
        BoxStyle {
            width: Size::Fill,
            height: Size::Fill,
            ..BoxStyle::default()
        },
    )
    .with_paint_meta(NodePaintMeta::boundary(boundary))
    .with_mutation_meta(NodeMutationMeta {
        rect_move: RectMoveInvalidation::Repaint,
    })
}

fn connection_leaf(id_suffix: &'static str, kind: LeafKind) -> CompiledNode {
    CompiledNode::leaf(
        id_suffix,
        kind,
        BoxStyle {
            position: Position::absolute_xy(0.0, 0.0),
            width: Size::Fixed(0.0),
            height: Size::Fixed(0.0),
            hittable: false,
            ..BoxStyle::default()
        },
    )
}

fn default_text_style() -> TextStyle {
    TextStyle::new(Color::BLACK, 14.0)
}

fn workspace_root_style() -> BoxStyle {
    BoxStyle {
        position: Position::relative(),
        width: Size::Fill,
        height: Size::Fill,
        hittable: true,
        ..BoxStyle::default()
    }
}

fn workspace_canvas_root_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        width: Size::Fill,
        height: Size::Fill,
        overflow: Overflow::Hidden,
        hittable: true,
        ..BoxStyle::default()
    }
}

fn workspace_grid_leaf(id_suffix: &'static str) -> CompiledNode {
    let mut node = grid_leaf_with_boundary(id_suffix, RepaintBoundaryReason::CanvasGridLayer);
    node.style.position = Position::absolute_xy(0.0, 0.0);
    node.style.z_index = -20;
    node.style.hittable = false;
    node.mutation_meta.rect_move = RectMoveInvalidation::Repaint;
    node
}

fn canvas_connections_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        width: Size::Fill,
        height: Size::Fill,
        z_index: -10,
        hittable: false,
        ..BoxStyle::default()
    }
}

fn panel_root_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        width: Size::Fill,
        height: Size::Fill,
        hittable: false,
        ..BoxStyle::default()
    }
}

fn overlay_root_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        width: Size::Fill,
        height: Size::Fill,
        hittable: false,
        z_index: 10_000,
        ..BoxStyle::default()
    }
}

fn text_box_root_style() -> BoxStyle {
    BoxStyle {
        direction: Direction::Column,
        width: Size::Fill,
        height: Size::Auto,
        ..BoxStyle::default()
    }
}

fn text_box_field_style() -> BoxStyle {
    BoxStyle {
        position: Position::relative(),
        width: Size::Fill,
        height: Size::Fixed(32.0),
        overflow: Overflow::Hidden,
        hittable: true,
        gestures: vec![Gesture::Tap, Gesture::Drag],
        ..BoxStyle::default()
    }
}

fn absolute_fill_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_inset(Inset::ZERO),
        width: Size::Fill,
        height: Size::Fill,
        hittable: false,
        ..BoxStyle::default()
    }
}

fn caret_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        width: Size::Fixed(1.0),
        height: Size::Fill,
        hittable: false,
        ..BoxStyle::default()
    }
}

fn caret_decoration() -> Decoration {
    Decoration {
        background: Some(Color::BLACK),
        border: None,
        radius: [0.0; 4],
        shadow: None,
    }
}

fn canvas_root_style() -> BoxStyle {
    BoxStyle {
        position: Position::relative(),
        width: Size::Fill,
        height: Size::Fill,
        overflow: Overflow::Hidden,
        ..BoxStyle::default()
    }
}

fn canvas_node_root_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        direction: Direction::Row,
        width: Size::Auto,
        height: Size::Auto,
        align_items: Align::Center,
        justify_content: Justify::Start,
        hittable: true,
        draggable: true,
        gestures: vec![Gesture::Tap, Gesture::Drag],
        ..BoxStyle::default()
    }
}

fn canvas_node_card_style() -> BoxStyle {
    BoxStyle {
        position: Position::relative(),
        direction: Direction::Column,
        width: Size::Fixed(304.0),
        height: Size::Fixed(132.0),
        hittable: true,
        resizable: true,
        ..BoxStyle::default()
    }
}

fn canvas_node_body_style() -> BoxStyle {
    BoxStyle {
        width: Size::Fill,
        height: Size::Fill,
        flex_grow: 1.0,
        ..BoxStyle::default()
    }
}

fn canvas_node_label_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, -20.0),
        direction: Direction::Row,
        width: Size::Auto,
        height: Size::Auto,
        ..BoxStyle::default()
    }
}

fn param_control_style() -> BoxStyle {
    BoxStyle {
        direction: Direction::Row,
        width: Size::Fill,
        height: Size::Auto,
        align_items: Align::Center,
        ..BoxStyle::default()
    }
}

fn panel_frame_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        direction: Direction::Column,
        width: Size::Fixed(320.0),
        height: Size::Fixed(240.0),
        hittable: true,
        draggable: true,
        resizable: true,
        ..BoxStyle::default()
    }
}

fn node_palette_root_style() -> BoxStyle {
    BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        direction: Direction::Column,
        width: Size::Fixed(320.0),
        height: Size::Fixed(372.0),
        padding: crate::tree::layout::Edges::all(10.0),
        gap: 8.0,
        z_index: 10_100,
        hittable: true,
        ..BoxStyle::default()
    }
}

fn node_palette_items_style() -> BoxStyle {
    BoxStyle {
        direction: Direction::Column,
        width: Size::Fill,
        height: Size::Fixed(320.0),
        gap: 4.0,
        overflow: Overflow::Scroll,
        ..BoxStyle::default()
    }
}

fn node_palette_category_style() -> BoxStyle {
    BoxStyle {
        width: Size::Fill,
        height: Size::Fixed(24.0),
        padding: crate::tree::layout::Edges::symmetric(4.0, 2.0),
        ..BoxStyle::default()
    }
}

fn node_palette_item_style() -> BoxStyle {
    BoxStyle {
        direction: Direction::Row,
        width: Size::Fill,
        height: Size::Fixed(34.0),
        padding: crate::tree::layout::Edges::symmetric(7.0, 10.0),
        align_items: Align::Center,
        hittable: true,
        gestures: vec![Gesture::Tap],
        ..BoxStyle::default()
    }
}

fn node_palette_empty_style() -> BoxStyle {
    BoxStyle {
        width: Size::Fill,
        height: Size::Fixed(34.0),
        padding: crate::tree::layout::Edges::symmetric(7.0, 10.0),
        ..BoxStyle::default()
    }
}

fn node_palette_surface_decoration() -> Decoration {
    Decoration {
        background: Some(Color {
            r: 0.98,
            g: 0.98,
            b: 0.99,
            a: 1.0,
        }),
        border: Some(crate::renderer::Border {
            width: 1.0,
            color: Color {
                r: 0.78,
                g: 0.80,
                b: 0.84,
                a: 1.0,
            },
        }),
        radius: [8.0; 4],
        shadow: None,
    }
}

fn node_palette_item_decoration() -> Decoration {
    Decoration {
        background: Some(Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }),
        border: Some(crate::renderer::Border {
            width: 1.0,
            color: Color {
                r: 0.86,
                g: 0.88,
                b: 0.91,
                a: 1.0,
            },
        }),
        radius: [6.0; 4],
        shadow: None,
    }
}

fn node_palette_title_text_style() -> TextStyle {
    TextStyle::new(
        Color {
            r: 0.07,
            g: 0.09,
            b: 0.13,
            a: 1.0,
        },
        16.0,
    )
}

fn node_palette_category_text_style() -> TextStyle {
    TextStyle::new(
        Color {
            r: 0.39,
            g: 0.43,
            b: 0.50,
            a: 1.0,
        },
        12.0,
    )
}

fn node_palette_item_text_style() -> TextStyle {
    TextStyle::new(
        Color {
            r: 0.12,
            g: 0.14,
            b: 0.18,
            a: 1.0,
        },
        13.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::template::{InstanceId, SlotValue, SlotValues, TemplateId};
    use crate::tree::{
        DirtyFlags, NodeId, NodeKind, NodeLocalRuntime, NodeProps, RuntimeSlots, StableId, Tree,
        TreeMutation, TreeNode,
    };

    fn host_tree() -> (Tree, NodeId) {
        let mut tree = Tree::new();
        let root = tree
            .insert_checked(TreeNode {
                id: StableId::from("root"),
                props: NodeProps::default(),
                style: BoxStyle::default(),
                decoration: None,
                kind: NodeKind::Container,
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                },
                children: Vec::new(),
                local_runtime: NodeLocalRuntime::default(),
                layout_meta: NodeLayoutMeta::boundary(RelayoutBoundaryReason::Root),
                paint_meta: NodePaintMeta::boundary(RepaintBoundaryReason::Root),
                mutation_meta: NodeMutationMeta::default(),
                runtime_slots: RuntimeSlots::default(),
            })
            .expect("root");
        tree.set_root(root);
        (tree, root)
    }

    #[test]
    fn text_box_template_mounts_expected_stable_ids() {
        let (mut tree, root) = host_tree();
        let registry = TemplateRegistry::with_builtin_templates();

        registry
            .instantiate(
                &mut tree,
                TemplateId::from(TEXT_BOX_TEMPLATE),
                InstanceId::from("input"),
                root,
                SlotValues::default(),
            )
            .expect("instantiate");

        assert!(tree.contains_stable_id("input"));
        assert!(tree.contains_stable_id("input::field"));
        assert!(tree.contains_stable_id("input::value"));
        assert!(tree.contains_stable_id("input::caret"));
        assert!(tree.contains_stable_id("input::selection"));
        tree.validate_index().expect("valid index");
    }

    #[test]
    fn multiple_text_box_instances_share_one_template_definition() {
        let (mut tree, root) = host_tree();
        let registry = TemplateRegistry::with_builtin_templates();

        let first = registry
            .instantiate(
                &mut tree,
                TemplateId::from(TEXT_BOX_TEMPLATE),
                InstanceId::from("first"),
                root,
                SlotValues::default(),
            )
            .expect("first");
        let second = registry
            .instantiate(
                &mut tree,
                TemplateId::from(TEXT_BOX_TEMPLATE),
                InstanceId::from("second"),
                root,
                SlotValues::default(),
            )
            .expect("second");

        assert_eq!(first.template_revision, second.template_revision);
        assert!(tree.contains_stable_id("first::value"));
        assert!(tree.contains_stable_id("second::value"));
        assert_ne!(tree.node_by_str("first"), tree.node_by_str("second"));
    }

    #[test]
    fn template_instance_patch_text_does_not_reinstantiate() {
        let (mut tree, root) = host_tree();
        let registry = TemplateRegistry::with_builtin_templates();
        tree.apply_mutation(
            &registry,
            TreeMutation::MountTemplate {
                parent: root,
                template: TemplateId::from(TEXT_BOX_TEMPLATE),
                instance: InstanceId::from("input"),
                payload: crate::template::TemplatePayload::from(SlotValues::default()),
            },
        )
        .expect("mount");
        let value = tree.node_by_str("input::value").expect("value");
        let node_count = tree.frame_stats_snapshot().tree_nodes;

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetText {
                    node: value,
                    value: "typed".to_string(),
                },
            )
            .expect("set text");

        assert!(invalidation.flags.contains(DirtyFlags::TEXT_LAYOUT));
        assert_eq!(tree.node_by_str("input::value"), Some(value));
        assert_eq!(tree.frame_stats_snapshot().tree_nodes, node_count);
    }

    #[test]
    fn compiled_template_slot_values_initialize_live_nodes() {
        let (mut tree, root) = host_tree();
        let registry = TemplateRegistry::with_builtin_templates();

        registry
            .instantiate(
                &mut tree,
                TemplateId::from(TEXT_BOX_TEMPLATE),
                InstanceId::from("input"),
                root,
                SlotValues::new().with("value", SlotValue::Text("hello".to_string())),
            )
            .expect("instantiate");

        let value = tree.node_by_str("input::value").expect("value");
        let Some(NodeKind::Leaf(LeafKind::Text { content, .. })) =
            tree.get(value).map(|node| &node.kind)
        else {
            panic!("value node must be text");
        };
        assert_eq!(content, "hello");
    }

    #[test]
    fn compiled_template_does_not_store_runtime_text_state() {
        let mut registry = TemplateRegistry::new();
        registry.register(text_box_template()).expect("register");
        let template_id = TemplateId::from(TEXT_BOX_TEMPLATE);
        let before = registry.template(&template_id).expect("template").clone();
        let (mut tree, root) = host_tree();

        registry
            .instantiate(
                &mut tree,
                template_id.clone(),
                InstanceId::from("input"),
                root,
                SlotValues::new().with("value", SlotValue::Text("live value".to_string())),
            )
            .expect("instantiate");

        let after = registry.template(&template_id).expect("template");
        assert_eq!(&before, after);
    }

    #[test]
    fn compiled_template_mount_rolls_back_on_invalid_slot() {
        let (mut tree, root) = host_tree();
        let registry = TemplateRegistry::with_builtin_templates();

        let error = registry
            .instantiate(
                &mut tree,
                TemplateId::from(TEXT_BOX_TEMPLATE),
                InstanceId::from("input"),
                root,
                SlotValues::new().with("unknown", SlotValue::Visible(true)),
            )
            .expect_err("invalid slot should fail");

        assert!(matches!(error, TemplateError::UnsupportedSlot { .. }));
        assert!(tree.node_by_str("input").is_none());
        assert!(tree.node_by_str("input::value").is_none());
        tree.validate_index().expect("rollback keeps index valid");
    }

    #[test]
    fn template_revision_detects_duplicate_incompatible_registration() {
        let mut registry = TemplateRegistry::new();
        registry.register(text_box_template()).expect("first");
        let duplicate = text_box_template()
            .with_boundaries(BoundaryDeclarations::default())
            .with_slots(TemplateSlots::default());
        let mut duplicate = duplicate;
        duplicate.revision = TemplateRevision::new(BUILTIN_TEMPLATE_REVISION.0 + 1);

        let error = registry
            .register(duplicate)
            .expect_err("different revision should fail");

        assert!(matches!(error, TemplateError::DuplicateTemplate { .. }));
    }

    #[test]
    fn canvas_grid_template_mounts_static_structure() {
        let (mut tree, root) = host_tree();
        let registry = TemplateRegistry::with_builtin_templates();

        registry
            .instantiate(
                &mut tree,
                TemplateId::from(CANVAS_ROOT_TEMPLATE),
                InstanceId::from("canvas"),
                root,
                SlotValues::default(),
            )
            .expect("canvas root");

        let grid = tree.node_by_str("canvas::grid").expect("grid");
        assert!(matches!(
            tree.get(grid).map(|node| &node.kind),
            Some(NodeKind::Leaf(LeafKind::Grid { .. }))
        ));
    }

    #[test]
    fn canvas_node_card_template_instantiates_directly() {
        let (mut tree, root) = host_tree();
        let registry = TemplateRegistry::with_builtin_templates();
        tree.clear_frame_stats();

        registry
            .instantiate(
                &mut tree,
                TemplateId::from(CANVAS_NODE_CARD_TEMPLATE),
                InstanceId::from("canvas_node::engine_node::7"),
                root,
                SlotValues::default(),
            )
            .expect("node card");

        assert!(tree.contains_stable_id("canvas_node::engine_node::7::card"));
    }
}
