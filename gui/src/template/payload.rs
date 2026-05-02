use super::SlotValues;

#[derive(Clone, Debug, Default)]
pub enum TemplatePayload {
    #[default]
    Empty,
    Slots(SlotValues),
    CanvasNodeCard(crate::canvas::retained_node_card::CanvasNodeCardTemplateData),
    DropdownOverlay(crate::overlay::retained::DropdownOverlayTemplateData),
    PanelFrame(crate::panel::retained::PanelFrameTemplateData),
}

impl TemplatePayload {
    pub fn into_slots(self) -> Option<SlotValues> {
        match self {
            Self::Empty => Some(SlotValues::default()),
            Self::Slots(slots) => Some(slots),
            Self::CanvasNodeCard(_) => None,
            Self::DropdownOverlay(_) => None,
            Self::PanelFrame(_) => None,
        }
    }
}

impl From<SlotValues> for TemplatePayload {
    fn from(value: SlotValues) -> Self {
        Self::Slots(value)
    }
}
