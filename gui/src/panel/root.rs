use crate::renderer::Rect;
use crate::tree::{Desc, Tree};
use crate::ui::{self, StyleBuilder};
use crate::widget::frameworks::panel::PanelProps;

use super::PanelConfig;

pub struct PanelDeclaration {
    pub config: PanelConfig,
    pub content: Vec<Desc>,
}

/// Legacy `Desc` panel root composer.
///
/// Owner: panel retained migration. Delete after panel frames and panel content
/// mount through `PanelFrameTemplate` plus retained content templates.
pub fn panel_root(tree: &mut Tree, viewport: Rect, panels: Vec<PanelDeclaration>) -> Desc {
    for panel in &panels {
        tree.ensure_panel(&panel.config);
    }

    let mut visible = panels
        .into_iter()
        .filter(|panel| {
            tree.panel_state(panel.config.id.as_str())
                .map(|state| state.visible)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    visible.sort_by_key(|panel| {
        tree.panel_state(panel.config.id.as_str())
            .map(|state| state.z_index)
            .unwrap_or_default()
    });

    ui::container("panel_root")
        .absolute_xy(0.0, 0.0)
        .fixed_width(viewport.w)
        .fixed_height(viewport.h)
        .hittable(false)
        .children(
            visible
                .into_iter()
                .filter_map(|panel| {
                    let state = tree.panel_state(panel.config.id.as_str())?;
                    Some(ui::widget(
                        panel.config.id.clone().into_cow(),
                        PanelProps {
                            title: panel.config.title,
                            rect: state.rect,
                            z_index: state.z_index,
                            min_size: panel.config.min_size,
                            titlebar_visible: panel.config.titlebar_visible,
                            draggable: panel.config.draggable,
                            resizable: panel.config.resizable,
                            closable: panel.config.closable,
                            content: panel.content,
                        },
                    ))
                })
                .collect::<Vec<_>>(),
        )
        .build()
}
