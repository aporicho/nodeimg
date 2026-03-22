#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WidgetId(pub String);

impl WidgetId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}
