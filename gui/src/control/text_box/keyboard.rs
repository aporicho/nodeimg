use super::system::TextBoxSystem;
use crate::control::SystemCx;
use crate::output::{FrameworkOutput, OutputBuilder, PlatformEffect};
use crate::shell::{Key, Modifiers};

impl TextBoxSystem {
    pub(super) fn handle_key_press(
        &mut self,
        cx: &mut SystemCx<'_>,
        key: Key,
        modifiers: Modifiers,
    ) -> FrameworkOutput {
        if key == Key::Escape {
            if let Some(control_id) = self.focused_control_id_for(cx) {
                if let Some(runtime) = self.store.text_box_mut(&control_id) {
                    if runtime.has_preedit() {
                        runtime.clear_preedit();
                        return FrameworkOutput::consumed();
                    }
                    if runtime.is_number() {
                        runtime.revert_to_external();
                    }
                }
            }
            cx.blur();
            self.active_drag_text_box = None;
            return FrameworkOutput::consumed();
        }

        let Some(control_id) = self.focused_control_id_for(cx) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.text_box_mut(&control_id) else {
            return FrameworkOutput::default();
        };

        if runtime.has_preedit() {
            match key {
                Key::Char('A' | 'C' | 'X' | 'V') if modifiers.ctrl || modifiers.meta => {
                    runtime.clear_preedit();
                }
                _ => return FrameworkOutput::default(),
            }
        }

        match key {
            Key::Enter => {
                if runtime.is_multiline() {
                    runtime.editor_mut().insert_char('\n');
                    return self
                        .output_for_editor_for(cx, &control_id)
                        .with_consumed(true);
                }
                self.finalize_number_input(&control_id)
            }
            Key::Backspace => {
                runtime.editor_mut().backspace();
                self.output_for_editor_for(cx, &control_id)
                    .with_consumed(true)
            }
            Key::Delete => {
                runtime.editor_mut().delete();
                self.output_for_editor_for(cx, &control_id)
                    .with_consumed(true)
            }
            Key::Left => {
                runtime.move_left(modifiers.shift);
                FrameworkOutput::consumed()
            }
            Key::Right => {
                runtime.move_right(modifiers.shift);
                FrameworkOutput::consumed()
            }
            Key::Up => {
                if runtime.is_number() {
                    self.step_number_input(&control_id, 1.0)
                } else if runtime.is_multiline() {
                    runtime.move_vertical(-1, modifiers.shift);
                    FrameworkOutput::consumed()
                } else {
                    FrameworkOutput::default()
                }
            }
            Key::Down => {
                if runtime.is_number() {
                    self.step_number_input(&control_id, -1.0)
                } else if runtime.is_multiline() {
                    runtime.move_vertical(1, modifiers.shift);
                    FrameworkOutput::consumed()
                } else {
                    FrameworkOutput::default()
                }
            }
            Key::Home => {
                runtime.move_home(modifiers.shift);
                FrameworkOutput::consumed()
            }
            Key::End => {
                runtime.move_end(modifiers.shift);
                FrameworkOutput::consumed()
            }
            Key::Char('A') if modifiers.ctrl || modifiers.meta => {
                runtime.editor_mut().select_all();
                FrameworkOutput::consumed()
            }
            Key::Char('C') if modifiers.ctrl || modifiers.meta => runtime
                .editor()
                .copy()
                .map(|text| {
                    OutputBuilder::new()
                        .effect(PlatformEffect::WriteClipboard(text))
                        .finish()
                })
                .unwrap_or_default(),
            Key::Char('X') if modifiers.ctrl || modifiers.meta => {
                let effects = runtime
                    .editor_mut()
                    .cut()
                    .map(|text| vec![PlatformEffect::WriteClipboard(text)])
                    .unwrap_or_default();
                let mut output = self.output_for_editor_for(cx, &control_id);
                output.effects = effects;
                output.with_consumed(true)
            }
            Key::Char('V') if modifiers.ctrl || modifiers.meta => OutputBuilder::new()
                .effect(PlatformEffect::RequestClipboardPaste)
                .finish(),
            _ => FrameworkOutput::default(),
        }
    }
}
