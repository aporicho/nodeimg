use std::time::Instant;
use std::{collections::HashMap, sync::Arc};

use crate::gesture::GestureArena;
use crate::renderer::{Rect, Renderer, TextMeasurer};
use crate::shell::AppEvent;
use crate::theme::Theme;
use crate::tree::layout::{Overflow, TextureHandle};
use crate::tree::{hit_test, layout, paint, reconcile, Desc, HitChain, NodeId, Tree};
use crate::widget::action::Action;
use crate::widget::props::WidgetBuildCx;
use crate::widget::state::InteractionStore;
use crate::widget::systems::{DropdownSystem, PopupSystem, TextInputSystem};

pub use crate::widget::systems::{OverlayPlacement, OverlayRequest};

/// GUI 中心对象。持有统一的控件树与当前手势竞技场。
pub struct Context {
    tree: Tree,
    gesture_arena: Option<GestureArena>,
    interaction_state: InteractionStore,
    dropdown_system: DropdownSystem,
    popup_system: PopupSystem,
    text_input_system: TextInputSystem,
    last_theme_revision: Option<u64>,
    last_tap_time: Option<Instant>,
    textures: HashMap<TextureHandle, Arc<wgpu::TextureView>>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImeRequest {
    pub allowed: bool,
    pub cursor_area: Option<Rect>,
}

#[derive(Debug, Clone)]
pub enum PlatformEffect {
    WriteClipboard(String),
    RequestClipboardPaste,
}

#[derive(Debug, Default)]
pub struct FrameworkOutput {
    pub actions: Vec<Action>,
    pub effects: Vec<PlatformEffect>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            tree: Tree::new(),
            gesture_arena: None,
            interaction_state: InteractionStore::new(),
            dropdown_system: DropdownSystem::new(),
            popup_system: PopupSystem::new(),
            text_input_system: TextInputSystem::new(),
            last_theme_revision: None,
            last_tap_time: None,
            textures: HashMap::new(),
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
        let desc = self.popup_system.compose_desc(&self.tree, desc, root_rect);
        let force_rebuild = self.last_theme_revision != Some(theme.revision);
        let build_cx = WidgetBuildCx {
            theme,
            force_rebuild,
        };
        reconcile(&mut self.tree, desc, build_cx);
        self.last_theme_revision = Some(theme.revision);
        if let Some(root) = self.tree.root() {
            layout(&mut self.tree, root, root_rect, &mut |text, style| {
                measurer.measure_with_style(text, style)
            });
        }
        self.interaction_state.sync_with_tree(&self.tree);
        self.text_input_system.sync_with_tree(
            &self.tree,
            measurer,
            theme,
            self.interaction_state.focused(),
            self.interaction_state.captured(),
        );
        self.dropdown_system
            .sync_with_tree(&self.tree, &self.popup_system);
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
                Some(self.text_input_system.store()),
                Some(&self.textures),
                theme,
            );
        }
    }

    pub fn register_texture(&mut self, handle: TextureHandle, view: Arc<wgpu::TextureView>) {
        self.textures.insert(handle, view);
    }

    pub fn open_overlay(&mut self, request: OverlayRequest) {
        self.popup_system.open(&self.tree, request);
    }

    pub fn close_overlay(&mut self) {
        self.popup_system
            .close(&self.tree, &mut self.interaction_state);
    }

    pub fn overlay_open(&self) -> bool {
        self.popup_system.is_open()
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        self.interaction_state.handle_event(&self.tree, event);
        self.handle_scroll_event(event);
        if self
            .popup_system
            .handle_event(&self.tree, &mut self.interaction_state, event)
        {
            return FrameworkOutput::default();
        }
        let dropdown_output = self.dropdown_system.handle_event(
            &self.tree,
            &mut self.interaction_state,
            &mut self.popup_system,
            event,
        );
        let text_output =
            self.text_input_system
                .handle_event(&self.tree, &mut self.interaction_state, event);
        if !dropdown_output.actions.is_empty() || !dropdown_output.effects.is_empty() {
            self.clear_gesture_arena();
            return dropdown_output.merge(text_output);
        }
        let gesture_output = self.handle_gesture_event(event);
        dropdown_output.merge(text_output).merge(gesture_output)
    }

    pub fn ime_request(&self) -> ImeRequest {
        self.text_input_system
            .ime_request(&self.tree, self.interaction_state.focused())
    }

    pub fn paste_focused_text(&mut self, text: &str) -> Vec<crate::widget::action::Action> {
        self.text_input_system.paste_focused_text(
            &self.tree,
            self.interaction_state.focused(),
            text,
        )
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
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkOutput {
    pub(crate) fn from_actions(actions: Vec<crate::widget::action::Action>) -> Self {
        Self {
            actions,
            effects: Vec::new(),
        }
    }

    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.actions.extend(other.actions);
        self.effects.extend(other.effects);
        self
    }
}

