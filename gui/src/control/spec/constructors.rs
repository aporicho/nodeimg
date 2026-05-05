use crate::renderer::ImageStyle;
use crate::tree::layout::TextureHandle;

use super::{ControlNode, ControlSpec};

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
        Self::image_with_min_height(texture, image_style, 128.0)
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

    pub fn read_only(value: impl Into<String>) -> Self {
        Self::ReadOnly {
            value: value.into(),
        }
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::Text {
            value: value.into(),
        }
    }

    pub fn text_area(value: impl Into<String>, min_rows: usize) -> Self {
        Self::TextArea {
            value: value.into(),
            min_rows,
        }
    }

    pub fn number(value: f32, min: f32, max: f32, step: f32, precision: usize) -> Self {
        Self::Number {
            value,
            min,
            max,
            step,
            precision,
        }
    }

    pub fn slider(value: f32, min: f32, max: f32, step: f32) -> Self {
        Self::Slider {
            value,
            min,
            max,
            step,
        }
    }

    pub fn toggle(checked: bool) -> Self {
        Self::Toggle { checked }
    }

    pub fn select(options: Vec<String>, selected: usize) -> Self {
        Self::Select { options, selected }
    }

    pub fn color(rgba: [f32; 4]) -> Self {
        Self::Color { rgba }
    }

    pub fn file_path(path: impl Into<String>, extensions: Vec<String>) -> Self {
        Self::FilePath {
            path: path.into(),
            extensions,
        }
    }
}

impl ControlNode {
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

    pub fn image_with_min_height(
        id: impl Into<String>,
        texture: TextureHandle,
        image_style: ImageStyle,
        min_height: f32,
    ) -> Self {
        Self::new(
            id,
            ControlSpec::image_with_min_height(texture, image_style, min_height),
        )
    }

    pub fn group(
        id: impl Into<String>,
        title: impl Into<String>,
        children: Vec<ControlNode>,
    ) -> Self {
        Self::new(id, ControlSpec::group(title, children))
    }

    pub fn read_only(id: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(id, ControlSpec::read_only(value))
    }

    pub fn text(id: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(id, ControlSpec::text(value))
    }

    pub fn text_area(id: impl Into<String>, value: impl Into<String>, min_rows: usize) -> Self {
        Self::new(id, ControlSpec::text_area(value, min_rows))
    }

    pub fn number(
        id: impl Into<String>,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        precision: usize,
    ) -> Self {
        Self::new(id, ControlSpec::number(value, min, max, step, precision))
    }

    pub fn slider(id: impl Into<String>, value: f32, min: f32, max: f32, step: f32) -> Self {
        Self::new(id, ControlSpec::slider(value, min, max, step))
    }

    pub fn toggle(id: impl Into<String>, checked: bool) -> Self {
        Self::new(id, ControlSpec::toggle(checked))
    }

    pub fn select(id: impl Into<String>, options: Vec<String>, selected: usize) -> Self {
        Self::new(id, ControlSpec::select(options, selected))
    }

    pub fn color(id: impl Into<String>, rgba: [f32; 4]) -> Self {
        Self::new(id, ControlSpec::color(rgba))
    }

    pub fn file_path(
        id: impl Into<String>,
        path: impl Into<String>,
        extensions: Vec<String>,
    ) -> Self {
        Self::new(id, ControlSpec::file_path(path, extensions))
    }
}
