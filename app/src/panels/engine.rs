use std::borrow::Cow;

use gui::control::ControlNode;
use gui::panel::{PanelConfig, PanelContentTemplate, PanelId};
use gui::renderer::Rect;

use super::{PanelAppliedSnapshot, PanelModeSet, PanelRenderInput, PanelSpec};

pub(super) fn spec(input: PanelRenderInput<'_>) -> PanelSpec {
    let engine = input.engine_panel;
    let status = format!(
        "Status: {} | Nodes: {} | Connections: {}",
        engine.execution_status, engine.node_count, engine.connection_count
    );
    let catalog = format!(
        "Catalog: {} definitions | Graph v{}{}",
        engine.node_def_count,
        engine.graph_version,
        if engine.dirty { " *" } else { "" }
    );
    let last_action = format!("Last: {}", engine.last_action);

    PanelSpec {
        modes: PanelModeSet::Full,
        config: PanelConfig {
            id: PanelId::new("engine"),
            title: Cow::Borrowed("Engine"),
            default_rect: Rect {
                x: 28.0,
                y: 96.0,
                w: 300.0,
                h: 150.0,
            },
            min_size: [260.0, 132.0],
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: true,
        },
        content: PanelContentTemplate::new(vec![ControlNode::group(
            "engine_status_group",
            "Runtime",
            vec![
                ControlNode::label("engine_status", status.clone()),
                ControlNode::muted_label("engine_catalog", catalog.clone()),
                ControlNode::muted_label("engine_last_action", last_action.clone()),
            ],
        )]),
        applied: PanelAppliedSnapshot::new()
            .with_text("engine_status", status)
            .with_text("engine_catalog", catalog)
            .with_text("engine_last_action", last_action),
    }
}
