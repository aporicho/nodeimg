#[derive(Clone, Debug, PartialEq)]
pub enum ControlValue {
    Text(String),
    Number(f32),
    Bool(bool),
    Selection(usize),
    Color([f32; 4]),
    FilePath(String),
}
