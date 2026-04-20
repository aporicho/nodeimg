#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeSourceKind {
    Builtin,
    Python,
    Api,
}
