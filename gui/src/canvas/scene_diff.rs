use crate::canvas::node_template::CanvasNodeRenderView;
use crate::canvas::retained_node_card::CanvasNodeCardTemplateData;
use crate::renderer::{Point, Rect};
use crate::template::{
    InstanceId, SlotValue, SlotValues, TemplateId, TemplatePayload, CANVAS_CONNECTION_TEMPLATE,
    CANVAS_NODE_CARD_TEMPLATE, CANVAS_PENDING_CONNECTION_TEMPLATE,
};
use crate::theme::Theme;
use crate::tree::{NodeId, StylePatch, TreeMutation};

#[derive(Clone, Debug)]
pub enum CanvasSceneChange {
    AddNode {
        parent: NodeId,
        owner_id: String,
    },
    AddNodeWithState {
        parent: NodeId,
        owner_id: String,
        rect: Rect,
        label: String,
        z_index: i32,
    },
    AddNodeCard {
        parent: NodeId,
        view: CanvasNodeRenderView,
        theme: Theme,
    },
    RemoveNode {
        root: NodeId,
    },
    AddConnection {
        parent: NodeId,
        id: String,
        from_port: String,
        to_port: String,
    },
    RemoveConnection {
        root: NodeId,
    },
    AddPendingConnection {
        parent: NodeId,
        from_port: String,
        cursor_canvas: Point,
    },
    UpdatePendingConnection {
        node: NodeId,
        from_port: String,
        cursor_canvas: Point,
    },
    RemovePendingConnection {
        root: NodeId,
    },
    DragNode {
        root: NodeId,
        rect: Rect,
    },
    ResizeNode {
        root: NodeId,
        rect: Rect,
    },
    SelectNode {
        root: NodeId,
        selected: bool,
    },
    SetText {
        node: NodeId,
        value: String,
    },
}

pub fn scene_change_to_mutation(change: CanvasSceneChange) -> TreeMutation {
    match change {
        CanvasSceneChange::AddNode { parent, owner_id } => TreeMutation::MountTemplate {
            parent,
            template: TemplateId::from(CANVAS_NODE_CARD_TEMPLATE),
            instance: InstanceId::from(format!("canvas_node::{owner_id}")),
            payload: TemplatePayload::default(),
        },
        CanvasSceneChange::AddNodeWithState {
            parent,
            owner_id,
            rect,
            label,
            z_index,
        } => TreeMutation::MountTemplate {
            parent,
            template: TemplateId::from(CANVAS_NODE_CARD_TEMPLATE),
            instance: InstanceId::from(format!("canvas_node::{owner_id}")),
            payload: TemplatePayload::from(
                SlotValues::new()
                    .with("rect", SlotValue::Rect(rect))
                    .with("label", SlotValue::Text(label))
                    .with("z_index", SlotValue::ZIndex(z_index)),
            ),
        },
        CanvasSceneChange::AddNodeCard {
            parent,
            view,
            theme,
        } => {
            let instance = InstanceId::from(super::canvas_node_stable_id(&view.state.owner_id));
            TreeMutation::MountTemplate {
                parent,
                template: TemplateId::from(CANVAS_NODE_CARD_TEMPLATE),
                instance,
                payload: TemplatePayload::CanvasNodeCard(CanvasNodeCardTemplateData::new(
                    view, &theme,
                )),
            }
        }
        CanvasSceneChange::RemoveNode { root } => TreeMutation::Unmount { node: root },
        CanvasSceneChange::AddConnection {
            parent,
            id,
            from_port,
            to_port,
        } => TreeMutation::MountTemplate {
            parent,
            template: TemplateId::from(CANVAS_CONNECTION_TEMPLATE),
            instance: InstanceId::from(id),
            payload: TemplatePayload::from(
                SlotValues::new().with("endpoints", SlotValue::Connection { from_port, to_port }),
            ),
        },
        CanvasSceneChange::RemoveConnection { root } => TreeMutation::Unmount { node: root },
        CanvasSceneChange::AddPendingConnection {
            parent,
            from_port,
            cursor_canvas,
        } => TreeMutation::MountTemplate {
            parent,
            template: TemplateId::from(CANVAS_PENDING_CONNECTION_TEMPLATE),
            instance: InstanceId::from("canvas_connection::pending"),
            payload: TemplatePayload::from(SlotValues::new().with(
                "pending",
                SlotValue::PendingConnection {
                    from_port,
                    cursor_canvas,
                },
            )),
        },
        CanvasSceneChange::UpdatePendingConnection {
            node,
            from_port,
            cursor_canvas,
        } => TreeMutation::SetPendingConnection {
            node,
            from_port,
            cursor_canvas,
        },
        CanvasSceneChange::RemovePendingConnection { root } => TreeMutation::Unmount { node: root },
        CanvasSceneChange::DragNode { root, rect } => TreeMutation::SetRect { node: root, rect },
        CanvasSceneChange::ResizeNode { root, rect } => TreeMutation::SetRect { node: root, rect },
        CanvasSceneChange::SelectNode { root, selected } => {
            let z_index = selected.then_some(1).unwrap_or(0);
            TreeMutation::SetStyle {
                node: root,
                patch: StylePatch {
                    z_index: Some(z_index),
                    ..StylePatch::default()
                },
            }
        }
        CanvasSceneChange::SetText { node, value } => TreeMutation::SetText { node, value },
    }
}

pub fn scene_changes_to_mutations(
    changes: impl IntoIterator<Item = CanvasSceneChange>,
) -> Vec<TreeMutation> {
    changes.into_iter().map(scene_change_to_mutation).collect()
}
