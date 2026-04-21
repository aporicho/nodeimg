use super::OverlayPlacement;
use crate::tree::Desc;

pub struct OverlayRequest {
    pub id: String,
    pub anchor_id: String,
    pub restore_focus_id: Option<String>,
    pub placement: OverlayPlacement,
    pub content: Desc,
    pub offset_x: f32,
    pub offset_y: f32,
    pub match_anchor_width: bool,
    pub dismiss_on_escape: bool,
    pub dismiss_on_outside_click: bool,
    pub restore_focus_to_anchor: bool,
}
