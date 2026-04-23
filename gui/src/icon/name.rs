#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconName(&'static str);

impl IconName {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}