impl Context {
    fn handle_gesture_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        match *event {
            AppEvent::MousePress { x, y, button }
                if button == crate::shell::MouseButton::Left && self.gesture_arena.is_none() =>
            {
                let chain = self.hit_test(x, y);
                if let Some(arena) = crate::gesture::arena_from_hit_chain(
                    &self.tree,
                    &chain,
                    x,
                    y,
                    self.last_tap_time,
                ) {
                    self.set_gesture_arena(arena);
                }
                FrameworkOutput::default()
            }
            AppEvent::MouseMove { x, y } => {
                let Some(arena) = self.gesture_arena_mut() else {
                    return FrameworkOutput::default();
                };
                arena
                    .pointer_move(x, y)
                    .map(|action| self.record_gesture_action(action))
                    .unwrap_or_default()
            }
            AppEvent::MouseRelease { x, y, button }
                if button == crate::shell::MouseButton::Left =>
            {
                let Some(mut arena) = self.take_gesture_arena() else {
                    return FrameworkOutput::default();
                };
                arena
                    .pointer_up(x, y)
                    .map(|action| self.record_gesture_action(action))
                    .unwrap_or_default()
            }
            AppEvent::Unfocused => {
                self.clear_gesture_arena();
                FrameworkOutput::default()
            }
            _ => FrameworkOutput::default(),
        }
    }

    fn record_gesture_action(&mut self, action: Action) -> FrameworkOutput {
        match &action {
            Action::Click(_) => {
                self.last_tap_time = Some(Instant::now());
            }
            Action::DoubleClick(_) => {
                self.last_tap_time = None;
            }
            _ => {}
        }
        FrameworkOutput::from_actions(vec![action])
    }

    fn handle_scroll_event(&mut self, event: &AppEvent) {
        let Some((x, y, delta)) = scroll_event_delta(event) else {
            return;
        };
        let Some(node_id) = self.scroll_target_at(x, y) else {
            return;
        };
        self.tree.scroll(node_id, delta);
    }

    fn scroll_target_at(&self, x: f32, y: f32) -> Option<NodeId> {
        self.hit_test(x, y).iter().find(|&node_id| {
            self.tree
                .get(node_id)
                .map(|node| node.style.overflow == Overflow::Scroll)
                .unwrap_or(false)
        })
    }
}

