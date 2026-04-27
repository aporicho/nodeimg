use super::node_style::NodeCardMetrics;
use super::node_template::{CanvasNodeParamTemplate, CanvasNodeTemplate};
use super::{canvas_node_stable_id, CanvasNodeLayout};
use crate::runtime::ControlIntrinsic;
use crate::theme::Theme;
use crate::widget::mapping::ParamControlSpec;
use crate::widget::param_control::param_control_min_height;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasNodeSizingRequest {
    pub min_width: f32,
    pub min_height: f32,
    pub target_width: f32,
    pub target_height: f32,
}

pub fn canvas_node_sizing_request(
    template: &CanvasNodeTemplate,
    layout: &CanvasNodeLayout,
    control_intrinsics: &[ControlIntrinsic],
    theme: &Theme,
) -> CanvasNodeSizingRequest {
    let (min_width, min_height) = canvas_node_template_min_size(template, theme);
    let metrics = NodeCardMetrics::from_theme(theme);
    let desired_body_height = body_desired_height(
        &template.params,
        &layout.owner_id,
        control_intrinsics,
        metrics,
        theme,
    );
    let desired_card_height = desired_body_height + metrics.card_padding * 2.0;
    let target_height = min_height
        .max(desired_card_height)
        .max(layout.user_min_height.unwrap_or(0.0));

    CanvasNodeSizingRequest {
        min_width,
        min_height,
        target_width: layout.rect.w.max(min_width),
        target_height,
    }
}

pub fn canvas_node_template_min_size(template: &CanvasNodeTemplate, theme: &Theme) -> (f32, f32) {
    let metrics = NodeCardMetrics::from_theme(theme);
    let body_height = body_min_height(&template.params, metrics, theme);
    (
        metrics.card_width,
        metrics
            .card_height
            .max(body_height + metrics.card_padding * 2.0),
    )
}

fn body_min_height(
    params: &[CanvasNodeParamTemplate],
    metrics: NodeCardMetrics,
    theme: &Theme,
) -> f32 {
    let row_count = params.len().max(1);
    params
        .iter()
        .map(|param| row_min_height(&param.control, metrics, theme))
        .chain((params.is_empty()).then_some(metrics.param_row_height))
        .sum::<f32>()
        + metrics.row_gap * row_count.saturating_sub(1) as f32
}

fn row_min_height(control: &ParamControlSpec, metrics: NodeCardMetrics, theme: &Theme) -> f32 {
    param_control_min_height(control, theme, metrics.control).max(metrics.param_row_height)
}

fn body_desired_height(
    params: &[CanvasNodeParamTemplate],
    owner_id: &str,
    control_intrinsics: &[ControlIntrinsic],
    metrics: NodeCardMetrics,
    theme: &Theme,
) -> f32 {
    if params.is_empty() {
        return metrics.param_row_height;
    }

    params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            row_desired_height(
                index,
                &param.control,
                owner_id,
                control_intrinsics,
                metrics,
                theme,
            )
        })
        .sum::<f32>()
        + metrics.row_gap * params.len().saturating_sub(1) as f32
}

fn row_desired_height(
    index: usize,
    control: &ParamControlSpec,
    owner_id: &str,
    control_intrinsics: &[ControlIntrinsic],
    metrics: NodeCardMetrics,
    theme: &Theme,
) -> f32 {
    let min_height = row_min_height(control, metrics, theme);
    let widget_id = format!(
        "{}::body::param::{index}::control::widget",
        canvas_node_stable_id(owner_id)
    );
    control_intrinsics
        .iter()
        .find(|intrinsic| intrinsic.widget_id == widget_id && intrinsic.affects_parent_height)
        .map(|intrinsic| {
            intrinsic.desired_size[1]
                .max(intrinsic.min_size[1])
                .max(min_height)
        })
        .unwrap_or(min_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::node_template::CanvasNodeParamTemplate;
    use crate::renderer::Rect;
    use crate::theme::light_theme;

    fn text_area_template() -> CanvasNodeTemplate {
        CanvasNodeTemplate {
            type_id: "text_area".to_string(),
            title: "Text Area".to_string(),
            subtitle: String::new(),
            category: "ui".to_string(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            params: vec![CanvasNodeParamTemplate::new(
                "prompt",
                "",
                "",
                "",
                ParamControlSpec::TextArea {
                    value: "hello".to_string(),
                    min_rows: 5,
                },
            )],
        }
    }

    #[test]
    fn sizing_request_uses_control_desired_height_as_absolute_target() {
        let theme = light_theme();
        let template = text_area_template();
        let layout = CanvasNodeLayout {
            owner_id: "engine_node::7".to_string(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 2500.0,
                h: 1300.0,
            },
            z_index: 0,
            collapsed: false,
            user_min_height: None,
        };
        let intrinsics = vec![ControlIntrinsic {
            widget_id: "canvas_node::engine_node::7::body::param::0::control::widget".to_string(),
            current_size: [252.0, 72.0],
            min_size: [252.0, 72.0],
            desired_size: [252.0, 220.0],
            affects_parent_width: false,
            affects_parent_height: true,
        }];

        let request = canvas_node_sizing_request(&template, &layout, &intrinsics, &theme);
        let metrics = NodeCardMetrics::from_theme(&theme);

        assert_eq!(request.target_width, 2500.0);
        assert_eq!(request.target_height, 220.0 + metrics.card_padding * 2.0);
    }

    #[test]
    fn sizing_request_does_not_make_manual_size_the_minimum() {
        let theme = light_theme();
        let template = text_area_template();
        let layout = CanvasNodeLayout {
            owner_id: "engine_node::7".to_string(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 2500.0,
                h: 1300.0,
            },
            z_index: 0,
            collapsed: false,
            user_min_height: None,
        };

        let request = canvas_node_sizing_request(&template, &layout, &[], &theme);

        assert_eq!(request.target_width, layout.rect.w);
        assert!(request.target_height < layout.rect.h);
        assert!(request.min_width < layout.rect.w);
        assert!(request.min_height < layout.rect.h);
    }

    #[test]
    fn sizing_request_respects_user_height_floor() {
        let theme = light_theme();
        let template = text_area_template();
        let layout = CanvasNodeLayout {
            owner_id: "engine_node::7".to_string(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 304.0,
                h: 260.0,
            },
            z_index: 0,
            collapsed: false,
            user_min_height: Some(260.0),
        };

        let request = canvas_node_sizing_request(&template, &layout, &[], &theme);

        assert_eq!(request.target_height, 260.0);
    }
}
