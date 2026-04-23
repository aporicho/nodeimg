use std::borrow::Cow;

use super::IconName;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconId(String);

impl IconId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for IconId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for IconId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<Cow<'static, str>> for IconId {
    fn from(value: Cow<'static, str>) -> Self {
        Self::new(value.into_owned())
    }
}

impl From<IconName> for IconId {
    fn from(value: IconName) -> Self {
        Self::new(value.as_str())
    }
}

impl std::borrow::Borrow<str> for IconId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
