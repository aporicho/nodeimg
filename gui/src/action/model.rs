#[derive(Clone, Debug, PartialEq)]
pub enum GuiAction {
    AddNode { type_id: String },
    OpenOverlay { id: String },
    CloseOverlay { id: Option<String> },
    ControlClicked { id: String },
}
