use crate::canvas::canvas_node_stable_id;
use crate::canvas::node_template::CanvasNodeRenderView;
use crate::context::ImeRequest;
use crate::output::{FrameworkOutput, OutputBuilder, PlatformEffect, WidgetEvent};
use crate::shell::{AppEvent, Key, Modifiers, MouseButton};
use crate::theme::{ControlSize, Density, Theme};
use crate::tree::{NodeId, Tree};
use crate::widget::atoms::number_input::format_number;
use crate::widget::atoms::text_box::{TextBoxFont, TextBoxMode};
use crate::widget::mapping::ParamControlSpec;
use crate::widget::state::{TextBoxSpec, TextBoxStore, TextBoxValueKind};
use crate::widget::systems::SystemCx;

pub(crate) struct TextBoxSystem {
    store: TextBoxStore,
    active_drag_text_box: Option<String>,
}

impl TextBoxSystem {
    pub(crate) fn new() -> Self {
        Self {
            store: TextBoxStore::new(),
            active_drag_text_box: None,
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        mut cx: SystemCx<'_>,
        event: &AppEvent,
    ) -> FrameworkOutput {
        let outcome = match event {
            AppEvent::MousePress { x, y, button } if *button == MouseButton::Left => {
                self.active_drag_text_box = None;
                if let Some(widget_id) = self.text_box_id_at(&cx, *x, *y) {
                    let point = self.text_box_pointer_point(&cx, &widget_id, *x, *y);
                    if let Some(runtime) = self.store.runtime_mut(&widget_id) {
                        runtime.clear_preedit();
                        runtime.set_caret_from_point(point.x, point.y);
                        self.active_drag_text_box = Some(widget_id);
                        return FrameworkOutput::consumed();
                    }
                }
                FrameworkOutput::default()
            }
            AppEvent::MouseMove { x, y } => {
                if let Some(widget_id) = self.active_drag_text_box.clone() {
                    if self.captured_widget_id_for(&cx).as_deref() == Some(widget_id.as_str()) {
                        let point = self.text_box_pointer_point(&cx, &widget_id, *x, *y);
                        if let Some(runtime) = self.store.runtime_mut(&widget_id) {
                            runtime.select_to_point(point.x, point.y);
                            return FrameworkOutput::consumed();
                        }
                    }
                }
                FrameworkOutput::default()
            }
            AppEvent::MouseRelease { button, .. } if *button == MouseButton::Left => {
                let consumed = self.active_drag_text_box.is_some();
                self.active_drag_text_box = None;
                FrameworkOutput::default().with_consumed(consumed)
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

    pub(crate) fn ime_request(&self, tree: &Tree, focused: Option<NodeId>) -> ImeRequest {
        let focused = self.focused_widget_id(tree, focused);
        ImeRequest {
            allowed: focused.is_some(),
            cursor_area: focused
                .as_deref()
                .and_then(|widget_id| self.store.runtime(widget_id).map(|r| r.caret_rect())),
        }
    }

    pub(crate) fn paste_focused_text(
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
        self.output_for_editor(tree, &widget_id).with_consumed(true)
    }

    pub(crate) fn store(&self) -> &TextBoxStore {
        &self.store
    }

    pub(crate) fn store_mut(&mut self) -> &mut TextBoxStore {
        &mut self.store
    }

    pub(crate) fn sync_retained_canvas_text_boxes(
        &mut self,
        tree: &Tree,
        views: &[CanvasNodeRenderView],
        measurer: &mut crate::renderer::TextMeasurer,
        theme: &Theme,
        focused: Option<NodeId>,
    ) {
        let focused_widget_id = self.focused_widget_id(tree, focused);
        for view in views {
            let stable_id = canvas_node_stable_id(&view.state.owner_id);
            for (index, param) in view.template.params.iter().enumerate() {
                let widget_id = format!("{stable_id}::body::param::{index}::control::widget");
                let Some(spec) = retained_param_text_box_spec(&param.control, theme) else {
                    continue;
                };
                self.store.sync_retained_text_box(
                    tree,
                    measurer,
                    theme,
                    widget_id,
                    spec,
                    focused_widget_id.as_deref(),
                );
            }
        }
        self.sync_sessions(tree, focused, None);
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
                    if runtime.is_number() {
                        runtime.revert_to_external();
                    }
                }
            }
            cx.blur();
            self.active_drag_text_box = None;
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
            Key::Enter => {
                if runtime.is_multiline() {
                    runtime.editor_mut().insert_char('\n');
                    return self
                        .output_for_editor_for(cx, &widget_id)
                        .with_consumed(true);
                }
                self.finalize_number_input(&widget_id)
            }
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
                runtime.move_left(modifiers.shift);
                FrameworkOutput::consumed()
            }
            Key::Right => {
                runtime.move_right(modifiers.shift);
                FrameworkOutput::consumed()
            }
            Key::Up => {
                if runtime.is_number() {
                    self.step_number_input(&widget_id, 1.0)
                } else if runtime.is_multiline() {
                    runtime.move_vertical(-1, modifiers.shift);
                    FrameworkOutput::consumed()
                } else {
                    FrameworkOutput::default()
                }
            }
            Key::Down => {
                if runtime.is_number() {
                    self.step_number_input(&widget_id, -1.0)
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

    fn text_box_id_at(&self, cx: &SystemCx<'_>, x: f32, y: f32) -> Option<String> {
        cx.hit_chain(x, y).iter().find_map(|node_id| {
            let name = cx.node_name(node_id)?;
            if self.store.runtime(name).is_some() {
                return Some(name.to_string());
            }
            retained_prefixes(name).find_map(|prefix| {
                self.store
                    .runtime(prefix)
                    .is_some()
                    .then(|| prefix.to_string())
            })
        })
    }

    fn text_box_pointer_point(
        &self,
        cx: &SystemCx<'_>,
        widget_id: &str,
        x: f32,
        y: f32,
    ) -> crate::renderer::Point {
        let field_id = format!("{widget_id}::field");
        cx.node_id_by_name(&field_id)
            .or_else(|| cx.node_id_by_name(widget_id))
            .and_then(|node_id| cx.screen_to_node_layout_point(node_id, x, y))
            .unwrap_or(crate::renderer::Point { x, y })
    }

    fn sync_sessions_for(&mut self, cx: &SystemCx<'_>) {
        self.sync_sessions(cx.tree(), cx.focused_node(), cx.captured_node());
    }

    fn sync_sessions(&mut self, tree: &Tree, focused: Option<NodeId>, captured: Option<NodeId>) {
        let focused = self.focused_widget_id(tree, focused);
        self.store.clear_unfocused_preedit(focused.as_deref());
        self.store.revert_unfocused_numbers(focused.as_deref());

        let keep_drag = self.active_drag_text_box.as_ref().is_some_and(|widget_id| {
            Some(widget_id.as_str()) == focused.as_deref()
                && Some(widget_id.as_str()) == self.captured_widget_id(tree, captured).as_deref()
                && self.store.runtime(widget_id).is_some()
        });
        if !keep_drag {
            self.active_drag_text_box = None;
        }
    }

    fn output_for_editor(&self, _tree: &Tree, widget_id: &str) -> FrameworkOutput {
        let Some(runtime) = self.store.runtime(widget_id) else {
            return FrameworkOutput::default();
        };

        match runtime.value_kind() {
            TextBoxValueKind::Text => changed_text_output(widget_id, runtime.editor().text()),
            TextBoxValueKind::Number {
                value, min, max, ..
            } => live_number_output(widget_id, runtime.editor().text(), value, min, max),
        }
    }

    fn output_for_editor_for(&self, cx: &SystemCx<'_>, widget_id: &str) -> FrameworkOutput {
        self.output_for_editor(cx.tree(), widget_id)
    }

    fn step_number_input(&mut self, widget_id: &str, direction: f32) -> FrameworkOutput {
        let Some(runtime) = self.store.runtime_mut(widget_id) else {
            return FrameworkOutput::default();
        };
        let TextBoxValueKind::Number {
            value,
            min,
            max,
            step,
            precision,
        } = runtime.value_kind()
        else {
            return FrameworkOutput::default();
        };

        let current = parse_number_text(runtime.editor().text()).unwrap_or(value);
        let next = (current + step * direction).clamp(min, max);
        runtime.clear_preedit();
        runtime
            .editor_mut()
            .set_text(&format_number(next, precision));
        changed_number_output(widget_id, next, value).with_consumed(true)
    }

    fn finalize_number_input(&mut self, widget_id: &str) -> FrameworkOutput {
        let Some(runtime) = self.store.runtime_mut(widget_id) else {
            return FrameworkOutput::default();
        };
        let TextBoxValueKind::Number {
            value, min, max, ..
        } = runtime.value_kind()
        else {
            return FrameworkOutput::default();
        };

        match parse_number_text(runtime.editor().text()) {
            Some(parsed) if parsed >= min && parsed <= max => {
                let output = changed_number_output(widget_id, parsed, value);
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

impl Default for TextBoxSystem {
    fn default() -> Self {
        Self::new()
    }
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

fn live_number_output(
    widget_id: &str,
    text: &str,
    external_value: f32,
    min: f32,
    max: f32,
) -> FrameworkOutput {
    parse_number_text(text)
        .filter(|value| *value >= min && *value <= max)
        .map(|value| changed_number_output(widget_id, value, external_value))
        .unwrap_or_default()
}

fn parse_number_text(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if trimmed.is_empty() || matches!(trimmed, "+" | "-" | "." | "+." | "-.") {
        return None;
    }
    trimmed.parse::<f32>().ok()
}

fn retained_param_text_box_spec(control: &ParamControlSpec, theme: &Theme) -> Option<TextBoxSpec> {
    let tokens = theme.text_field_metrics(ControlSize::Small, Density::Compact);
    match control {
        ParamControlSpec::Text { value } => Some(TextBoxSpec {
            external_text: value.clone(),
            mode: TextBoxMode::SingleLine,
            value_kind: TextBoxValueKind::Text,
            tokens,
            font: TextBoxFont::Body,
            disabled: false,
        }),
        ParamControlSpec::TextArea { value, min_rows } => Some(TextBoxSpec {
            external_text: value.clone(),
            mode: TextBoxMode::MultiLine {
                min_rows: *min_rows,
            },
            value_kind: TextBoxValueKind::Text,
            tokens,
            font: TextBoxFont::Body,
            disabled: false,
        }),
        ParamControlSpec::Number {
            value,
            min,
            max,
            step,
            precision,
        } => Some(TextBoxSpec {
            external_text: format_number(*value, *precision),
            mode: TextBoxMode::SingleLine,
            value_kind: TextBoxValueKind::Number {
                value: *value,
                min: *min,
                max: *max,
                step: *step,
                precision: *precision,
            },
            tokens,
            font: TextBoxFont::Mono,
            disabled: false,
        }),
        _ => None,
    }
}

fn retained_prefixes(id: &str) -> impl Iterator<Item = &str> {
    id.match_indices("::")
        .map(|(index, _)| &id[..index])
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
}
