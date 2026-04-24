use super::catalog::PlaygroundItemId;

pub(super) const PLAYGROUND_PAGE_ID: &str = "developer_playground_page";
pub(super) const PLAYGROUND_CANVAS_ID: &str = "developer_playground_canvas";
pub(super) const PLAYGROUND_GRID_ID: &str = "developer_playground_grid";
pub(super) const PLAYGROUND_TILE_PREFIX: &str = "playground_item::";

pub(super) const CONTROL_LABEL_ID: &str = "playground_control_label";
pub(super) const CONTROL_BUTTON_ID: &str = "playground_control_button";
pub(super) const CONTROL_TEXT_INPUT_ID: &str = "playground_control_text_input";
pub(super) const CONTROL_SLIDER_ID: &str = "playground_control_slider";
pub(super) const CONTROL_SLIDER_TRACK_ID: &str = "playground_control_slider::track";
pub(super) const CONTROL_NUMBER_INPUT_ID: &str = "playground_control_number_input";
pub(super) const CONTROL_TOGGLE_ID: &str = "playground_control_toggle";
pub(super) const CONTROL_CHECKBOX_ID: &str = "playground_control_checkbox";
pub(super) const CONTROL_RADIO_FAST_ID: &str = "playground_control_radio_fast";
pub(super) const CONTROL_RADIO_BALANCED_ID: &str = "playground_control_radio_balanced";
pub(super) const CONTROL_DROPDOWN_ID: &str = "playground_control_dropdown";
pub(super) const CONTROL_IMAGE_VIEWER_ID: &str = "playground_control_image_viewer";
pub(super) const CONTAINER_COLLAPSIBLE_ID: &str = "playground_container_collapsible";
pub(super) const CONTROL_POPUP_BUTTON_ID: &str = "playground_control_popup_button";
pub(super) const CONTROL_POPUP_CLOSE_ID: &str = "playground_control_popup_close";
pub(super) const CONTROL_POPUP_OVERLAY_ID: &str = "playground_control_popup_overlay";

pub(super) fn tile_id(item: PlaygroundItemId) -> String {
    format!("{PLAYGROUND_TILE_PREFIX}{}", item.key())
}

pub(super) fn tile_stage_id(item: PlaygroundItemId) -> String {
    format!("{}::stage", tile_id(item))
}

pub(super) fn tile_title_id(item: PlaygroundItemId) -> String {
    format!("{}::title", tile_id(item))
}

pub(super) fn tile_description_id(item: PlaygroundItemId) -> String {
    format!("{}::description", tile_id(item))
}

pub(super) fn item_from_tile_id(id: &str) -> Option<PlaygroundItemId> {
    let key = id.strip_prefix(PLAYGROUND_TILE_PREFIX)?;
    if key.contains("::") {
        return None;
    }
    PlaygroundItemId::from_key(key)
}

pub(super) fn is_tile_drag_target(id: &str) -> bool {
    item_from_tile_id(id).is_some()
}

pub(super) fn is_slider_target(id: &str) -> bool {
    id == CONTROL_SLIDER_ID || id.starts_with("playground_control_slider::")
}
