use super::model::TextBoxSpec;
use super::runtime::TextBoxRuntime;
use crate::control::{ControlIntrinsic, TextBoxMode};
use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::renderer::Rect;

pub(super) fn log_text_box_sizing(
    control_id: &str,
    runtime: &TextBoxRuntime,
    spec: &TextBoxSpec,
    focused: bool,
    external_text_changed: bool,
    value_rect: Option<Rect>,
) {
    let TextBoxMode::MultiLine { min_rows } = spec.mode else {
        return;
    };

    let line_count = runtime.layout.lines.len();
    let height_delta = runtime.desired_height - runtime.field_rect.h;
    let editor_dirty = runtime.editor.text() != runtime.last_external_text;
    let should_debug = external_text_changed || editor_dirty || height_delta.abs() > 0.5;
    if should_debug {
        if !render_trace::is_debug_enabled() {
            return;
        }
    } else if !render_trace::is_trace_enabled() {
        return;
    }
    let summary = TextBoxSizingTraceSummary {
        control_id,
        focused,
        external_text_changed,
        editor_dirty,
        text_bytes: runtime.editor.text().len(),
        text_chars: runtime.editor.text().chars().count(),
        min_rows,
        line_count,
        line_height: runtime.layout.line_height,
        layout_w: runtime.layout.width,
        layout_h: runtime.layout.height,
        field_w: runtime.field_rect.w,
        field_h: runtime.field_rect.h,
        content_w: runtime.content_rect.w,
        content_h: runtime.content_rect.h,
        min_h: runtime.min_height,
        desired_h: runtime.desired_height,
        height_delta,
        value_rect_w: value_rect.map(|rect| rect.w),
        value_rect_h: value_rect.map(|rect| rect.h),
    };

    if should_debug {
        render_trace::debug_stage(RenderTraceStage::TextRuntimeSync, summary);
    } else {
        render_trace::trace_stage(RenderTraceStage::TextRuntimeSync, summary);
    }
}

pub(super) fn log_control_intrinsic(intrinsic: &ControlIntrinsic) {
    let height_delta = intrinsic.desired_size[1] - intrinsic.current_size[1];
    let log_at_debug = height_delta.abs() > 0.5;
    if log_at_debug {
        if !render_trace::is_debug_enabled() {
            return;
        }
    } else if !render_trace::is_trace_enabled() {
        return;
    }
    let summary = ControlIntrinsicTraceSummary {
        control_id: intrinsic.control_id.as_str(),
        current_w: intrinsic.current_size[0],
        current_h: intrinsic.current_size[1],
        min_w: intrinsic.min_size[0],
        min_h: intrinsic.min_size[1],
        desired_w: intrinsic.desired_size[0],
        desired_h: intrinsic.desired_size[1],
        height_delta,
        affects_parent_height: intrinsic.affects_parent_height,
    };
    if log_at_debug {
        render_trace::debug_stage(RenderTraceStage::TextRuntimeSync, summary);
    } else {
        render_trace::trace_stage(RenderTraceStage::TextRuntimeSync, summary);
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct TextBoxSizingTraceSummary<'a> {
    control_id: &'a str,
    focused: bool,
    external_text_changed: bool,
    editor_dirty: bool,
    text_bytes: usize,
    text_chars: usize,
    min_rows: usize,
    line_count: usize,
    line_height: f32,
    layout_w: f32,
    layout_h: f32,
    field_w: f32,
    field_h: f32,
    content_w: f32,
    content_h: f32,
    min_h: f32,
    desired_h: f32,
    height_delta: f32,
    value_rect_w: Option<f32>,
    value_rect_h: Option<f32>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct ControlIntrinsicTraceSummary<'a> {
    control_id: &'a str,
    current_w: f32,
    current_h: f32,
    min_w: f32,
    min_h: f32,
    desired_w: f32,
    desired_h: f32,
    height_delta: f32,
    affects_parent_height: bool,
}
