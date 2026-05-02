use super::model::{PanelRootRuntime, PanelRuntime};
use crate::panel::PanelConfig;
use crate::tree::Tree;

pub(crate) const PANEL_ROOT_ID: &str = "panel_root";

pub(crate) fn ensure_panel(tree: &mut Tree, config: &PanelConfig) {
    let id = config.id.as_str();
    if panel_state(tree, id).is_some() {
        let panel = panel_state_mut(tree, id).expect("panel state checked above");
        panel.min_size = config.min_size;
        panel.rect.w = panel.rect.w.max(config.min_size[0]);
        panel.rect.h = panel.rect.h.max(config.min_size[1]);
        return;
    }

    let z_index = {
        let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        let z_index = root.next_z;
        root.next_z += 1;
        z_index
    };
    *tree.ensure_runtime_slot_by_stable_id::<PanelRuntime>(id) =
        PanelRuntime::from_config(config, z_index);
}

pub(crate) fn panel_state<'a>(tree: &'a Tree, id: &str) -> Option<&'a PanelRuntime> {
    tree.runtime_slot_by_stable_id::<PanelRuntime>(id)
}

pub(crate) fn panel_state_mut<'a>(tree: &'a mut Tree, id: &str) -> Option<&'a mut PanelRuntime> {
    tree.runtime_slot_by_stable_id_mut::<PanelRuntime>(id)
}

pub(crate) fn bring_to_front(tree: &mut Tree, id: &str) {
    if panel_state(tree, id).is_none() {
        return;
    }
    let next_z = {
        let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        let z = root.next_z;
        root.next_z += 1;
        root.focused = Some(id.to_string());
        z
    };
    if let Some(panel) = panel_state_mut(tree, id) {
        panel.z_index = next_z;
    }
}
