use std::borrow::Cow;

use super::node_spec::{
    node_render_spec, NodeBodyRowSpec, NodeHeaderSpec, NodePortGroupTriggerSpec, NodePortSpec,
    NodeRenderSpec,
};
use super::node_style::NodeCardMetrics;
use super::node_template::CanvasNodeRenderView;
use super::{CanvasPortConnectionState, CanvasPortSide};
use crate::gesture::Gesture;
use crate::icon::{names, IconSpec};
use crate::renderer::{Border, Color, Rect};
use crate::template::{
    InstanceId, RetainedTemplate, TemplateError, TemplateId, TemplateInstance, TemplatePayload,
    TemplateRevision, CANVAS_NODE_CARD_TEMPLATE,
};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Overflow, Position,
    RelayoutBoundaryReason, Size, TextLayout, TextOverflow,
};
use crate::tree::{
    NodeId, NodeKind, NodeLayoutMeta, NodeLocalRuntime, NodeMutationMeta, NodePaintMeta, NodeProps,
    RectMoveInvalidation, RepaintBoundaryReason, RuntimeSlots, StableId, Tree, TreeNode,
};
use crate::widget::mapping::ParamControlSpec;
use crate::widget::param_control::{
    param_control_layout_policy, ParamControlHeight, ParamControlLayoutPolicy,
};

const REVISION: TemplateRevision = TemplateRevision::new(2);

#[derive(Clone, Debug)]
pub struct CanvasNodeCardTemplateData {
    view: CanvasNodeRenderView,
    theme: Theme,
}

impl CanvasNodeCardTemplateData {
    pub fn new(view: CanvasNodeRenderView, theme: &Theme) -> Self {
        Self {
            view,
            theme: theme.clone(),
        }
    }
}

pub struct CanvasNodeCardRetainedTemplate;

impl RetainedTemplate for CanvasNodeCardRetainedTemplate {
    fn id(&self) -> TemplateId {
        TemplateId::from(CANVAS_NODE_CARD_TEMPLATE)
    }

    fn revision(&self) -> TemplateRevision {
        REVISION
    }

    fn instantiate(
        &self,
        tree: &mut Tree,
        instance: InstanceId,
        parent: NodeId,
        payload: TemplatePayload,
    ) -> Result<TemplateInstance, TemplateError> {
        let TemplatePayload::CanvasNodeCard(data) = payload else {
            return Err(TemplateError::UnsupportedPayload {
                template: self.id(),
                reason: "canvas node card template requires canvas node card payload",
            });
        };
        let spec = node_render_spec(&data.view.template, &data.view.state, &data.theme);
        let mut mount = MountCx::new(tree);
        let root = mount_node_card(&mut mount, parent, &spec, &data.theme)?;
        Ok(TemplateInstance {
            template: self.id(),
            template_revision: self.revision(),
            instance,
            root_node: root,
        })
    }
}

struct MountCx<'a> {
    tree: &'a mut Tree,
    mounted: Vec<NodeId>,
}

impl<'a> MountCx<'a> {
    fn new(tree: &'a mut Tree) -> Self {
        Self {
            tree,
            mounted: Vec::new(),
        }
    }

    fn child(&mut self, parent: NodeId, node: TreeNode) -> Result<NodeId, TemplateError> {
        let id = self.tree.insert_checked(node)?;
        self.mounted.push(id);
        if !self.tree.append_child(parent, id) {
            self.rollback();
            return Err(TemplateError::MissingParent(parent));
        }
        Ok(id)
    }

    fn rollback(&mut self) {
        for id in self.mounted.iter().rev().copied() {
            self.tree.detach_from_parent(id);
            self.tree.remove(id);
        }
        self.mounted.clear();
    }
}

