use crate::renderer::Rect;
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PanelId(Cow<'static, str>);

impl PanelId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn into_cow(self) -> Cow<'static, str> {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct PanelConfig {
    pub id: PanelId,
    pub title: Cow<'static, str>,
    pub default_rect: Rect,
    pub min_size: [f32; 2],
    pub titlebar_visible: bool,
    pub draggable: bool,
    pub resizable: bool,
    pub closable: bool,
    pub initially_visible: bool,
}
