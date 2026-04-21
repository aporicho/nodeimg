use crate::renderer::Rect;

#[derive(Clone, Debug)]
pub struct PanelLayout {
    pub id: String,
    pub rect: Rect,
    pub visible: bool,
    pub z_index: i32,
    pub collapsed: bool,
}
