use super::spec::ControlInteractionSpec;
use super::target;
use crate::control::{ControlValue, SystemCx};
use crate::output::{ControlEvent, OutputBuilder};
use crate::runtime::RuntimeEventResult;
use crate::tree::NodeId;

const VALUE_EPSILON: f32 = 0.000_001;

#[derive(Default)]
pub(super) struct SliderRuntime {
    active: Option<ActiveSlider>,
}

struct ActiveSlider {
    id: String,
    last_value: f32,
}

impl SliderRuntime {
    pub(super) fn handle_mouse_press(
        &mut self,
        cx: &SystemCx<'_>,
        x: f32,
        y: f32,
    ) -> Option<RuntimeEventResult> {
        let hit = target::target_at(cx, x, y)?;
        let ControlInteractionSpec::Slider { .. } = hit.spec else {
            return None;
        };
        let next_value = value_from_pointer(cx, hit.node_id, x, y, hit.spec)?;
        self.active = Some(ActiveSlider {
            id: hit.id.clone(),
            last_value: next_value,
        });
        Some(value_changed(hit.id, next_value))
    }

    pub(super) fn handle_mouse_move(
        &mut self,
        cx: &SystemCx<'_>,
        x: f32,
        y: f32,
    ) -> Option<RuntimeEventResult> {
        let active_id = self.active.as_ref()?.id.clone();
        let hit = target::target_by_id(cx, &active_id)?;
        let next_value = value_from_pointer(cx, hit.node_id, x, y, hit.spec)?;
        let active = self.active.as_mut()?;
        if values_match(active.last_value, next_value) {
            return Some(consumed());
        }
        active.last_value = next_value;
        Some(value_changed(active_id, next_value))
    }

    pub(super) fn handle_mouse_release(
        &mut self,
        cx: &SystemCx<'_>,
        x: f32,
        y: f32,
    ) -> Option<RuntimeEventResult> {
        let active = self.active.take()?;
        let Some(hit) = target::target_by_id(cx, &active.id) else {
            return Some(consumed());
        };
        let Some(next_value) = value_from_pointer(cx, hit.node_id, x, y, hit.spec) else {
            return Some(consumed());
        };
        if values_match(active.last_value, next_value) {
            return Some(consumed());
        }
        Some(value_changed(active.id, next_value))
    }

    pub(super) fn clear(&mut self) {
        self.active = None;
    }
}

pub(super) fn value_from_pointer(
    cx: &SystemCx<'_>,
    node_id: NodeId,
    x: f32,
    y: f32,
    spec: ControlInteractionSpec,
) -> Option<f32> {
    let ControlInteractionSpec::Slider {
        value,
        min,
        max,
        step,
    } = spec
    else {
        return None;
    };
    let node = cx.tree().get(node_id)?;
    let width = node.rect.w;
    if !width.is_finite() || width <= 0.0 {
        return Some(clamp_value(value, min, max));
    }
    let point = cx.screen_to_node_layout_point(node_id, x, y)?;
    let ratio = ((point.x - node.rect.x) / width).clamp(0.0, 1.0);
    let raw = min + (max - min) * ratio;
    Some(snap_value(clamp_value(raw, min, max), min, max, step))
}

fn value_changed(id: String, value: f32) -> RuntimeEventResult {
    RuntimeEventResult {
        output: OutputBuilder::new()
            .control(ControlEvent::ValueChanged {
                id,
                value: ControlValue::Number(value),
            })
            .finish(),
        cancel_gesture: true,
    }
}

fn consumed() -> RuntimeEventResult {
    RuntimeEventResult {
        output: crate::output::FrameworkOutput::consumed(),
        cancel_gesture: true,
    }
}

fn values_match(left: f32, right: f32) -> bool {
    (left - right).abs() <= VALUE_EPSILON
}

fn clamp_value(value: f32, min: f32, max: f32) -> f32 {
    if !value.is_finite() {
        return finite_or_zero(min);
    }
    if !min.is_finite() || !max.is_finite() {
        return value;
    }
    let lo = min.min(max);
    let hi = min.max(max);
    value.clamp(lo, hi)
}

fn snap_value(value: f32, min: f32, max: f32, step: f32) -> f32 {
    if !step.is_finite() || step <= 0.0 || !min.is_finite() {
        return clamp_value(value, min, max);
    }
    let snapped = min + ((value - min) / step).round() * step;
    clamp_value(snapped, min, max)
}

fn finite_or_zero(value: f32) -> f32 {
    value.is_finite().then_some(value).unwrap_or(0.0)
}
