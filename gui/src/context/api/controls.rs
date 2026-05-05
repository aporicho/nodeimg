use super::super::Context;
use crate::control::{ControlIntrinsic, ControlTextBoxSyncItem};
use crate::renderer::TextMeasurer;
use crate::theme::Theme;

pub struct ControlsApi<'a> {
    pub(in crate::context) ctx: &'a Context,
}

impl ControlsApi<'_> {
    pub fn intrinsics(&self) -> Vec<ControlIntrinsic> {
        self.ctx.control_intrinsics()
    }

    pub fn has_dirty_intrinsics(&self) -> bool {
        self.ctx.has_dirty_control_intrinsics()
    }
}

pub struct ControlsMutApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl ControlsMutApi<'_> {
    pub fn take_dirty_intrinsics(&mut self) -> Vec<ControlIntrinsic> {
        self.ctx.take_dirty_control_intrinsics()
    }

    pub fn sync_text_boxes(
        &mut self,
        items: &[ControlTextBoxSyncItem<'_>],
        measurer: &mut TextMeasurer,
        theme: &Theme,
    ) {
        self.ctx.sync_text_boxes(items, measurer, theme);
    }
}
