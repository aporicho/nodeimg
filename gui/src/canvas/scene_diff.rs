use crate::canvas::node_template::CanvasNodeRenderView;
use crate::canvas::retained_node_card::CanvasNodeCardTemplateData;
use crate::renderer::Point;
use crate::template::{
    InstanceId, SlotValue, SlotValues, TemplateId, TemplatePayload, CANVAS_CONNECTION_TEMPLATE,
    CANVAS_NODE_CARD_TEMPLATE, CANVAS_PENDING_CONNECTION_TEMPLATE,
};
use crate::theme::Theme;
use crate::tree::{NodeId, TreeMutation};

#[derive(Clone, Debug)]
pub enum CanvasSceneChange {
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
    SetText {
        node: NodeId,
        value: String,
    },
}

pub fn scene_change_to_mutation(change: CanvasSceneChange) -> TreeMutation {
    match change {
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
        CanvasSceneChange::SetText { node, value } => TreeMutation::SetText { node, value },
    }
}
