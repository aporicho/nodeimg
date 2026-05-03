use super::system::TextBoxSystem;
use crate::control::SystemCx;
use crate::output::FrameworkOutput;
use crate::shell::{AppEvent, MouseButton};

impl TextBoxSystem {
    pub(crate) fn handle_event(
        &mut self,
        mut cx: SystemCx<'_>,
        event: &AppEvent,
    ) -> FrameworkOutput {
        let outcome = match event {
            AppEvent::MousePress { x, y, button } if *button == MouseButton::Left => {
                if let Some(output) = self.handle_mouse_press(&cx, *x, *y) {
                    return output;
                }
                FrameworkOutput::default()
            }
            AppEvent::MouseMove { x, y } => {
                if let Some(output) = self.handle_mouse_move(&cx, *x, *y) {
                    return output;
                }
                FrameworkOutput::default()
            }
            AppEvent::MouseRelease { button, .. } if *button == MouseButton::Left => {
                self.handle_mouse_release()
            }
            AppEvent::ImePreedit { text, caret } => self.handle_ime_preedit(&cx, text, *caret),
            AppEvent::TextInput { text } => self.commit_text_input(&cx, text),
            AppEvent::KeyPress { key, modifiers } => {
                self.handle_key_press(&mut cx, *key, *modifiers)
            }
            AppEvent::Unfocused => {
                self.active_drag_text_box = None;
                FrameworkOutput::default()
            }
            _ => FrameworkOutput::default(),
        };

        self.sync_sessions_for(&cx);
        outcome
    }
}
