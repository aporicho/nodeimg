use crate::context::ImeRequest;
use crate::output::{FrameworkOutput, OutputBuilder, PlatformEffect, WidgetEvent};
use crate::renderer::TextMeasurer;
use crate::shell::{AppEvent, Key, Modifiers, MouseButton};
use crate::theme::Theme;
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::number_input::{format_number, NumberInputProps};
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::state::{TextFieldKind, TextInputStore};
use crate::widget::systems::SystemCx;

pub struct TextInputSystem {
    store: TextInputStore,
    active_drag_text_input: Option<String>,
}

impl TextInputSystem {
    pub fn new() -> Self {
        Self {
            store: TextInputStore::new(),
            active_drag_text_input: None,
        }
    }

    pub fn sync_with_tree(
        &mut self,
        tree: &Tree,
        measurer: &mut TextMeasurer,
        theme: &Theme,
        focused: Option<NodeId>,
        captured: Option<NodeId>,
    ) {
        let focused_widget_id = self.focused_widget_id(tree, focused);
        self.store
            .sync_with_tree(tree, measurer, theme, focused_widget_id.as_deref());
        self.sync_sessions(tree, focused, captured);
    }

    pub fn handle_event(&mut self, mut cx: SystemCx<'_>, event: &AppEvent) -> FrameworkOutput {
        let outcome = match event {
            AppEvent::MousePress { x, y, button } if *button == MouseButton::Left => {
                self.active_drag_text_input = None;
                if let Some(widget_id) = self.text_input_id_at(&cx, *x, *y) {
                    if let Some(runtime) = self.store.runtime_mut(&widget_id) {
                        runtime.clear_preedit();
                        runtime.set_caret_from_x(*x);
                        self.active_drag_text_input = Some(widget_id);
                        return FrameworkOutput::consumed();
                    }
                }
                FrameworkOutput::default()
            }
            AppEvent::MouseMove { x, .. } => {
                if let Some(widget_id) = self.active_drag_text_input.clone() {
                    if self.captured_widget_id_for(&cx).as_deref() == Some(widget_id.as_str()) {
                        if let Some(runtime) = self.store.runtime_mut(&widget_id) {
                            runtime.select_to_x(*x);
                            return FrameworkOutput::consumed();
                        }
                    }
                }
                FrameworkOutput::default()
            }
            AppEvent::MouseRelease { button, .. } if *button == MouseButton::Left => {
                let consumed = self.active_drag_text_input.is_some();
                self.active_drag_text_input = None;
                FrameworkOutput::default().with_consumed(consumed)
            }
            AppEvent::ImePreedit { text, caret } => self.handle_ime_preedit(&cx, text, *caret),
            AppEvent::TextInput { text } => self.commit_text_input(&cx, text),
            AppEvent::KeyPress { key, modifiers } => {
                self.handle_key_press(&mut cx, *key, *modifiers)
            }
            AppEvent::Unfocused => {
                self.active_drag_text_input = None;
                FrameworkOutput::default()
            }
            _ => FrameworkOutput::default(),
        };

        self.sync_sessions_for(&cx);
        outcome
    }

    pub fn ime_request(&self, tree: &Tree, focused: Option<NodeId>) -> ImeRequest {
        let focused = self.focused_widget_id(tree, focused);
        ImeRequest {
            allowed: focused.is_some(),
            cursor_area: focused.as_deref().and_then(|widget_id| {
                self.store
                    .runtime(widget_id)
                    .map(|runtime| runtime.caret_rect())
            }),
        }
    }

    pub fn paste_focused_text(
        &mut self,
        tree: &Tree,
        focused: Option<NodeId>,
        text: &str,
    ) -> FrameworkOutput {
        if text.is_empty() {
            return FrameworkOutput::default();
        }

        let Some(widget_id) = self.focused_widget_id(tree, focused) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.runtime_mut(&widget_id) else {
            return FrameworkOutput::default();
        };

        runtime.clear_preedit();
        runtime.editor_mut().paste(text);
        self.output_for_editor(tree, &widget_id)
    }

    pub fn store(&self) -> &TextInputStore {
        &self.store
    }

    fn commit_text_input(&mut self, cx: &SystemCx<'_>, text: &str) -> FrameworkOutput {
        if text.is_empty() {
            return FrameworkOutput::default();
        }

        let Some(widget_id) = self.focused_widget_id_for(cx) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.runtime_mut(&widget_id) else {
            return FrameworkOutput::default();
        };

        runtime.clear_preedit();
        runtime.editor_mut().insert_str(text);
        self.output_for_editor_for(cx, &widget_id)
            .with_consumed(true)
    }

