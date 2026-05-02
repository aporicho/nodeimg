pub(crate) mod anatomy;
pub(crate) mod atoms;
pub(crate) mod build;
mod desc;
pub(crate) mod frameworks;
pub(crate) mod mapping;
pub(crate) mod param_control;
pub(crate) mod props;

pub(crate) mod painters;
pub(crate) mod resize_edge;
pub(crate) mod state;
pub(crate) mod systems;

pub(crate) use crate::text::TextEditState;
pub use atoms::{
    button::ButtonProps,
    checkbox::CheckboxProps,
    color_swatch::{format_color, ColorSwatchProps},
    dot::DotProps,
    dropdown::DropdownProps,
    image_viewer::ImageViewerProps,
    label::{LabelProps, LabelVariant},
    number_input::{format_number, NumberInputProps},
    path_input::PathInputProps,
    radio::RadioProps,
    separator::{SeparatorOrientation, SeparatorProps},
    slider::SliderProps,
    surface::SurfaceProps,
    text_area::TextAreaProps,
    text_box::{TextBoxFont, TextBoxMode, TextBoxProps},
    text_input::TextInputProps,
    toggle::ToggleProps,
    truncated_text::TruncatedTextProps,
};
pub use desc::WidgetDesc;
pub use frameworks::{
    collapsible::CollapsibleProps, group::GroupProps, list_view::ListViewProps, panel::PanelProps,
    scroll_area::ScrollAreaProps,
};
pub use mapping::{ParamControlMap, ParamControlSpec};
pub use props::{WidgetBuild, WidgetBuildCx, WidgetProps, WidgetRole};
pub use resize_edge::ResizeEdge;
