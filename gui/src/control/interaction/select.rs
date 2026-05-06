use super::spec::ControlInteractionSpec;
use super::target;
use crate::control::{ControlValue, SystemCx};
use crate::output::{ControlEvent, OutputBuilder};
use crate::runtime::RuntimeEventResult;

#[derive(Default)]
pub(super) struct SelectRuntime {
    active_id: Option<String>,
}

impl SelectRuntime {
    pub(super) fn handle_mouse_press(
        &mut self,
        cx: &SystemCx<'_>,
        x: f32,
        y: f32,
    ) -> Option<RuntimeEventResult> {
        let hit = target::target_at(cx, x, y)?;
        if !matches!(hit.spec, ControlInteractionSpec::Select { .. }) {
            return None;
        }
        self.active_id = Some(hit.id);
        Some(consumed())
    }

    pub(super) fn handle_mouse_release(
        &mut self,
        cx: &SystemCx<'_>,
        x: f32,
        y: f32,
    ) -> Option<RuntimeEventResult> {
        let active_id = self.active_id.take()?;
        let Some(hit) = target::target_at(cx, x, y) else {
            return Some(consumed());
        };
        if hit.id != active_id {
            return Some(consumed());
        }
        let ControlInteractionSpec::Select {
            selected,
            options_len,
        } = hit.spec
        else {
            return Some(consumed());
        };
        if options_len == 0 {
            return Some(consumed());
        }
        let next = (selected + 1) % options_len;
        Some(RuntimeEventResult {
            output: OutputBuilder::new()
                .control(ControlEvent::ValueChanged {
                    id: active_id,
                    value: ControlValue::Selection(next),
                })
                .finish(),
            cancel_gesture: true,
        })
    }

    pub(super) fn clear(&mut self) {
        self.active_id = None;
    }
}

fn consumed() -> RuntimeEventResult {
    RuntimeEventResult {
        output: crate::output::FrameworkOutput::consumed(),
        cancel_gesture: true,
    }
}
