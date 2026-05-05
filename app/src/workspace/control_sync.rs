use gui::canvas::canvas_node_stable_id;
use gui::canvas::node_template::CanvasNodeRenderView;
use gui::control::ControlTextBoxSyncItem;

pub(crate) fn canvas_text_box_sync_items(
    nodes: &[CanvasNodeRenderView],
) -> Vec<ControlTextBoxSyncItem<'_>> {
    nodes
        .iter()
        .flat_map(|view| {
            let stable_id = canvas_node_stable_id(&view.state.owner_id);
            view.template.params.iter().map(move |param| {
                ControlTextBoxSyncItem::new(
                    format!("{stable_id}::body::param::{}::control::content", param.key),
                    &param.control,
                )
            })
        })
        .collect()
}