fn mount_node_card(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    spec: &NodeRenderSpec,
    theme: &Theme,
) -> Result<NodeId, TemplateError> {
    let metrics = spec.metrics;
    let root = cx.child(
        parent,
        container(
            spec.id.clone(),
            BoxStyle {
                position: Position::absolute_xy(spec.layout.rect.x, spec.layout.rect.y),
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Row,
                gap: metrics.column_gap,
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Start,
                overflow: Overflow::Visible,
                z_index: spec.layout.z_index,
                hittable: Some(true),
                draggable: true,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            None,
        )
        .with_layout_boundary(RelayoutBoundaryReason::CanvasNodeCard)
        .with_paint_boundary(RepaintBoundaryReason::CanvasNodeCard)
        .with_rect_move_invalidation(RectMoveInvalidation::BoundaryPlacement),
    )?;

    mount_pin_column(
        cx,
        root,
        &spec.input_column_id,
        &spec.input_trigger,
        CanvasPortSide::Input,
        &spec.inputs,
        theme,
        metrics,
    )?;
    let card = mount_card(cx, root, spec, theme, metrics)?;
    mount_node_body(cx, card, &spec.body, theme, metrics)?;
    mount_node_header(cx, card, &spec.header, theme, metrics)?;
    mount_pin_column(
        cx,
        root,
        &spec.output_column_id,
        &spec.output_trigger,
        CanvasPortSide::Output,
        &spec.outputs,
        theme,
        metrics,
    )?;

    Ok(root)
}

fn mount_card(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    spec: &NodeRenderSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<NodeId, TemplateError> {
    cx.child(
        parent,
        container(
            spec.card_id.clone(),
            BoxStyle {
                position: Position::relative(),
                width: Size::Fixed(metrics.card_width),
                height: Size::Fixed(metrics.card_height),
                padding: Edges::all(metrics.card_padding),
                direction: Direction::Column,
                gap: metrics.row_gap,
                align_items: Align::Start,
                justify_content: crate::tree::layout::Justify::Start,
                overflow: Overflow::Visible,
                hittable: Some(true),
                resizable: true,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: if spec.selected { 2.0 } else { 1.0 },
                    color: if spec.selected {
                        theme.colors.text
                    } else {
                        theme.colors.border
                    },
                }),
                radius: [metrics.card_radius; 4],
                shadow: None,
            }),
        ),
    )
}

fn mount_pin_column(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    trigger: &NodePortGroupTriggerSpec,
    side: CanvasPortSide,
    ports: &[NodePortSpec],
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let column = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                position: Position::relative(),
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Column,
                gap: metrics.pin_row_gap,
                align_items: Align::Start,
                justify_content: crate::tree::layout::Justify::Start,
                overflow: Overflow::Visible,
                hittable: Some(false),
                ..BoxStyle::default()
            },
            None,
        ),
    )?;

    for port in ports {
        mount_pin_row(cx, column, side, port, theme, metrics)?;
    }
    if !ports.is_empty() {
        mount_port_group_trigger(cx, column, trigger, theme, metrics)?;
    }
    Ok(())
}

