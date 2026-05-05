#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Button,
    Label,
    Image,
    Group,
    ReadOnly,
    Text,
    TextArea,
    Number,
    Slider,
    Toggle,
    Select,
    Color,
    FilePath,
}