    fn handle_ime_preedit(
        &mut self,
        cx: &SystemCx<'_>,
        text: &str,
        caret: Option<(usize, usize)>,
    ) -> FrameworkOutput {
        let Some(widget_id) = self.focused_widget_id_for(cx) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.runtime_mut(&widget_id) else {
            return FrameworkOutput::default();
        };

        runtime.set_preedit(text, caret);
        FrameworkOutput::consumed()
    }

    fn handle_key_press(
        &mut self,
        cx: &mut SystemCx<'_>,
        key: Key,
        modifiers: Modifiers,
    ) -> FrameworkOutput {
        if key == Key::Escape {
            if let Some(widget_id) = self.focused_widget_id_for(cx) {
                if let Some(runtime) = self.store.runtime_mut(&widget_id) {
                    if runtime.has_preedit() {
                        runtime.clear_preedit();
                        return FrameworkOutput::consumed();
                    }
                    if runtime.kind() == TextFieldKind::NumberInput {
                        runtime.revert_to_external();
                    }
                }
            }
            cx.blur();
            self.active_drag_text_input = None;
            return FrameworkOutput::consumed();
        }

        let Some(widget_id) = self.focused_widget_id_for(cx) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.runtime_mut(&widget_id) else {
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
            Key::Backspace => {
                runtime.editor_mut().backspace();
                self.output_for_editor_for(cx, &widget_id)
                    .with_consumed(true)
            }
            Key::Delete => {
                runtime.editor_mut().delete();
                self.output_for_editor_for(cx, &widget_id)
                    .with_consumed(true)
            }
            Key::Left => {
                if modifiers.shift {
                    runtime.editor_mut().select_left();
                } else {
                    runtime.editor_mut().move_left();
                }
                FrameworkOutput::consumed()
            }
            Key::Right => {
                if modifiers.shift {
                    runtime.editor_mut().select_right();
                } else {
                    runtime.editor_mut().move_right();
                }
                FrameworkOutput::consumed()
            }
            Key::Home => {
                if modifiers.shift {
                    runtime.editor_mut().select_to(0);
                } else {
                    runtime.editor_mut().move_home();
                }
                FrameworkOutput::consumed()
            }
            Key::End => {
                let text_end = runtime.editor().text().len();
                if modifiers.shift {
                    runtime.editor_mut().select_to(text_end);
                } else {
                    runtime.editor_mut().move_end();
                }
                FrameworkOutput::consumed()
            }
            Key::Up => self.step_number_input(cx, &widget_id, 1.0),
            Key::Down => self.step_number_input(cx, &widget_id, -1.0),
            Key::Enter => self.finalize_number_input(cx, &widget_id),
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
                let mut output = self.output_for_editor_for(cx, &widget_id);
                output.effects = effects;
                output.with_consumed(true)
            }
            Key::Char('V') if modifiers.ctrl || modifiers.meta => OutputBuilder::new()
                .effect(PlatformEffect::RequestClipboardPaste)
                .finish(),
            _ => FrameworkOutput::default(),
        }
    }

    fn focused_widget_id(&self, tree: &Tree, focused: Option<NodeId>) -> Option<String> {
        self.store.focused_widget_id(tree, focused)
    }

    fn captured_widget_id(&self, tree: &Tree, captured: Option<NodeId>) -> Option<String> {
        self.store.focused_widget_id(tree, captured)
    }

    fn focused_widget_id_for(&self, cx: &SystemCx<'_>) -> Option<String> {
        self.focused_widget_id(cx.tree(), cx.focused_node())
    }

    fn captured_widget_id_for(&self, cx: &SystemCx<'_>) -> Option<String> {
        self.captured_widget_id(cx.tree(), cx.captured_node())
    }

    fn text_input_id_at(&self, cx: &SystemCx<'_>, x: f32, y: f32) -> Option<String> {
        cx.hit_chain(x, y).iter().find_map(|node_id| {
            if cx.is_widget_type(node_id, "TextInput") || cx.is_widget_type(node_id, "NumberInput")
            {
                return cx.node_name(node_id).map(str::to_string);
            }
            None
        })
    }

    fn sync_sessions_for(&mut self, cx: &SystemCx<'_>) {
        self.sync_sessions(cx.tree(), cx.focused_node(), cx.captured_node());
    }

    fn sync_sessions(&mut self, tree: &Tree, focused: Option<NodeId>, captured: Option<NodeId>) {
        let focused = self.focused_widget_id(tree, focused);
        self.store.clear_unfocused_preedit(focused.as_deref());
        self.store.revert_unfocused_numbers(focused.as_deref());

        let keep_drag = self
            .active_drag_text_input
            .as_ref()
            .is_some_and(|widget_id| {
                Some(widget_id.as_str()) == focused.as_deref()
                    && Some(widget_id.as_str())
                        == self.captured_widget_id(tree, captured).as_deref()
                    && self.store.runtime(widget_id).is_some()
            });
        if !keep_drag {
            self.active_drag_text_input = None;
        }
    }

    fn output_for_editor(&self, tree: &Tree, widget_id: &str) -> FrameworkOutput {
        let Some(runtime) = self.store.runtime(widget_id) else {
            return FrameworkOutput::default();
        };

        match text_field_props(tree, widget_id) {
            Some(TextFieldProps::TextInput) => {
                changed_text_output(widget_id, runtime.editor().text())
            }
            Some(TextFieldProps::NumberInput(props)) => {
                live_number_output(widget_id, runtime.editor().text(), props)
            }
            None => FrameworkOutput::default(),
        }
    }

    fn output_for_editor_for(&self, cx: &SystemCx<'_>, widget_id: &str) -> FrameworkOutput {
        self.output_for_editor(cx.tree(), widget_id)
    }

    fn step_number_input(
        &mut self,
        cx: &SystemCx<'_>,
        widget_id: &str,
        direction: f32,
    ) -> FrameworkOutput {
        let Some(TextFieldProps::NumberInput(props)) = text_field_props_for(cx, widget_id) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.runtime_mut(widget_id) else {
            return FrameworkOutput::default();
        };

        let current = parse_number_text(runtime.editor().text()).unwrap_or(props.value);
        let next = (current + props.step * direction).clamp(props.min, props.max);
        runtime.clear_preedit();
        runtime
            .editor_mut()
            .set_text(&format_number(next, props.precision));
        changed_number_output(widget_id, next, props.value).with_consumed(true)
    }

    fn finalize_number_input(&mut self, cx: &SystemCx<'_>, widget_id: &str) -> FrameworkOutput {
        let Some(TextFieldProps::NumberInput(props)) = text_field_props_for(cx, widget_id) else {
            return FrameworkOutput::default();
        };
        let Some(runtime) = self.store.runtime_mut(widget_id) else {
            return FrameworkOutput::default();
        };

        match parse_number_text(runtime.editor().text()) {
            Some(parsed) if parsed >= props.min && parsed <= props.max => {
                let output = changed_number_output(widget_id, parsed, props.value);
                if output.events.is_empty() {
                    runtime.revert_to_external();
                }
                output.with_consumed(true)
            }
            _ => {
                runtime.revert_to_external();
                FrameworkOutput::consumed()
            }
        }
    }
}

