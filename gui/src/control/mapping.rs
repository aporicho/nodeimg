#[derive(Clone, Debug, PartialEq)]
pub enum ParamControlSpec {
    ReadOnly {
        value: String,
    },
    Text {
        value: String,
    },
    TextArea {
        value: String,
        min_rows: usize,
    },
    Number {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        precision: usize,
    },
    Slider {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    Toggle {
        checked: bool,
    },
    Select {
        options: Vec<String>,
        selected: usize,
    },
    Color {
        rgba: [f32; 4],
    },
    FilePath {
        path: String,
        extensions: Vec<String>,
    },
}

impl Default for ParamControlSpec {
    fn default() -> Self {
        Self::ReadOnly {
            value: String::new(),
        }
    }
}

pub trait ParamControlMap<P> {
    fn control_for_param(&self, param: &P) -> ParamControlSpec;
}
