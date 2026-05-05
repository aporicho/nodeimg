use crate::renderer::ImageStyle;
use crate::tree::layout::TextureHandle;

use super::node::ControlNode;

#[derive(Clone, Debug, PartialEq)]
pub enum ControlSpec {
    Button {
        label: String,
    },
    Label {
        text: String,
        muted: bool,
    },
    Image {
        texture: TextureHandle,
        image_style: ImageStyle,
        min_height: f32,
    },
    Group {
        title: String,
        children: Vec<ControlNode>,
    },
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

impl Default for ControlSpec {
    fn default() -> Self {
        Self::ReadOnly {
            value: String::new(),
        }
    }
}