impl Default for TextInputSystem {
    fn default() -> Self {
        Self::new()
    }
}

enum TextFieldProps<'a> {
    TextInput,
    NumberInput(&'a NumberInputProps),
}

fn text_field_props<'a>(tree: &'a Tree, widget_id: &str) -> Option<TextFieldProps<'a>> {
    let (_, node) = tree
        .iter()
        .find(|(_, node)| node.id.as_ref() == widget_id)?;
    let NodeKind::Widget(props) = &node.kind else {
        return None;
    };

    props
        .as_any()
        .downcast_ref::<TextInputProps>()
        .map(|_| TextFieldProps::TextInput)
        .or_else(|| {
            props
                .as_any()
                .downcast_ref::<NumberInputProps>()
                .map(TextFieldProps::NumberInput)
        })
}

fn text_field_props_for<'a>(cx: &'a SystemCx<'_>, widget_id: &str) -> Option<TextFieldProps<'a>> {
    text_field_props(cx.tree(), widget_id)
}

fn changed_text_output(widget_id: &str, value: &str) -> FrameworkOutput {
    OutputBuilder::new()
        .widget(WidgetEvent::TextChanged {
            id: widget_id.to_string(),
            value: value.to_string(),
        })
        .finish()
}

fn changed_number_output(widget_id: &str, value: f32, external_value: f32) -> FrameworkOutput {
    if (value - external_value).abs() < f32::EPSILON {
        FrameworkOutput::default()
    } else {
        OutputBuilder::new()
            .widget(WidgetEvent::NumberChanged {
                id: widget_id.to_string(),
                value,
            })
            .finish()
    }
}

fn live_number_output(widget_id: &str, text: &str, props: &NumberInputProps) -> FrameworkOutput {
    parse_number_text(text)
        .filter(|value| *value >= props.min && *value <= props.max)
        .map(|value| changed_number_output(widget_id, value, props.value))
        .unwrap_or_default()
}

fn parse_number_text(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if trimmed.is_empty() || matches!(trimmed, "+" | "-" | "." | "+." | "-.") {
        return None;
    }
    trimmed.parse::<f32>().ok()
}
