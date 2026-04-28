use std::borrow::{Borrow, Cow};
use std::fmt;

pub type TreeNodeId = usize;
pub type NodeId = TreeNodeId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StableId(Cow<'static, str>);

impl StableId {
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

impl AsRef<str> for StableId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for StableId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for StableId {
    fn default() -> Self {
        Self(Cow::Borrowed(""))
    }
}

impl From<Cow<'static, str>> for StableId {
    fn from(value: Cow<'static, str>) -> Self {
        Self::new(value)
    }
}

impl From<&'static str> for StableId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for StableId {
    fn from(value: String) -> Self {
        Self::new(Cow::Owned(value))
    }
}
