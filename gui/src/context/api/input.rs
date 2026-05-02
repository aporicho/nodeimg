use super::super::{Context, ImeRequest};
use crate::output::FrameworkOutput;
use crate::shell::AppEvent;
use crate::tree::NodeId;

pub struct InputApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl InputApi<'_> {
    pub fn handle_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        self.ctx.handle_event(event)
    }

    pub fn ime_request(&self) -> ImeRequest {
        self.ctx.ime_request()
    }

    pub fn paste_focused_text(&mut self, text: &str) -> FrameworkOutput {
        self.ctx.paste_focused_text(text)
    }

    pub fn request_focus(&mut self, node_id: NodeId) {
        self.ctx.request_focus(node_id);
    }
}
