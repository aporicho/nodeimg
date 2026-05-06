use super::select::SelectRuntime;
use super::slider::SliderRuntime;
use super::toggle::ToggleRuntime;
use crate::control::SystemCx;
use crate::runtime::RuntimeEventResult;
use crate::shell::{AppEvent, MouseButton};

#[derive(Default)]
pub(crate) struct ControlInteractionSystem {
    select: SelectRuntime,
    slider: SliderRuntime,
    toggle: ToggleRuntime,
}

impl ControlInteractionSystem {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn handle_pre_gesture_event(
        &mut self,
        cx: SystemCx<'_>,
        event: &AppEvent,
    ) -> RuntimeEventResult {
        match *event {
            AppEvent::MousePress {
                x,
                y,
                button: MouseButton::Left,
            } => self
                .slider
                .handle_mouse_press(&cx, x, y)
                .or_else(|| self.select.handle_mouse_press(&cx, x, y))
                .or_else(|| self.toggle.handle_mouse_press(&cx, x, y))
                .unwrap_or_default(),
            AppEvent::MouseMove { x, y } => {
                self.slider.handle_mouse_move(&cx, x, y).unwrap_or_default()
            }
            AppEvent::MouseRelease {
                x,
                y,
                button: MouseButton::Left,
            } => self
                .slider
                .handle_mouse_release(&cx, x, y)
                .or_else(|| self.select.handle_mouse_release(&cx, x, y))
                .or_else(|| self.toggle.handle_mouse_release(&cx, x, y))
                .unwrap_or_default(),
            AppEvent::Unfocused => {
                self.select.clear();
                self.slider.clear();
                self.toggle.clear();
                RuntimeEventResult::default()
            }
            _ => RuntimeEventResult::default(),
        }
    }
}