fn mount_port_group_trigger(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    trigger: &NodePortGroupTriggerSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let diameter = metrics.port_group_trigger_diameter;
    let (x, y) = port_group_trigger_offset(trigger.side, metrics);
    let border_color = if trigger.open {
        theme.colors.accent
    } else {
        theme.colors.border
    };
    let icon_color = if trigger.open {
        theme.colors.accent
    } else {
        theme.colors.text_muted
    };
    let trigger_node = cx.child(
        parent,
        container(
            trigger.id.clone(),
            BoxStyle {
                position: Position::absolute_xy(x, y),
                width: Size::Fixed(diameter),
                height: Size::Fixed(diameter),
                direction: Direction::Row,
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Center,
                hittable: Some(true),
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: if trigger.open { 2.0 } else { 1.5 },
                    color: border_color,
                }),
                radius: [diameter * 0.5; 4],
                shadow: None,
            }),
        ),
    )?;
    cx.child(
        trigger_node,
        leaf(
            format!("{}::icon", trigger.id),
            LeafKind::Icon {
                spec: IconSpec::new(names::PLUS, icon_color),
            },
            BoxStyle {
                width: Size::Fixed(metrics.port_group_trigger_icon_size),
                height: Size::Fixed(metrics.port_group_trigger_icon_size),
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn port_group_trigger_offset(side: CanvasPortSide, metrics: NodeCardMetrics) -> (f32, f32) {
    let diameter = metrics.port_group_trigger_diameter;
    let dot_center_offset = (metrics.pin_dot_diameter - diameter) * 0.5;
    let x = match side {
        CanvasPortSide::Input => dot_center_offset,
        CanvasPortSide::Output => {
            metrics.pin_label_width + metrics.pin_label_gap + dot_center_offset
        }
    };
    let y = -diameter - metrics.pin_row_gap;
    (x, y)
}

fn mount_pin_row(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    side: CanvasPortSide,
    port: &NodePortSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let row = cx.child(
        parent,
        container(
            port.row_id.clone(),
            BoxStyle {
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Row,
                gap: metrics.pin_label_gap,
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Start,
                hittable: Some(true),
                gestures: vec![Gesture::Tap, Gesture::Drag],
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    if side == CanvasPortSide::Output {
        mount_pin_label(cx, row, port, theme, metrics)?;
        mount_pin_dot(cx, row, port, theme, metrics)?;
    } else {
        mount_pin_dot(cx, row, port, theme, metrics)?;
        mount_pin_label(cx, row, port, theme, metrics)?;
    }
    Ok(())
}

fn mount_pin_dot(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    port: &NodePortSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    cx.child(
        parent,
        leaf(
            port.dot_id.clone(),
            LeafKind::Circle {
                radius: metrics.pin_dot_diameter * 0.5,
                fill: Some(port_state_color(port, theme)),
                stroke: Some(Border {
                    width: match port.connection_state {
                        CanvasPortConnectionState::Idle => 1.0,
                        CanvasPortConnectionState::Source
                        | CanvasPortConnectionState::CompatibleTarget
                        | CanvasPortConnectionState::IncompatibleTarget
                        | CanvasPortConnectionState::DropTarget
                        | CanvasPortConnectionState::RejectedDropTarget => 2.0,
                    },
                    color: port_border_color(port.connection_state, theme),
                }),
            },
            BoxStyle {
                width: Size::Fixed(metrics.pin_dot_diameter),
                height: Size::Fixed(metrics.pin_dot_diameter),
                hittable: Some(true),
                gestures: vec![Gesture::Tap, Gesture::Drag],
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn mount_pin_label(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    port: &NodePortSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    cx.child(
        parent,
        leaf(
            port.label_id.clone(),
            LeafKind::Text {
                content: port.name.clone(),
                style: theme.text_style_label_sm(),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Fixed(metrics.pin_label_width),
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn mount_node_body(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    body: &super::node_spec::NodeBodySpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let body_node = cx.child(
        parent,
        container(
            body.id.clone(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fill,
                flex_grow: 1.0,
                gap: metrics.row_gap,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    for row in &body.rows {
        mount_body_row(cx, body_node, row, theme, metrics)?;
    }
    Ok(())
}

fn mount_body_row(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    row: &NodeBodyRowSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    match row {
        NodeBodyRowSpec::Summary { id, text_id, text } => {
            let row = cx.child(
                parent,
                body_row(
                    id,
                    theme,
                    metrics,
                    Align::Center,
                    Size::Fixed(metrics.param_row_height),
                    0.0,
                ),
            )?;
            cx.child(
                row,
                leaf(
                    text_id.clone(),
                    LeafKind::Text {
                        content: text.clone(),
                        style: theme.text_style_label_sm(),
                        layout: ellipsis_text_layout(),
                    },
                    BoxStyle {
                        width: Size::Fill,
                        height: Size::Auto,
                        flex_shrink: 1.0,
                        ..BoxStyle::default()
                    },
                ),
            )?;
        }
        NodeBodyRowSpec::Param {
            id,
            control_id,
            control,
        } => {
            let policy = param_control_layout_policy(control, theme, metrics.control);
            let row = cx.child(
                parent,
                body_row(
                    id,
                    theme,
                    metrics,
                    policy.row_align,
                    row_height(policy, metrics),
                    row_flex(policy),
                ),
            )?;
            mount_param_control(cx, row, control_id, control, theme, metrics, policy)?;
        }
    }
    Ok(())
}

fn body_row(
    id: &str,
    theme: &Theme,
    metrics: NodeCardMetrics,
    align_items: Align,
    height: Size,
    flex_grow: f32,
) -> TreeNode {
    container(
        id.to_string(),
        BoxStyle {
            width: Size::Fill,
            height,
            min_height: match height {
                Size::Fixed(value) => value,
                Size::Fill => metrics.param_row_height,
                Size::Auto => 0.0,
            },
            flex_grow,
            direction: Direction::Row,
            gap: metrics.param_label_gap,
            align_items,
            justify_content: crate::tree::layout::Justify::Start,
            ..BoxStyle::default()
        },
        Some(Decoration {
            background: Some(theme.colors.canvas_bg),
            border: None,
            radius: [metrics.row_radius; 4],
            shadow: None,
        }),
    )
}

fn row_height(policy: ParamControlLayoutPolicy, metrics: NodeCardMetrics) -> Size {
    match policy.height {
        ParamControlHeight::Fixed(height) => Size::Fixed(metrics.param_row_height.max(height)),
        ParamControlHeight::Fill { .. } => Size::Fill,
    }
}

fn row_flex(policy: ParamControlLayoutPolicy) -> f32 {
    policy.fills_parent_height().then_some(1.0).unwrap_or(0.0)
}

fn mount_param_control(
    cx: &mut MountCx<'_>,
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
                flex_grow: row_flex(policy),
                direction: Direction::Row,
                align_items: policy.wrapper_align,
                justify_content: crate::tree::layout::Justify::Start,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let child_id = format!("{id}::widget");
    match control {
        ParamControlSpec::Text { value } => mount_text_field(
            cx,
            wrapper,
            &child_id,
            value,
            false,
            1,
            "TextInput",
            theme,
            metrics,
        )?,
        ParamControlSpec::TextArea { value, min_rows } => mount_text_field(
            cx, wrapper, &child_id, value, true, *min_rows, "TextArea", theme, metrics,
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
            "NumberInput",
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

fn mount_control_text(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    text: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    cx.child(
        parent,
        leaf(
            id.to_string(),
            LeafKind::Text {
                content: text.to_string(),
                style: theme.text_style_label_sm(),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn mount_text_field(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    value: &str,
    multiline: bool,
    min_rows: usize,
    role: &'static str,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let field_tokens = theme.text_field_metrics(metrics.control.size, metrics.control.density);
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: if multiline {
                    Size::Fill
                } else {
                    Size::Fixed(field_tokens.field_height)
                },
                min_height: if multiline {
                    min_rows.max(1) as f32 * field_tokens.value_size * 1.2
                        + field_tokens.padding_y * 2.0
                } else {
                    field_tokens.field_height
                },
                flex_grow: multiline.then_some(1.0).unwrap_or(0.0),
                ..BoxStyle::default()
            },
            None,
        )
        .with_semantic_role(role),
    )?;
    let field = cx.child(
        root,
        container(
            format!("{id}::field"),
            BoxStyle {
                width: Size::Fill,
                height: if multiline {
                    Size::Fill
                } else {
                    Size::Fixed(field_tokens.field_height)
                },
                min_height: if multiline {
                    min_rows.max(1) as f32 * field_tokens.value_size * 1.2
                        + field_tokens.padding_y * 2.0
                } else {
                    field_tokens.field_height
                },
                padding: Edges::symmetric(field_tokens.padding_y, field_tokens.padding_x),
                position: Position::relative(),
                overflow: if multiline {
                    Overflow::Scroll
                } else {
                    Overflow::Hidden
                },
                hittable: Some(true),
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: field_tokens.border_width,
                    color: theme.colors.border,
                }),
                radius: [field_tokens.radius; 4],
                shadow: None,
            }),
        ),
    )?;
    cx.child(
        field,
        container(
            format!("{id}::selection"),
            BoxStyle {
                position: Position::absolute_inset(crate::tree::layout::Inset::ZERO),
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    cx.child(
        field,
        leaf(
            format!("{id}::value"),
            LeafKind::Text {
                content: value.to_string(),
                style: crate::renderer::TextStyle::new(theme.colors.text, field_tokens.value_size)
                    .with_family(theme.text.body_family)
                    .with_line_height(theme.text.default_line_height),
                layout: if multiline {
                    TextLayout::default()
                } else {
                    ellipsis_text_layout()
                },
            },
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
        ),
    )?;
    cx.child(
        field,
        container(
            format!("{id}::caret"),
            BoxStyle {
                position: Position::absolute_xy(0.0, 0.0),
                width: Size::Fixed(1.5),
                height: Size::Fixed(field_tokens.value_size * 1.2),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.caret),
                border: None,
                radius: [1.0; 4],
                shadow: None,
            }),
        ),
    )?;
    Ok(())
}

fn mount_slider(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    value: f32,
    min: f32,
    max: f32,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(metrics.control.control_height),
                direction: Direction::Row,
                align_items: Align::Center,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let track = cx.child(
        root,
        container(
            format!("{id}::track"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(4.0),
                position: Position::relative(),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.border),
                border: None,
                radius: [2.0; 4],
                shadow: None,
            }),
        ),
    )?;
    let range = (max - min).abs().max(f32::EPSILON);
    let t = ((value - min) / range).clamp(0.0, 1.0);
    cx.child(
        track,
        container(
            format!("{id}::thumb"),
            BoxStyle {
                position: Position::absolute_xy(t * (metrics.control.control_width - 10.0), -3.0),
                width: Size::Fixed(10.0),
                height: Size::Fixed(10.0),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.accent),
                border: None,
                radius: [5.0; 4],
                shadow: None,
            }),
        ),
    )?;
    Ok(())
}

fn mount_toggle(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    checked: bool,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let width = metrics.control.control_height * 1.65;
    let height = metrics.control.control_height * 0.72;
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fixed(width),
                height: Size::Fixed(height),
                position: Position::relative(),
                hittable: Some(true),
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(if checked {
                    theme.colors.accent
                } else {
                    theme.colors.border
                }),
                border: None,
                radius: [height * 0.5; 4],
                shadow: None,
            }),
        ),
    )?;
    let knob = height - 4.0;
    cx.child(
        root,
        container(
            format!("{id}::knob"),
            BoxStyle {
                position: Position::absolute_xy(
                    if checked { width - knob - 2.0 } else { 2.0 },
                    2.0,
                ),
                width: Size::Fixed(knob),
                height: Size::Fixed(knob),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: None,
                radius: [knob * 0.5; 4],
                shadow: None,
            }),
        ),
    )?;
    Ok(())
}

fn mount_color_control(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    id: &str,
    rgba: [f32; 4],
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(metrics.control.control_height),
                direction: Direction::Row,
                gap: metrics.control.control_height * 0.25,
                align_items: Align::Center,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    cx.child(
        root,
        container(
            format!("{id}::swatch"),
            BoxStyle {
                width: Size::Fixed(metrics.control.control_height * 0.75),
                height: Size::Fixed(metrics.control.control_height * 0.75),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(Color {
                    r: rgba[0],
                    g: rgba[1],
                    b: rgba[2],
                    a: rgba[3],
                }),
                border: Some(Border {
                    width: 1.0,
                    color: theme.colors.border,
                }),
                radius: [theme.radii.sm; 4],
                shadow: None,
            }),
        ),
    )?;
    mount_control_text(
        cx,
        root,
        &format!("{id}::value"),
        &format!(
            "#{:02X}{:02X}{:02X}",
            (rgba[0].clamp(0.0, 1.0) * 255.0) as u8,
            (rgba[1].clamp(0.0, 1.0) * 255.0) as u8,
            (rgba[2].clamp(0.0, 1.0) * 255.0) as u8
        ),
        theme,
    )
}

fn mount_node_header(
    cx: &mut MountCx<'_>,
    parent: NodeId,
    header: &NodeHeaderSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let row = cx.child(
        parent,
        container(
            header.row_id.clone(),
            BoxStyle {
                position: Position::absolute_xy(0.0, -metrics.title_lift),
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Row,
                gap: metrics.title_label_gap,
                align_items: Align::Center,
                hittable: Some(false),
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    cx.child(
        row,
        leaf(
            header.dot_id.clone(),
            LeafKind::Circle {
                radius: metrics.title_dot_diameter * 0.5,
                fill: Some(header.category_color),
                stroke: None,
            },
            BoxStyle {
                width: Size::Fixed(metrics.title_dot_diameter),
                height: Size::Fixed(metrics.title_dot_diameter),
                ..BoxStyle::default()
            },
        ),
    )?;
    cx.child(
        row,
        leaf(
            header.text_id.clone(),
            LeafKind::Text {
                content: header.title.clone(),
                style: theme.text_style_label_sm(),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Fixed(metrics.title_label_width),
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn ellipsis_text_layout() -> TextLayout {
    TextLayout {
        overflow: TextOverflow::Ellipsis,
        ..TextLayout::default()
    }
}

fn port_state_color(port: &NodePortSpec, theme: &Theme) -> Color {
    match port.connection_state {
        CanvasPortConnectionState::CompatibleTarget => theme.colors.accent,
        CanvasPortConnectionState::DropTarget => theme.colors.accent,
        CanvasPortConnectionState::IncompatibleTarget => incompatible_port_color(),
        CanvasPortConnectionState::RejectedDropTarget => incompatible_port_color(),
        CanvasPortConnectionState::Source | CanvasPortConnectionState::Idle => {
            port_color(port.side, theme)
        }
    }
}

fn port_border_color(state: CanvasPortConnectionState, theme: &Theme) -> Color {
    match state {
        CanvasPortConnectionState::CompatibleTarget => theme.colors.accent,
        CanvasPortConnectionState::DropTarget => theme.colors.accent,
        CanvasPortConnectionState::IncompatibleTarget => incompatible_port_color(),
        CanvasPortConnectionState::RejectedDropTarget => incompatible_port_color(),
        CanvasPortConnectionState::Source => theme.colors.text,
        CanvasPortConnectionState::Idle => theme.colors.surface,
    }
}

fn incompatible_port_color() -> Color {
    Color {
        r: 0.863,
        g: 0.149,
        b: 0.149,
        a: 1.0,
    }
}

fn port_color(side: CanvasPortSide, theme: &Theme) -> Color {
    match side {
        CanvasPortSide::Input => theme.colors.text_muted,
        CanvasPortSide::Output => theme.colors.accent,
    }
}

fn container(id: String, style: BoxStyle, decoration: Option<Decoration>) -> TreeNode {
    TreeNode {
        id: StableId::from(id),
        props: NodeProps::default(),
        style,
        decoration,
        kind: NodeKind::Container,
        rect: zero_rect(),
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: NodeLayoutMeta::default(),
        paint_meta: NodePaintMeta::default(),
        mutation_meta: NodeMutationMeta::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

fn leaf(id: String, kind: LeafKind, style: BoxStyle) -> TreeNode {
    TreeNode {
        id: StableId::from(id),
        props: NodeProps::default(),
        style,
        decoration: None,
        kind: NodeKind::Leaf(kind),
        rect: zero_rect(),
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: NodeLayoutMeta::default(),
        paint_meta: NodePaintMeta::default(),
        mutation_meta: NodeMutationMeta::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

trait TreeNodeExt {
    fn with_semantic_role(self, role: &'static str) -> Self;
    fn with_layout_boundary(self, reason: RelayoutBoundaryReason) -> Self;
    fn with_paint_boundary(self, reason: RepaintBoundaryReason) -> Self;
    fn with_rect_move_invalidation(self, invalidation: RectMoveInvalidation) -> Self;
}

impl TreeNodeExt for TreeNode {
    fn with_semantic_role(mut self, role: &'static str) -> Self {
        self.props.semantic_role = Some(Cow::Borrowed(role));
        self
    }

    fn with_layout_boundary(mut self, reason: RelayoutBoundaryReason) -> Self {
        self.layout_meta.set_boundary(reason);
        self
    }

    fn with_paint_boundary(mut self, reason: RepaintBoundaryReason) -> Self {
        self.paint_meta.set_boundary(reason);
        self
    }

    fn with_rect_move_invalidation(mut self, invalidation: RectMoveInvalidation) -> Self {
        self.mutation_meta.rect_move = invalidation;
        self
    }
}

fn zero_rect() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    }
}
