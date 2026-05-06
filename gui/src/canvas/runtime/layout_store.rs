use super::model::{CanvasInteractionRuntime, CanvasNodeRuntime, CANVAS_INTERACTION_ID};
use crate::canvas::{canvas_node_stable_id, CanvasNodeIdentity, CanvasNodeLayout};
use crate::tree::Tree;
use std::collections::HashSet;

pub(crate) fn sync_node_layouts(
    tree: &mut Tree,
    identities: &[CanvasNodeIdentity],
) -> Vec<CanvasNodeLayout> {
    let owner_ids: HashSet<&str> = identities
        .iter()
        .map(|identity| identity.owner_id.as_str())
        .collect();
    tree.retain_retained_runtime_slots(|stable_id, slots| {
        if slots.get::<CanvasNodeRuntime>().is_none() {
            return true;
        }
        let Some(owner_id) = stable_id.strip_prefix("canvas_node::") else {
            return true;
        };
        owner_ids.contains(owner_id)
    });
    if let Some(interaction) =
        tree.runtime_slot_by_stable_id_mut::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
    {
        interaction.retain_owner_ids(&owner_ids);
    }

    let mut layouts = Vec::with_capacity(identities.len());
    for (index, identity) in identities.iter().enumerate() {
        let stable_id = canvas_node_stable_id(&identity.owner_id);
        let runtime = tree.ensure_runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id);
        if runtime.owner_id.is_empty() {
            *runtime = CanvasNodeRuntime::from_identity(identity, index as i32);
        }
        layouts.push(runtime.to_layout());
    }

    layouts.sort_by(|a, b| {
        a.z_index
            .cmp(&b.z_index)
            .then_with(|| a.owner_id.cmp(&b.owner_id))
    });
    layouts
}

pub(crate) fn export_node_layouts(tree: &Tree) -> Vec<CanvasNodeLayout> {
    let mut layouts: Vec<CanvasNodeLayout> = tree
        .iter()
        .filter_map(|(_, node)| {
            node.runtime_slots
                .get::<CanvasNodeRuntime>()
                .map(CanvasNodeRuntime::to_layout)
        })
        .chain(tree.retained_runtime_values().filter_map(|slots| {
            slots
                .get::<CanvasNodeRuntime>()
                .map(CanvasNodeRuntime::to_layout)
        }))
        .collect();

    layouts.sort_by(|a, b| a.owner_id.cmp(&b.owner_id));
    layouts
}

pub(crate) fn node_layout(tree: &Tree, owner_id: &str) -> Option<CanvasNodeLayout> {
    let stable_id = canvas_node_stable_id(owner_id);
    tree.runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id)
        .map(CanvasNodeRuntime::to_layout)
}

pub(crate) fn import_node_layouts(tree: &mut Tree, layouts: &[CanvasNodeLayout]) {
    for layout in layouts {
        let stable_id = canvas_node_stable_id(&layout.owner_id);
        let Some(runtime) = tree.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            continue;
        };
        runtime.rect = layout.rect;
        runtime.z_index = layout.z_index;
        runtime.collapsed = layout.collapsed;
        runtime.user_min_height = layout.user_min_height;
    }
}

pub(crate) fn bring_node_to_front(tree: &mut Tree, owner_id: &str) -> bool {
    let stable_id = canvas_node_stable_id(owner_id);
    let Some(current) = tree
        .runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id)
        .map(|runtime| runtime.z_index)
    else {
        return false;
    };

    let max_other = export_node_layouts(tree)
        .into_iter()
        .filter(|layout| layout.owner_id != owner_id)
        .map(|layout| layout.z_index)
        .max();
    let Some(max_other) = max_other else {
        return false;
    };
    if current > max_other {
        return false;
    }

    let Some(runtime) = tree.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id) else {
        return false;
    };
    runtime.z_index = max_other + 1;
    true
}
