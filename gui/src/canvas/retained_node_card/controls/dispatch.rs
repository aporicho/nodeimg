use super::color::mount_color_control;
use super::slider::mount_slider;
use super::text::mount_control_text;
use super::text_field::mount_text_field;
use super::toggle::mount_toggle;
use crate::canvas::node_style::NodeCardMetrics;
use crate::control::{ControlRole, ParamControlHeight, ParamControlLayoutPolicy, ParamControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Direction, Size};
use crate::tree::NodeId;

use super::super::node_factory::container;

pub(in crate::canvas::retained_node_card) fn mount_param_control(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ParamControlSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
    policy: ParamControlLayoutPolicy,
) -> Result<(), TemplateError> {
    let wrapper = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fixed(metrics.control.control_width),
                height: match policy.height {
                    ParamControlHeight::Fixed(height) => Size::Fixed(height),
                    ParamControlHeight::Fill { .. } => Size::Fill,
                },
                min_height: policy.min_height(),
                flex_grow: control_flex(policy),
                direction: Direction::Row,
                align_items: policy.wrapper_align,
                justify_content: crate::tree::layout::Justify::Start,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let child_id = format!("{id}::content");
    match control {
        ParamControlSpec::Text { value } => mount_text_field(
            cx,
            wrapper,
            &child_id,
            value,
            false,
            1,
            ControlRole::TextInput,
            theme,
            metrics,
        )?,
        ParamControlSpec::TextArea { value, min_rows } => mount_text_field(
            cx,
            wrapper,
            &child_id,
            value,
            true,
            *min_rows,
            ControlRole::TextArea,
            theme,
            metrics,
        )?,
        ParamControlSpec::ReadOnly { value } => {
            mount_control_text(cx, wrapper, &child_id, value, theme)?
        }
        ParamControlSpec::Number {
            value, precision, ..
        } => mount_text_field(
            cx,
            wrapper,
            &child_id,
            &format!("{value:.precision$}"),
            false,
            1,
            ControlRole::NumberInput,
            theme,
            metrics,
        )?,
        ParamControlSpec::Slider {
            value, min, max, ..
        } => mount_slider(cx, wrapper, &child_id, *value, *min, *max, theme, metrics)?,
        ParamControlSpec::Toggle { checked } => {
            mount_toggle(cx, wrapper, &child_id, *checked, theme, metrics)?
        }
        ParamControlSpec::Select { options, selected } => {
            let value = options.get(*selected).cloned().unwrap_or_default();
            mount_control_text(cx, wrapper, &child_id, &value, theme)?
        }
        ParamControlSpec::Color { rgba } => {
            mount_color_control(cx, wrapper, &child_id, *rgba, theme, metrics)?
        }
        ParamControlSpec::FilePath { path, .. } => {
            mount_control_text(cx, wrapper, &child_id, path, theme)?
        }
    }
    Ok(())
}

fn control_flex(policy: ParamControlLayoutPolicy) -> f32 {
    policy.fills_parent_height().then_some(1.0).unwrap_or(0.0)
}
