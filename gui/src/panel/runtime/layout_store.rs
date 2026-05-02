use super::model::{PanelRootRuntime, PanelRuntime};
use super::store::{panel_state_mut, PANEL_ROOT_ID};
use crate::panel::PanelLayout;
use crate::tree::Tree;

pub(crate) fn export_layouts(tree: &Tree) -> Vec<PanelLayout> {
    let mut layouts: Vec<PanelLayout> = tree
        .iter()
        .filter_map(|(_, node)| {
            let panel = node.runtime_slots.get::<PanelRuntime>()?;
            Some(panel_layout_from_runtime(node.id.as_ref(), panel))
        })
        .chain(tree.retained_runtime_iter().filter_map(|(id, slots)| {
            let panel = slots.get::<PanelRuntime>()?;
            Some(panel_layout_from_runtime(id, panel))
        }))
        .collect();

    layouts.sort_by(|a, b| a.id.cmp(&b.id));
    layouts
}

pub(crate) fn import_layouts(tree: &mut Tree, layouts: &[PanelLayout]) {
    let mut max_imported_z: Option<i32> = None;
    for layout in layouts {
        let Some(panel) = panel_state_mut(tree, &layout.id) else {
            continue;
        };
        panel.rect = layout.rect;
        panel.rect.w = panel.rect.w.max(panel.min_size[0]);
        panel.rect.h = panel.rect.h.max(panel.min_size[1]);
        panel.visible = layout.visible;
        panel.z_index = layout.z_index;
        panel.collapsed = layout.collapsed;
        max_imported_z = Some(max_imported_z.map_or(layout.z_index, |z| z.max(layout.z_index)));
    }

    if let Some(max_imported_z) = max_imported_z {
        let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.next_z = root.next_z.max(max_imported_z + 1);
    }
}

fn panel_layout_from_runtime(id: &str, panel: &PanelRuntime) -> PanelLayout {
    PanelLayout {
        id: id.to_string(),
        rect: panel.rect,
        visible: panel.visible,
        z_index: panel.z_index,
        collapsed: panel.collapsed,
    }
}
