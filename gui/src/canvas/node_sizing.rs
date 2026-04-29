use super::node_style::NodeCardMetrics;
use super::node_template::{CanvasNodeParamTemplate, CanvasNodeTemplate};
use super::{canvas_node_stable_id, CanvasNodeLayout};
use crate::runtime::ControlIntrinsic;
use crate::theme::Theme;
use crate::widget::mapping::ParamControlSpec;
use crate::widget::param_control::{
    param_control_kind, param_control_layout_policy, param_control_min_height,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasNodeSizingRequest {
    pub min_width: f32,
    pub min_height: f32,
    pub target_width: f32,
    pub target_height: f32,
    pub missing_auto_height_intrinsics: usize,
}

pub fn canvas_node_sizing_request(
    template: &CanvasNodeTemplate,
    layout: &CanvasNodeLayout,
    control_intrinsics: &[ControlIntrinsic],
    theme: &Theme,
) -> CanvasNodeSizingRequest {
    let (min_width, min_height) = canvas_node_template_min_size(template, theme);
    let metrics = NodeCardMetrics::from_theme(theme);
    let desired_body = body_desired_height(
        &template.params,
        &layout.owner_id,
        control_intrinsics,
        metrics,
        theme,
    );
    let desired_card_height = desired_body.height + metrics.card_padding * 2.0;
    let settled_target_height = min_height
        .max(desired_card_height)
        .max(layout.user_min_height.unwrap_or(0.0));
    let target_height = if desired_body.missing_auto_height_intrinsics > 0 {
        layout
            .rect
            .h
            .max(min_height)
            .max(layout.user_min_height.unwrap_or(0.0))
    } else {
        settled_target_height
    };
    let stable_id = canvas_node_stable_id(&layout.owner_id);
    let owner_intrinsic_count = control_intrinsics
        .iter()
        .filter(|intrinsic| intrinsic.widget_id.starts_with(stable_id.as_str()))
        .count();
    let height_delta = target_height - layout.rect.h;
    if owner_intrinsic_count > 0
        || height_delta.abs() > 0.5
        || desired_body.missing_auto_height_intrinsics > 0
    {
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            owner_id = %layout.owner_id,
            stable_id = %stable_id,
            current_w = layout.rect.w,
            current_h = layout.rect.h,
            min_w = min_width,
            min_h = min_height,
            desired_body_h = desired_body.height,
            desired_card_h = desired_card_height,
            target_w = layout.rect.w.max(min_width),
            target_h = target_height,
            settled_target_h = settled_target_height,
            height_delta,
            user_min_height = layout.user_min_height,
            total_intrinsic_count = control_intrinsics.len(),
            owner_intrinsic_count,
            missing_auto_height_intrinsics = desired_body.missing_auto_height_intrinsics,
            "calculate canvas node sizing request"
        );
    }

    CanvasNodeSizingRequest {
        min_width,
        min_height,
        target_width: layout.rect.w.max(min_width),
        target_height,
        missing_auto_height_intrinsics: desired_body.missing_auto_height_intrinsics,
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct BodyDesiredHeight {
    height: f32,
    missing_auto_height_intrinsics: usize,
}

fn body_desired_height(
    params: &[CanvasNodeParamTemplate],
    owner_id: &str,
    control_intrinsics: &[ControlIntrinsic],
    metrics: NodeCardMetrics,
    theme: &Theme,
) -> BodyDesiredHeight {
    if params.is_empty() {
        return BodyDesiredHeight {
            height: metrics.param_row_height,
            missing_auto_height_intrinsics: 0,
        };
    }

    let rows = params.iter().enumerate().fold(
        BodyDesiredHeight::default(),
        |mut total, (index, param)| {
            let row = row_desired_height(
                index,
                &param.control,
                owner_id,
                control_intrinsics,
                metrics,
                theme,
            );
            total.height += row.height;
            if row.missing_auto_height_intrinsic {
                total.missing_auto_height_intrinsics += 1;
            }
            total
        },
    );
    BodyDesiredHeight {
        height: rows.height + metrics.row_gap * params.len().saturating_sub(1) as f32,
        missing_auto_height_intrinsics: rows.missing_auto_height_intrinsics,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RowDesiredHeight {
    height: f32,
    missing_auto_height_intrinsic: bool,
}

fn row_desired_height(
    index: usize,
    control: &ParamControlSpec,
    owner_id: &str,
    control_intrinsics: &[ControlIntrinsic],
    metrics: NodeCardMetrics,
    theme: &Theme,
) -> RowDesiredHeight {
    let min_height = row_min_height(control, metrics, theme);
    let policy = param_control_layout_policy(control, theme, metrics.control);
    let control_kind = param_control_kind(control);
    let widget_id = format!(
        "{}::body::param::{index}::control::widget",
        canvas_node_stable_id(owner_id)
    );
    let intrinsic = control_intrinsics
        .iter()
        .find(|intrinsic| intrinsic.widget_id == widget_id && intrinsic.affects_parent_height);
    match intrinsic {
        Some(intrinsic) => {
            let desired_height = intrinsic.desired_size[1]
                .max(intrinsic.min_size[1])
                .max(min_height);
            if (desired_height - min_height).abs() > 0.5
                || (desired_height - intrinsic.current_size[1]).abs() > 0.5
            {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    owner_id,
                    index,
                    widget_id = %intrinsic.widget_id,
                    control_kind = ?control_kind,
                    row_min_h = min_height,
                    current_h = intrinsic.current_size[1],
                    intrinsic_min_h = intrinsic.min_size[1],
                    intrinsic_desired_h = intrinsic.desired_size[1],
                    row_desired_h = desired_height,
                    "use control intrinsic for canvas node row height"
                );
            }
            RowDesiredHeight {
                height: desired_height,
                missing_auto_height_intrinsic: false,
            }
        }
        None => {
            if policy.affects_parent_height {
                tracing::trace!(
                    target: "nodeimg::render_trace::node",
                    owner_id,
                    index,
                    expected_widget_id = %widget_id,
                    control_kind = ?control_kind,
                    total_intrinsic_count = control_intrinsics.len(),
                    row_min_h = min_height,
                    "missing auto-height control intrinsic for canvas node row"
                );
            }
            RowDesiredHeight {
                height: min_height,
                missing_auto_height_intrinsic: policy.affects_parent_height,
            }
        }
    }
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
        assert_eq!(request.missing_auto_height_intrinsics, 0);
    }

    #[test]
    fn sizing_request_preserves_current_height_until_auto_height_intrinsic_exists() {
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
        assert_eq!(request.target_height, layout.rect.h);
        assert!(request.min_width < layout.rect.w);
        assert!(request.min_height < layout.rect.h);
        assert_eq!(request.missing_auto_height_intrinsics, 1);
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
        assert_eq!(request.missing_auto_height_intrinsics, 1);
    }
}