fn scroll_event_delta(event: &AppEvent) -> Option<(f32, f32, f32)> {
    match *event {
        AppEvent::ScrollLine { x, y, delta_y, .. } => Some((x, y, -delta_y * 32.0)),
        AppEvent::ScrollPixel { x, y, delta_y, .. } => Some((x, y, -delta_y)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::TextMeasurer;
    use crate::shell::{Key, Modifiers, MouseButton};
    use crate::theme::dark_theme;
    use crate::tree::layout::{BoxStyle, Size};
    use crate::widget::atoms::button::ButtonProps;
    use crate::widget::atoms::dropdown::DropdownProps;
    use crate::widget::atoms::label::{LabelProps, LabelVariant};
    use crate::widget::atoms::number_input::NumberInputProps;
    use crate::widget::atoms::text_input::TextInputProps;
    use crate::widget::frameworks::scroll_area::ScrollAreaProps;
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

    fn button_desc() -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(120.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("button"),
                props: Box::new(ButtonProps {
                    label: Cow::Borrowed("Run"),
                    icon: None,
                    disabled: false,
                }),
            }],
        }
    }

    fn popup_content_desc() -> Desc {
        Desc::Widget {
            id: Cow::Borrowed("popup_button"),
            props: Box::new(ButtonProps {
                label: Cow::Borrowed("Overlay"),
                icon: None,
                disabled: false,
            }),
        }
    }

    fn scroll_desc() -> Desc {
        let items = (0..20)
            .map(|index| Desc::Widget {
                id: Cow::Owned(format!("item_{index}")),
                props: Box::new(LabelProps {
                    text: Cow::Owned(format!("Item {index}")),
                    variant: LabelVariant::Body,
                    muted: false,
                }),
            })
            .collect();

        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(200.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("scroll"),
                props: Box::new(ScrollAreaProps {
                    height: 80.0,
                    content: items,
                }),
            }],
        }
    }

    fn number_desc(value: f32) -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(120.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("number"),
                props: Box::new(NumberInputProps {
                    label: Cow::Borrowed("Radius"),
                    value,
                    min: 0.0,
                    max: 10.0,
                    step: 0.5,
                    precision: 2,
                    disabled: false,
                }),
            }],
        }
    }

    fn dropdown_desc(selected: usize) -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(180.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("dropdown"),
                props: Box::new(DropdownProps {
                    label: Cow::Borrowed("Mode"),
                    options: vec![
                        Cow::Borrowed("Normal"),
                        Cow::Borrowed("Multiply"),
                        Cow::Borrowed("Screen"),
                    ],
                    selected,
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

    fn button_rect(ctx: &Context) -> Rect {
        ctx.tree()
            .iter()
            .find_map(|(_, node)| (node.id.as_ref() == "button").then_some(node.rect))
            .expect("button rect")
    }

    fn node_rect(ctx: &Context, node_name: &str) -> Rect {
        ctx.tree()
            .iter()
            .find_map(|(_, node)| (node.id.as_ref() == node_name).then_some(node.rect))
            .expect("node rect")
    }

    fn focus_number(ctx: &mut Context) {
        let rect = node_rect(ctx, "number::field");
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

    fn dropdown_field_rect(ctx: &Context) -> Rect {
        node_rect(ctx, "dropdown::field")
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
            outcome.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if text == "hello"
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
            outcome.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if text == "hello"
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
            outcome.effects.as_slice(),
            [PlatformEffect::WriteClipboard(text)] if !text.is_empty()
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

    #[test]
    fn context_emits_click_without_demo_owned_gesture_arena() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = button_rect(&ctx);

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });
        let output = ctx.handle_event(&AppEvent::MouseRelease {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            button: MouseButton::Left,
        });

        assert!(matches!(
            output.actions.as_slice(),
            [Action::Click(id)] if id == "button"
        ));
    }

    #[test]
    fn context_tracks_double_click_without_demo_tap_state() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = button_rect(&ctx);
        let x = rect.x + rect.w * 0.5;
        let y = rect.y + rect.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let output = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });

        assert!(matches!(
            output.actions.as_slice(),
            [Action::DoubleClick(id)] if id == "button"
        ));
    }

    #[test]
    fn overlay_open_is_composed_into_tree_and_hit_chain() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.open_overlay(OverlayRequest {
            id: "test_popup".to_string(),
            anchor_id: "button".to_string(),
            restore_focus_id: Some("button".to_string()),
            placement: OverlayPlacement::BelowStart,
            content: popup_content_desc(),
            offset_x: 0.0,
            offset_y: 8.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: true,
        });
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let popup_rect = node_rect(&ctx, "__overlay::test_popup");
        let chain = ctx.hit_test(popup_rect.x + 2.0, popup_rect.y + 2.0);
        assert!(chain.iter().any(|node_id| {
            ctx.tree()
                .get(node_id)
                .is_some_and(|node| node.id.as_ref().starts_with("__overlay::test_popup"))
        }));
    }

    #[test]
    fn outside_click_dismisses_overlay() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.open_overlay(OverlayRequest {
            id: "test_popup".to_string(),
            anchor_id: "button".to_string(),
            restore_focus_id: Some("button".to_string()),
            placement: OverlayPlacement::BelowStart,
            content: popup_content_desc(),
            offset_x: 0.0,
            offset_y: 8.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: true,
        });
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: 300.0,
            y: 100.0,
            button: MouseButton::Left,
        });

        assert!(!ctx.overlay_open());
    }

    #[test]
    fn escape_dismisses_overlay_and_restores_focus() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.open_overlay(OverlayRequest {
            id: "test_popup".to_string(),
            anchor_id: "button".to_string(),
            restore_focus_id: Some("button".to_string()),
            placement: OverlayPlacement::BelowStart,
            content: popup_content_desc(),
            offset_x: 0.0,
            offset_y: 8.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: true,
        });
        ctx.update(
            button_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        ctx.interaction_state.blur();

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Escape,
            modifiers: Modifiers::default(),
        });

        assert!(!ctx.overlay_open());
        let focused_name = ctx
            .interaction_state
            .focused()
            .and_then(|id| ctx.tree().get(id))
            .map(|node| node.id.as_ref().to_string());
        assert_eq!(focused_name.as_deref(), Some("button"));
    }

    #[test]
    fn scroll_events_update_scroll_area_offset() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            scroll_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 200.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "scroll");

        let _ = ctx.handle_event(&AppEvent::ScrollPixel {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            delta_x: 0.0,
            delta_y: -24.0,
        });

        let (_, node) = ctx
            .tree()
            .iter()
            .find(|(_, node)| node.id.as_ref() == "scroll")
            .expect("scroll node");
        assert!(node.scroll_offset > 0.0);
    }

    #[test]
    fn number_input_emits_number_change_for_valid_text() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            number_desc(1.5),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        focus_number(&mut ctx);
        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });

        let output = ctx.handle_event(&AppEvent::TextInput {
            text: "2".to_string(),
        });

        assert!(matches!(
            output.actions.as_slice(),
            [Action::NumberChange { id, value }] if id == "number" && (*value - 2.0).abs() < 0.0001
        ));
    }

    #[test]
    fn number_input_reverts_invalid_text_on_blur() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            number_desc(1.5),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        focus_number(&mut ctx);

        let output = ctx.handle_event(&AppEvent::TextInput {
            text: "a".to_string(),
        });
        assert!(output.actions.is_empty());
        assert_eq!(
            ctx.text_input_system
                .store()
                .runtime("number")
                .unwrap()
                .editor()
                .text(),
            "a1.50"
        );

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: 300.0,
            y: 100.0,
            button: MouseButton::Left,
        });
        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x: 300.0,
            y: 100.0,
            button: MouseButton::Left,
        });

        assert_eq!(
            ctx.text_input_system
                .store()
                .runtime("number")
                .unwrap()
                .editor()
                .text(),
            "1.50"
        );
    }

    #[test]
    fn number_input_step_uses_current_edited_value() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 120.0,
        };
        ctx.update(number_desc(1.5), viewport, &mut measurer, &theme);
        focus_number(&mut ctx);
        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Char('A'),
            modifiers: Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        });
        let _ = ctx.handle_event(&AppEvent::TextInput {
            text: "2.5".to_string(),
        });

        let output = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Up,
            modifiers: Modifiers::default(),
        });

        assert!(matches!(
            output.actions.as_slice(),
            [Action::NumberChange { id, value }] if id == "number" && (*value - 3.0).abs() < 0.0001
        ));
    }

    #[test]
    fn focused_number_input_does_not_clobber_dirty_editor_on_external_sync() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 120.0,
        };
        ctx.update(number_desc(1.5), viewport, &mut measurer, &theme);
        focus_number(&mut ctx);
        let _ = ctx.handle_event(&AppEvent::TextInput {
            text: "a".to_string(),
        });

        ctx.update(number_desc(2.0), viewport, &mut measurer, &theme);

        assert_eq!(
            ctx.text_input_system
                .store()
                .runtime("number")
                .unwrap()
                .editor()
                .text(),
            "a1.50"
        );
        assert_eq!(
            ctx.text_input_system
                .store()
                .runtime("number")
                .unwrap()
                .external_text(),
            "2.00"
        );
    }

    #[test]
    fn dropdown_click_open_and_select_emits_select_change() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 360.0,
        };
        ctx.update(dropdown_desc(0), viewport, &mut measurer, &theme);
        let rect = dropdown_field_rect(&ctx);

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
        assert!(ctx.overlay_open());

        ctx.update(dropdown_desc(0), viewport, &mut measurer, &theme);
        let option_rect = node_rect(&ctx, "__dropdown_option::dropdown::1");
        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: option_rect.x + 4.0,
            y: option_rect.y + option_rect.h * 0.5,
            button: MouseButton::Left,
        });
        let output = ctx.handle_event(&AppEvent::MouseRelease {
            x: option_rect.x + 4.0,
            y: option_rect.y + option_rect.h * 0.5,
            button: MouseButton::Left,
        });
        eprintln!("dropdown output: {:?}", output.actions);

        assert!(matches!(
            output.actions.as_slice(),
            [Action::SelectChange { id, selected }] if id == "dropdown" && *selected == 1
        ));
        assert!(!ctx.overlay_open());
    }

    #[test]
    fn dropdown_keyboard_open_and_select_uses_highlighted_option() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        let viewport = Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 360.0,
        };
        ctx.update(dropdown_desc(0), viewport, &mut measurer, &theme);
        let dropdown_id = ctx
            .tree()
            .iter()
            .find_map(|(id, node)| (node.id.as_ref() == "dropdown").then_some(id))
            .expect("dropdown id");
        ctx.interaction_state.focus(dropdown_id);

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Enter,
            modifiers: Modifiers::default(),
        });
        assert!(ctx.overlay_open());

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Down,
            modifiers: Modifiers::default(),
        });
        let output = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Enter,
            modifiers: Modifiers::default(),
        });

        assert!(matches!(
            output.actions.as_slice(),
            [Action::SelectChange { id, selected }] if id == "dropdown" && *selected == 1
        ));
    }
}
