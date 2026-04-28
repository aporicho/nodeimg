#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemplateRevision(pub u64);

impl TemplateRevision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}
