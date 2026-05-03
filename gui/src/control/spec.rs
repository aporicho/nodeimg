use crate::renderer::ImageStyle;
use crate::tree::layout::TextureHandle;

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

#[derive(Clone, Debug, PartialEq)]
pub struct ControlNode {
    id: String,
    spec: ControlSpec,
}

impl ControlSpec {
    pub fn button(label: impl Into<String>) -> Self {
        Self::Button {
            label: label.into(),
        }
    }

    pub fn label(text: impl Into<String>) -> Self {
        Self::Label {
            text: text.into(),
            muted: false,
        }
    }

    pub fn muted_label(text: impl Into<String>) -> Self {
        Self::Label {
            text: text.into(),
            muted: true,
        }
    }

    pub fn image(texture: TextureHandle, image_style: ImageStyle) -> Self {
        Self::Image {
            texture,
            image_style,
            min_height: 128.0,
        }
    }

    pub fn image_with_min_height(
        texture: TextureHandle,
        image_style: ImageStyle,
        min_height: f32,
    ) -> Self {
        Self::Image {
            texture,
            image_style,
            min_height,
        }
    }

    pub fn group(title: impl Into<String>, children: Vec<ControlNode>) -> Self {
        Self::Group {
            title: title.into(),
            children,
        }
    }
}

impl ControlNode {
    pub fn new(id: impl Into<String>, spec: ControlSpec) -> Self {
        Self {
            id: id.into(),
            spec,
        }
    }

    pub fn button(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(id, ControlSpec::button(label))
    }

    pub fn label(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, ControlSpec::label(text))
    }

    pub fn muted_label(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, ControlSpec::muted_label(text))
    }

    pub fn image(id: impl Into<String>, texture: TextureHandle, image_style: ImageStyle) -> Self {
        Self::new(id, ControlSpec::image(texture, image_style))
    }

    pub fn group(
        id: impl Into<String>,
        title: impl Into<String>,
        children: Vec<ControlNode>,
    ) -> Self {
        Self::new(id, ControlSpec::group(title, children))
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn spec(&self) -> &ControlSpec {
        &self.spec
    }
}

impl Default for ControlSpec {
    fn default() -> Self {
        Self::ReadOnly {
            value: String::new(),
        }
    }
}

pub trait ControlSpecMap<P> {
    fn control_for_param(&self, param: &P) -> ControlSpec;
}
