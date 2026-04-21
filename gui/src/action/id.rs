use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionId(Cow<'static, str>);

impl ActionId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<&'static str> for ActionId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ActionId {
    fn from(value: String) -> Self {
        Self::new(Cow::Owned(value))
    }
}
