use std::borrow::Cow;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TemplateId(Cow<'static, str>);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstanceId(Cow<'static, str>);

impl TemplateId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl InstanceId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&'static str> for TemplateId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for TemplateId {
    fn from(value: String) -> Self {
        Self::new(Cow::Owned(value))
    }
}

impl From<&'static str> for InstanceId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for InstanceId {
    fn from(value: String) -> Self {
        Self::new(Cow::Owned(value))
    }
}
