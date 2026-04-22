use super::PanelBuildContext;
use gui::panel::{PanelConfig, PanelDeclaration, PanelId};
use gui::renderer::Rect;
use gui::tree::Desc;
use gui::ui;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::frameworks::group::GroupProps;
use std::borrow::Cow;

pub(crate) fn panel(ctx: &PanelBuildContext<'_>) -> PanelDeclaration {
    let engine = ctx.engine;
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

    PanelDeclaration {
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
        content: vec![ui::widget(
            Cow::Borrowed("engine_status_group"),
            GroupProps {
                title: Cow::Borrowed("Runtime"),
                content: vec![
                    status_label("engine_status", status, false),
                    status_label("engine_catalog", catalog, true),
                    status_label("engine_last_action", last_action, true),
                ],
            },
        )
        .build()],
    }
}

fn status_label(id: &'static str, text: String, muted: bool) -> Desc {
    ui::widget(
        Cow::Borrowed(id),
        LabelProps {
            text: Cow::Owned(text),
            variant: LabelVariant::Caption,
            muted,
        },
    )
    .build()
}
