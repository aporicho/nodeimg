use super::OverlayPlacement;
use crate::theme::{ControlSize, Density};
use std::borrow::Cow;

pub struct OverlayRequest {
    pub id: String,
    pub anchor_id: String,
    pub restore_focus_id: Option<String>,
    pub placement: OverlayPlacement,
    pub content: OverlayContent,
    pub offset_x: f32,
    pub offset_y: f32,
    pub match_anchor_width: bool,
    pub dismiss_on_escape: bool,
    pub dismiss_on_outside_click: bool,
    pub restore_focus_to_anchor: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OverlayContent {
    DropdownOptions(DropdownOverlayContent),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DropdownOverlayContent {
    pub dropdown_id: String,
    pub title: Cow<'static, str>,
    pub options: Vec<Cow<'static, str>>,
    pub selected: usize,
    pub highlighted: usize,
    pub size: ControlSize,
    pub density: Density,
}
