use crate::gesture::GestureArena;
use crate::renderer::{Rect, Renderer, TextMeasurer};
use crate::shell::{AppEvent, Key, Modifiers, MouseButton};
use crate::theme::Theme;
use crate::tree::{hit_test, layout, paint, reconcile, Desc, HitChain, NodeId, Tree};
use crate::widget::action::Action;
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::props::WidgetBuildCx;
use crate::widget::state::{InteractionStore, TextInputStore};

/// GUI 中心对象。持有统一的控件树与当前手势竞技场。
pub struct Context {
    tree: Tree,
    gesture_arena: Option<GestureArena>,
    interaction_state: InteractionStore,
    text_input_state: TextInputStore,
    active_drag_text_input: Option<String>,
    last_theme_revision: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImeRequest {
    pub allowed: bool,
    pub cursor_area: Option<Rect>,
}

#[derive(Debug)]
pub enum ClipboardRequest {
    Copy(String),
    Cut(String),
    Paste,
}

#[derive(Debug, Default)]
pub struct EventOutcome {
    pub actions: Vec<Action>,
    pub clipboard: Option<ClipboardRequest>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            tree: Tree::new(),
            gesture_arena: None,
            interaction_state: InteractionStore::new(),
            text_input_state: TextInputStore::new(),
            active_drag_text_input: None,
            last_theme_revision: None,
        }
    }

    /// 更新整棵树并重新布局。
    pub fn update(
        &mut self,
        desc: Desc,
        root_rect: Rect,
        measurer: &mut TextMeasurer,
        theme: &Theme,
    ) {
        let force_rebuild = self.last_theme_revision != Some(theme.revision);
        let build_cx = WidgetBuildCx {
            theme,
            force_rebuild,
        };
        reconcile(&mut self.tree, desc, build_cx);
        self.last_theme_revision = Some(theme.revision);
        if let Some(root) = self.tree.root() {
            layout(&mut self.tree, root, root_rect, &mut |text, size| {
                measurer.measure(text, size)
            });
        }
        self.interaction_state.sync_with_tree(&self.tree);
        self.text_input_state
            .sync_with_tree(&self.tree, measurer, theme);
        self.sync_text_input_sessions();
    }

    /// 渲染整棵树。
    pub fn render(
        &self,
        renderer: &mut Renderer,
        _viewport_w: f32,
        _viewport_h: f32,
        theme: &Theme,
    ) {
        if let Some(root) = self.tree.root() {
            paint(
                &self.tree,
                root,
                renderer,
                Some(&self.interaction_state),
                Some(&self.text_input_state),
                theme,
            );
        }
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> EventOutcome {
        self.interaction_state.handle_event(&self.tree, event);
        let outcome = self.handle_text_input_event(event);
        self.sync_text_input_sessions();
        outcome
    }

    pub fn ime_request(&self) -> ImeRequest {
        let focused = self.focused_text_input_id();
        ImeRequest {
            allowed: focused.is_some(),
            cursor_area: focused.as_deref().and_then(|widget_id| {
                self.text_input_state
                    .runtime(widget_id)
                    .map(|runtime| runtime.caret_rect())
            }),
        }
    }

    pub fn paste_focused_text(&mut self, text: &str) -> Vec<Action> {
        if text.is_empty() {
            return Vec::new();
        }

        let Some(widget_id) = self.focused_text_input_id() else {
            return Vec::new();
        };
        let Some(runtime) = self.text_input_state.runtime_mut(&widget_id) else {
            return Vec::new();
        };

        runtime.clear_preedit();
        let before = runtime.editor().text().to_string();
        runtime.editor_mut().paste(text);
        changed_action(&widget_id, &before, runtime.editor().text())
    }

    /// 命中测试，返回从叶子到根的命中链。
    pub fn hit_test(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test(&self.tree, root, x, y)
    }

    pub fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn gesture_arena(&self) -> Option<&GestureArena> {
        self.gesture_arena.as_ref()
    }

    pub fn interaction_state(&self) -> &InteractionStore {
        &self.interaction_state
    }

    pub fn gesture_arena_mut(&mut self) -> Option<&mut GestureArena> {
        self.gesture_arena.as_mut()
    }

    pub fn set_gesture_arena(&mut self, arena: GestureArena) {
        self.gesture_arena = Some(arena);
    }

    pub fn clear_gesture_arena(&mut self) {
        self.gesture_arena = None;
    }

    pub fn take_gesture_arena(&mut self) -> Option<GestureArena> {
        self.gesture_arena.take()
    }

    fn handle_text_input_event(&mut self, event: &AppEvent) -> EventOutcome {
        match event {
            AppEvent::MousePress { x, y, button } if *button == MouseButton::Left => {
                self.active_drag_text_input = None;
                if let Some(widget_id) = self.text_input_id_at(*x, *y) {
                    if let Some(runtime) = self.text_input_state.runtime_mut(&widget_id) {
                        runtime.clear_preedit();
                        runtime.set_caret_from_x(*x);
                        self.active_drag_text_input = Some(widget_id);
                    }
                }
                EventOutcome::default()
            }
            AppEvent::MouseMove { x, .. } => {
                if let Some(widget_id) = self.active_drag_text_input.clone() {
                    if self.captured_text_input_id().as_deref() == Some(widget_id.as_str()) {
                        if let Some(runtime) = self.text_input_state.runtime_mut(&widget_id) {
                            runtime.select_to_x(*x);
                        }
                    }
                }
                EventOutcome::default()
            }
            AppEvent::MouseRelease { button, .. } if *button == MouseButton::Left => {
                self.active_drag_text_input = None;
                EventOutcome::default()
            }
            AppEvent::ImePreedit { text, caret } => self.handle_ime_preedit(text, *caret),
            AppEvent::TextInput { text } => self.commit_text_input(text),
            AppEvent::KeyPress { key, modifiers } => self.handle_text_input_key(*key, *modifiers),
            AppEvent::Unfocused => {
                self.active_drag_text_input = None;
                EventOutcome::default()
            }
            _ => EventOutcome::default(),
        }
    }

    fn commit_text_input(&mut self, text: &str) -> EventOutcome {
        if text.is_empty() {
            return EventOutcome::default();
        }

        let Some(widget_id) = self.focused_text_input_id() else {
            return EventOutcome::default();
        };
        let Some(runtime) = self.text_input_state.runtime_mut(&widget_id) else {
            return EventOutcome::default();
        };

        runtime.clear_preedit();
        runtime.editor_mut().insert_str(text);
        EventOutcome {
            actions: vec![Action::TextChange {
                id: widget_id,
                value: runtime.editor().text().to_string(),
            }],
            clipboard: None,
        }
    }

    fn handle_ime_preedit(&mut self, text: &str, caret: Option<(usize, usize)>) -> EventOutcome {
        let Some(widget_id) = self.focused_text_input_id() else {
            return EventOutcome::default();
        };
        let Some(runtime) = self.text_input_state.runtime_mut(&widget_id) else {
            return EventOutcome::default();
        };

        runtime.set_preedit(text, caret);
        EventOutcome::default()
    }

    fn handle_text_input_key(&mut self, key: Key, modifiers: Modifiers) -> EventOutcome {
        if key == Key::Escape {
            if let Some(widget_id) = self.focused_text_input_id() {
                if let Some(runtime) = self.text_input_state.runtime_mut(&widget_id) {
                    if runtime.has_preedit() {
                        runtime.clear_preedit();
                        return EventOutcome::default();
                    }
                }
            }
            self.interaction_state.blur();
            self.active_drag_text_input = None;
            return EventOutcome::default();
        }

        let Some(widget_id) = self.focused_text_input_id() else {
            return EventOutcome::default();
        };
        let Some(runtime) = self.text_input_state.runtime_mut(&widget_id) else {
            return EventOutcome::default();
        };

        if runtime.has_preedit() {
            match key {
                Key::Char('A' | 'C' | 'X' | 'V') if modifiers.ctrl || modifiers.meta => {
                    runtime.clear_preedit();
                }
                _ => return EventOutcome::default(),
            }
        }

        match key {
            Key::Backspace => {
                let before = runtime.editor().text().to_string();
                runtime.editor_mut().backspace();
                EventOutcome::from_actions(changed_action(
                    &widget_id,
                    &before,
                    runtime.editor().text(),
                ))
            }
            Key::Delete => {
                let before = runtime.editor().text().to_string();
                runtime.editor_mut().delete();
                EventOutcome::from_actions(changed_action(
                    &widget_id,
                    &before,
                    runtime.editor().text(),
                ))
            }
            Key::Left => {
                if modifiers.shift {
                    runtime.editor_mut().select_left();
                } else {
                    runtime.editor_mut().move_left();
                }
                EventOutcome::default()
            }
            Key::Right => {
                if modifiers.shift {
                    runtime.editor_mut().select_right();
                } else {
                    runtime.editor_mut().move_right();
                }
                EventOutcome::default()
            }
            Key::Home => {
                if modifiers.shift {
                    runtime.editor_mut().select_to(0);
                } else {
                    runtime.editor_mut().move_home();
                }
                EventOutcome::default()
            }
            Key::End => {
                let text_end = runtime.editor().text().len();
                if modifiers.shift {
                    runtime.editor_mut().select_to(text_end);
                } else {
                    runtime.editor_mut().move_end();
                }
                EventOutcome::default()
            }
            Key::Char('A') if modifiers.ctrl || modifiers.meta => {
                runtime.editor_mut().select_all();
                EventOutcome::default()
            }
            Key::Char('C') if modifiers.ctrl || modifiers.meta => runtime
                .editor()
                .copy()
                .map(|text| EventOutcome {
                    actions: Vec::new(),
                    clipboard: Some(ClipboardRequest::Copy(text)),
                })
                .unwrap_or_default(),
            Key::Char('X') if modifiers.ctrl || modifiers.meta => {
                let before = runtime.editor().text().to_string();
                let clipboard = runtime.editor_mut().cut().map(ClipboardRequest::Cut);
                EventOutcome {
                    actions: changed_action(&widget_id, &before, runtime.editor().text()),
                    clipboard,
                }
            }
            Key::Char('V') if modifiers.ctrl || modifiers.meta => EventOutcome {
                actions: Vec::new(),
                clipboard: Some(ClipboardRequest::Paste),
            },
            _ => EventOutcome::default(),
        }
    }

    fn focused_text_input_id(&self) -> Option<String> {
        self.text_input_state
            .focused_widget_id(&self.tree, self.interaction_state.focused())
    }

    fn captured_text_input_id(&self) -> Option<String> {
        self.text_input_state
            .focused_widget_id(&self.tree, self.interaction_state.captured())
    }

    fn text_input_id_at(&self, x: f32, y: f32) -> Option<String> {
        self.hit_test(x, y).iter().find_map(|node_id| {
            let node = self.tree.get(node_id)?;
            let crate::tree::NodeKind::Widget(props) = &node.kind else {
                return None;
            };
            props
                .as_any()
                .downcast_ref::<TextInputProps>()
                .map(|_| node.id.to_string())
        })
    }

    fn sync_text_input_sessions(&mut self) {
        let focused = self.focused_text_input_id();
        self.text_input_state
            .clear_unfocused_preedit(focused.as_deref());

        let keep_drag = self
            .active_drag_text_input
            .as_ref()
            .is_some_and(|widget_id| {
                Some(widget_id.as_str()) == focused.as_deref()
                    && Some(widget_id.as_str()) == self.captured_text_input_id().as_deref()
                    && self.text_input_state.runtime(widget_id).is_some()
            });
        if !keep_drag {
            self.active_drag_text_input = None;
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

fn changed_action(widget_id: &str, before: &str, after: &str) -> Vec<Action> {
    if before == after {
        Vec::new()
    } else {
        vec![Action::TextChange {
            id: widget_id.to_string(),
            value: after.to_string(),
        }]
    }
}

impl EventOutcome {
    fn from_actions(actions: Vec<Action>) -> Self {
        Self {
            actions,
            clipboard: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::TextMeasurer;
    use crate::shell::Modifiers;
    use crate::theme::dark_theme;
    use crate::tree::layout::{BoxStyle, Size};
    use std::borrow::Cow;

    fn test_desc(value: &str) -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(120.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("input"),
                props: Box::new(TextInputProps {
                    label: Cow::Borrowed("Prompt"),
                    value: Cow::Owned(value.to_string()),
                    disabled: false,
                }),
            }],
        }
    }

    fn test_context(value: &str) -> Context {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            test_desc(value),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx
    }

    fn field_rect(ctx: &Context) -> Rect {
        ctx.tree()
            .iter()
            .find_map(|(_, node)| (node.id.as_ref() == "input::field").then_some(node.rect))
            .expect("text input field rect")
    }

    fn focus_input(ctx: &mut Context) {
        let rect = field_rect(ctx);
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + 4.0,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
    }

    #[test]
    fn ctrl_c_requests_clipboard_copy_without_text_change() {
        let mut ctx = test_context("hello");
        focus_input(&mut ctx);

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        let outcome = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('C'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        assert!(outcome.actions.is_empty());
        assert!(matches!(
            outcome.clipboard,
            Some(ClipboardRequest::Copy(text)) if text == "hello"
        ));
    }

    #[test]
    fn ctrl_x_requests_cut_and_returns_text_change() {
        let mut ctx = test_context("hello");
        focus_input(&mut ctx);

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        let outcome = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('X'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        assert!(matches!(
            outcome.clipboard,
            Some(ClipboardRequest::Cut(text)) if text == "hello"
        ));
        assert!(matches!(
            outcome.actions.as_slice(),
            [Action::TextChange { id, value }] if id == "input" && value.is_empty()
        ));
    }

    #[test]
    fn drag_selection_can_be_copied() {
        let mut ctx = test_context("hello world");
        let rect = field_rect(&ctx);
        let y = rect.y + rect.h * 0.5;
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + 4.0,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseMove {
            x: rect.x + 48.0,
            y,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + 48.0,
            y,
            button: MouseButton::Left,
        });

        let outcome = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('C'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        assert!(matches!(
            outcome.clipboard,
            Some(ClipboardRequest::Copy(text)) if !text.is_empty()
        ));
    }

    #[test]
    fn theme_revision_forces_widget_rebuild() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let dark = dark_theme();
        let mut updated = dark_theme();
        updated.revision = 99;
        updated.components.text_input.field_height = 52.0;

        ctx.update(
            test_desc("hello"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &dark,
        );

        let before_height = ctx
            .tree()
            .iter()
            .find_map(|(_, node)| (node.id.as_ref() == "input::field").then_some(node.rect.h))
            .expect("text input field");

        ctx.update(
            test_desc("hello"),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &updated,
        );

        let after_height = ctx
            .tree()
            .iter()
            .find_map(|(_, node)| (node.id.as_ref() == "input::field").then_some(node.rect.h))
            .expect("text input field");

        assert_eq!(before_height, dark.components.text_input.field_height);
        assert_eq!(after_height, updated.components.text_input.field_height);
    }
}
