use std::cell::Cell;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::renderer::Rect;
use crate::tree::NodeId;

pub const TARGET_RENDER: &str = "nodeimg::render_trace";
pub const TARGET_RENDER_NODE: &str = "nodeimg::render_trace::node";
pub const TARGET_RENDER_GPU: &str = "nodeimg::render_trace::gpu";

static NEXT_FRAME_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static CURRENT_FRAME_ID: Cell<u64> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RenderTraceFrame {
    pub id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderTraceStage {
    RedrawRequested,
    AppUpdate,
    SceneSync,
    TreeMutation,
    DirtyPropagation,
    LayoutFlush,
    TextRuntimeSync,
    PaintFlush,
    PaintFragment,
    DisplayListLowering,
    RendererPrepare,
    RendererDispatch,
    Present,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectSummary {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextPayloadSummary {
    pub bytes: usize,
    pub chars: usize,
}

impl TextPayloadSummary {
    pub fn new(text: &str) -> Self {
        Self {
            bytes: text.len(),
            chars: text.chars().count(),
        }
    }
}

impl From<Rect> for RectSummary {
    fn from(value: Rect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            w: value.w,
            h: value.h,
        }
    }
}

pub fn next_render_trace_frame() -> RenderTraceFrame {
    let frame = RenderTraceFrame {
        id: NEXT_FRAME_ID.fetch_add(1, Ordering::Relaxed),
    };
    set_current_render_trace_frame(frame);
    frame
}

pub fn set_current_render_trace_frame(frame: RenderTraceFrame) {
    CURRENT_FRAME_ID.with(|current| current.set(frame.id));
}

pub fn clear_current_render_trace_frame() {
    CURRENT_FRAME_ID.with(|current| current.set(0));
}

pub fn current_render_trace_frame() -> RenderTraceFrame {
    CURRENT_FRAME_ID.with(|current| RenderTraceFrame { id: current.get() })
}

pub fn is_debug_enabled() -> bool {
    tracing::enabled!(target: TARGET_RENDER, tracing::Level::DEBUG)
}

pub fn is_trace_enabled() -> bool {
    tracing::enabled!(target: TARGET_RENDER, tracing::Level::TRACE)
}

pub fn is_node_trace_enabled() -> bool {
    tracing::enabled!(target: TARGET_RENDER_NODE, tracing::Level::TRACE)
}

pub fn is_gpu_debug_enabled() -> bool {
    tracing::enabled!(target: TARGET_RENDER_GPU, tracing::Level::DEBUG)
}

pub fn is_gpu_trace_enabled() -> bool {
    tracing::enabled!(target: TARGET_RENDER_GPU, tracing::Level::TRACE)
}

pub fn debug_stage(summary_stage: RenderTraceStage, summary: impl fmt::Debug) {
    if is_debug_enabled() {
        let frame = current_render_trace_frame();
        tracing::debug!(
            target: TARGET_RENDER,
            frame_id = frame.id,
            stage = ?summary_stage,
            summary = ?summary,
            "render trace stage"
        );
    }
}

pub fn trace_stage(summary_stage: RenderTraceStage, summary: impl fmt::Debug) {
    if is_trace_enabled() {
        let frame = current_render_trace_frame();
        tracing::trace!(
            target: TARGET_RENDER,
            frame_id = frame.id,
            stage = ?summary_stage,
            summary = ?summary,
            "render trace stage"
        );
    }
}

pub fn trace_node(
    stage: RenderTraceStage,
    node: NodeId,
    stable_id: Option<&str>,
    summary: impl fmt::Debug,
) {
    if is_node_trace_enabled() {
        let frame = current_render_trace_frame();
        tracing::trace!(
            target: TARGET_RENDER_NODE,
            frame_id = frame.id,
            stage = ?stage,
            node,
            stable_id,
            summary = ?summary,
            "render trace node"
        );
    }
}

pub fn debug_gpu(stage: RenderTraceStage, summary: impl fmt::Debug) {
    if is_gpu_debug_enabled() {
        let frame = current_render_trace_frame();
        tracing::debug!(
            target: TARGET_RENDER_GPU,
            frame_id = frame.id,
            stage = ?stage,
            summary = ?summary,
            "render trace gpu"
        );
    }
}

pub fn trace_gpu(stage: RenderTraceStage, summary: impl fmt::Debug) {
    if is_gpu_trace_enabled() {
        let frame = current_render_trace_frame();
        tracing::trace!(
            target: TARGET_RENDER_GPU,
            frame_id = frame.id,
            stage = ?stage,
            summary = ?summary,
            "render trace gpu"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_trace_frame_ids_are_monotonic() {
        clear_current_render_trace_frame();
        let first = next_render_trace_frame();
        let second = next_render_trace_frame();

        assert!(second.id > first.id);
        assert_eq!(current_render_trace_frame(), second);
    }

    #[test]
    fn render_trace_redacts_text_payloads() {
        let summary = TextPayloadSummary::new("hello 世界");

        assert_eq!(summary.bytes, "hello 世界".len());
        assert_eq!(summary.chars, 8);
        assert!(!format!("{summary:?}").contains("hello"));
    }
}
