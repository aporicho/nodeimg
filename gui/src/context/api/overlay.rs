use super::super::Context;
use crate::overlay::OverlayRequest;

pub struct OverlayApi<'a> {
    pub(in crate::context) ctx: &'a Context,
}

impl OverlayApi<'_> {
    pub fn is_open(&self) -> bool {
        self.ctx.overlay_open()
    }
}

pub struct OverlayMutApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl OverlayMutApi<'_> {
    pub fn open(&mut self, request: OverlayRequest) {
        self.ctx.open_overlay(request);
    }

    pub fn close(&mut self) {
        self.ctx.close_overlay();
    }
}
