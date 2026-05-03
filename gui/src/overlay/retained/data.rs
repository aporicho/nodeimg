use super::super::OverlayContent;
use crate::theme::Theme;

#[derive(Clone, Debug)]
pub struct DropdownOverlayTemplateData {
    pub(in crate::overlay::retained) overlay_id: String,
    pub(in crate::overlay::retained) x: f32,
    pub(in crate::overlay::retained) y: f32,
    pub(in crate::overlay::retained) width: Option<f32>,
    pub(in crate::overlay::retained) content: OverlayContent,
    pub(in crate::overlay::retained) theme: Theme,
}

impl DropdownOverlayTemplateData {
    pub fn new(
        overlay_id: String,
        x: f32,
        y: f32,
        width: Option<f32>,
        content: OverlayContent,
        theme: &Theme,
    ) -> Self {
        Self {
            overlay_id,
            x,
            y,
            width,
            content,
            theme: theme.clone(),
        }
    }
}
