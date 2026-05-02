use crate::action::GuiAction;
use crate::control::ResizeEdge;

#[derive(Debug, Clone)]
pub enum PlatformEffect {
    WriteClipboard(String),
    RequestClipboardPaste,
}

#[derive(Debug, Clone)]
pub enum GuiEvent {
    Control(ControlEvent),
    Overlay(OverlayEvent),
}

#[derive(Debug, Clone)]
pub enum ControlEvent {
    Click {
        id: String,
    },
    DoubleClick {
        id: String,
    },
    LongPress {
        id: String,
    },
    TextChanged {
        id: String,
        value: String,
    },
    NumberChanged {
        id: String,
        value: f32,
    },
    SelectionChanged {
        id: String,
        selected: usize,
    },
    DragStart {
        id: String,
        x: f32,
        y: f32,
    },
    DragMove {
        id: String,
        x: f32,
        y: f32,
    },
    DragEnd {
        id: String,
        x: f32,
        y: f32,
    },
    ResizeStart {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
    ResizeMove {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
    ResizeEnd {
        id: String,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    },
}

#[derive(Debug, Clone)]
pub enum OverlayEvent {
    Opened { id: String },
    Closed { id: String },
}

#[derive(Debug, Default)]
pub struct FrameworkOutput {
    pub events: Vec<GuiEvent>,
    pub actions: Vec<GuiAction>,
    pub effects: Vec<PlatformEffect>,
    pub consumed: bool,
}

impl FrameworkOutput {
    pub(crate) fn consumed() -> Self {
        Self {
            consumed: true,
            ..Self::default()
        }
    }

    pub(crate) fn with_consumed(mut self, consumed: bool) -> Self {
        self.consumed |= consumed;
        self
    }

    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.events.extend(other.events);
        self.actions.extend(other.actions);
        self.effects.extend(other.effects);
        self.consumed |= other.consumed;
        self
    }
}

#[derive(Debug, Default)]
pub(crate) struct OutputBuilder {
    output: FrameworkOutput,
}

impl OutputBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn event(mut self, event: GuiEvent) -> Self {
        self.output.events.push(event);
        self
    }

    pub(crate) fn control(self, event: ControlEvent) -> Self {
        self.event(GuiEvent::Control(event))
    }

    pub(crate) fn action(mut self, action: GuiAction) -> Self {
        self.output.actions.push(action);
        self
    }

    pub(crate) fn effect(mut self, effect: PlatformEffect) -> Self {
        self.output.effects.push(effect);
        self
    }

    pub(crate) fn finish(mut self) -> FrameworkOutput {
        self.output.consumed |= !self.output.events.is_empty()
            || !self.output.actions.is_empty()
            || !self.output.effects.is_empty();
        self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_preserves_actions() {
        let left = OutputBuilder::new()
            .action(GuiAction::ControlClicked {
                id: "left".to_string(),
            })
            .finish();
        let right = OutputBuilder::new()
            .action(GuiAction::ControlClicked {
                id: "right".to_string(),
            })
            .finish();

        let merged = left.merge(right);

        assert_eq!(
            merged.actions,
            vec![
                GuiAction::ControlClicked {
                    id: "left".to_string()
                },
                GuiAction::ControlClicked {
                    id: "right".to_string()
                }
            ]
        );
        assert!(merged.consumed);
    }
}
