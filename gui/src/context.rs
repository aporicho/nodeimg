use std::sync::Arc;

use crate::event::gesture_adapter;
use crate::event::router;
use crate::gesture::{Gesture, GestureSession, GestureSessionUpdate};
use crate::interaction::InteractionState;
use crate::panel::PanelDeclaration;
use crate::renderer::{Rect, Renderer, TextMeasurer};
use crate::runtime::{
    ResourceRegistry, RuntimeEventCx, RuntimeEventResult, RuntimeSyncCx, RuntimeSystems,
};
use crate::shell::AppEvent;
use crate::theme::Theme;
use crate::tree::layout::TextureHandle;
use crate::tree::{hit_test, layout, paint, reconcile, Desc, HitChain, NodeId, NodeKind, Tree};
use crate::widget::props::WidgetBuildCx;

pub use crate::output::{
    FrameworkOutput, GuiEvent, OverlayEvent, PanelEvent, PlatformEffect, WidgetEvent,
};
pub use crate::overlay::{OverlayPlacement, OverlayRequest};

/// GUI 中心对象。持有统一的控件树与框架级交互 session。
pub struct Context {
    pub(crate) tree: Tree,
    gesture_session: GestureSession,
    interaction: InteractionState,
    pub(crate) systems: RuntimeSystems,
    pub(crate) last_theme_revision: Option<u64>,
    pub(crate) resources: ResourceRegistry,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImeRequest {
    pub allowed: bool,
    pub cursor_area: Option<Rect>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            tree: Tree::new(),
            gesture_session: GestureSession::new(),
            interaction: InteractionState::new(),
            systems: RuntimeSystems::new(),
            last_theme_revision: None,
            resources: ResourceRegistry::new(),
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
        let desc = self.systems.compose_desc(&self.tree, desc, root_rect);
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
        self.interaction.sync_with_tree(&self.tree);
        self.systems.sync_with_tree(RuntimeSyncCx {
            tree: &mut self.tree,
            interaction: &self.interaction,
            measurer,
            theme,
        });
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
                Some(&self.interaction),
                Some(self.systems.text_input_store()),
                Some(self.resources.textures()),
                theme,
            );
        }
    }

    pub fn register_texture(&mut self, handle: TextureHandle, view: Arc<wgpu::TextureView>) {
        self.resources.register_texture(handle, view);
    }

    pub fn open_overlay(&mut self, request: OverlayRequest) {
        self.systems.open_overlay(&self.tree, request);
    }

    pub fn close_overlay(&mut self) {
        self.systems
            .close_overlay(&self.tree, &mut self.interaction);
    }

    pub fn overlay_open(&self) -> bool {
        self.systems.overlay_open()
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        router::handle_event(self, event)
    }

    pub fn ime_request(&self) -> ImeRequest {
        self.systems
            .ime_request(&self.tree, self.interaction.focused())
    }

    pub fn paste_focused_text(&mut self, text: &str) -> FrameworkOutput {
        self.systems
            .paste_focused_text(&self.tree, self.interaction.focused(), text)
    }

    pub fn panel_root(&mut self, viewport: Rect, panels: Vec<PanelDeclaration>) -> Desc {
        crate::panel::panel_root(&mut self.tree, viewport, panels)
    }

    pub fn handle_panel_event(&mut self, event: &PanelEvent) -> bool {
        crate::panel::event::apply_panel_event(&mut self.tree, event)
    }

    pub fn sync_canvas_node_layouts(
        &mut self,
        identities: &[crate::canvas::CanvasNodeIdentity],
    ) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.sync_canvas_node_layouts(identities)
    }

    pub fn export_canvas_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.export_canvas_node_layouts()
    }

    pub fn import_canvas_node_layouts(&mut self, layouts: &[crate::canvas::CanvasNodeLayout]) {
        self.tree.import_canvas_node_layouts(layouts);
    }

    pub fn move_canvas_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        self.tree.move_canvas_node_by(owner_id, dx, dy)
    }

    pub fn canvas_port_group_view(
        &self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> crate::canvas::CanvasPortGroupView {
        self.tree.canvas_port_group_view(owner_id, side)
    }

    pub fn toggle_canvas_port_group(
        &mut self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> bool {
        self.tree.toggle_canvas_port_group(owner_id, side)
    }

    pub fn select_canvas_node(&mut self, owner_id: &str) -> bool {
        self.tree.select_canvas_node(owner_id)
    }

    pub fn clear_canvas_selection(&mut self) {
        self.tree.clear_canvas_selection();
    }

    pub fn is_canvas_node_selected(&self, owner_id: &str) -> bool {
        self.tree.is_canvas_node_selected(owner_id)
    }

    pub fn pending_canvas_connection(&self) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.pending_canvas_connection()
    }

    pub fn begin_pending_canvas_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.tree
            .begin_pending_canvas_connection(from_port_id, cursor_canvas)
    }

    pub fn update_pending_canvas_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        self.tree.update_pending_canvas_connection(cursor_canvas)
    }

    pub fn end_pending_canvas_connection(
        &mut self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.end_pending_canvas_connection()
    }

    pub fn cancel_pending_canvas_connection(&mut self) -> bool {
        self.tree.cancel_pending_canvas_connection()
    }

    pub fn hovered_canvas_port_id(&self) -> Option<String> {
        self.tree.hovered_canvas_port_id()
    }

    pub fn set_hovered_canvas_port(&mut self, port_id: Option<&str>) -> bool {
        self.tree.set_hovered_canvas_port(port_id)
    }

    pub fn export_panel_layouts(&self) -> Vec<crate::panel::PanelLayout> {
        self.tree.export_panel_layouts()
    }

    pub fn import_panel_layouts(&mut self, layouts: &[crate::panel::PanelLayout]) {
        self.tree.import_panel_layouts(layouts);
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

    pub fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.tree
            .iter()
            .find_map(|(node_id, node)| (node.id.as_ref() == id).then_some(node_id))
    }

    pub fn node_exists(&self, id: &str) -> bool {
        self.node_id_by_name(id).is_some()
    }

    pub fn node_rect(&self, id: &str) -> Option<Rect> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.node_rect_by_node(node_id))
    }

    pub fn node_rect_by_node(&self, node_id: NodeId) -> Option<Rect> {
        self.tree.get(node_id).map(|node| node.rect)
    }

    pub fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.tree.get(node_id).map(|node| node.id.as_ref())
    }

    pub fn node_scroll_offset(&self, id: &str) -> Option<f32> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.tree.get(node_id))
            .map(|node| node.scroll_offset())
    }

    pub fn node_has_gesture(&self, node_id: NodeId, gesture: Gesture) -> bool {
        self.tree
            .get(node_id)
            .map(|node| node.style.gestures.contains(&gesture))
            .unwrap_or(false)
    }

    pub fn node_root_widget_type(&self, node_id: NodeId) -> Option<&'static str> {
        let node_name = self.node_name(node_id)?;
        let root_name = node_name.split("::").next()?;
        let root_id = self.node_id_by_name(root_name)?;
        let root_node = self.tree.get(root_id)?;
        let NodeKind::Widget(props) = &root_node.kind else {
            return None;
        };
        Some(props.widget_type())
    }

    pub fn node_is_text_input_field(&self, node_id: NodeId) -> bool {
        self.node_name(node_id)
            .is_some_and(|id| id.ends_with("::field"))
            && self.node_root_widget_type(node_id) == Some("TextInput")
    }

    pub fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub fn focused_widget_id(&self) -> Option<&str> {
        self.node_name_for(self.focused_node())
    }

    pub fn hovered_node(&self) -> Option<NodeId> {
        self.interaction.hovered()
    }

    pub fn hovered_widget_id(&self) -> Option<&str> {
        self.node_name_for(self.hovered_node())
    }

    pub fn captured_node(&self) -> Option<NodeId> {
        self.interaction.captured()
    }

    pub fn captured_widget_id(&self) -> Option<&str> {
        self.node_name_for(self.captured_node())
    }

    pub fn request_focus(&mut self, node_id: NodeId) {
        self.interaction.focus(node_id);
    }

    pub fn clear_focus(&mut self) {
        self.interaction.blur();
    }

    pub(crate) fn handle_interaction_event(&mut self, event: &AppEvent) {
        self.interaction.handle_event(&self.tree, event);
    }

    pub(crate) fn handle_runtime_pre_gesture_event(
        &mut self,
        event: &AppEvent,
    ) -> RuntimeEventResult {
        self.systems.handle_pre_gesture_event(
            RuntimeEventCx {
                tree: &mut self.tree,
                interaction: &mut self.interaction,
            },
            event,
        )
    }

    pub(crate) fn handle_gesture_event(&mut self, event: &AppEvent) -> FrameworkOutput {
        let update = self.handle_gesture_session_event(event);
        let output = update
            .signal
            .as_ref()
            .map(|signal| gesture_adapter::gesture_signal_output(&self.tree, signal))
            .unwrap_or_default();
        output.with_consumed(update.consumed)
    }

    pub(crate) fn handle_gesture_session_event(
        &mut self,
        event: &AppEvent,
    ) -> GestureSessionUpdate {
        self.gesture_session.handle_event(&self.tree, event)
    }

    pub(crate) fn cancel_gesture(&mut self) {
        self.gesture_session.cancel();
    }

    fn node_name_for(&self, node_id: Option<NodeId>) -> Option<&str> {
        node_id.and_then(|id| self.node_name(id))
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
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
    use crate::widget::atoms::slider::SliderProps;
    use crate::widget::atoms::text_input::TextInputProps;
    use crate::widget::atoms::toggle::ToggleProps;
    use crate::widget::frameworks::panel::PanelProps;
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
                    label: Some(Cow::Borrowed("Prompt")),
                    value: Cow::Owned(value.to_string()),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
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
                    size: Default::default(),
                    density: Default::default(),
                }),
            }],
        }
    }

    fn toggle_desc() -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(120.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("toggle"),
                props: Box::new(ToggleProps {
                    label: Some(Cow::Borrowed("Grid")),
                    value: true,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            }],
        }
    }

    fn slider_desc() -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(120.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("slider"),
                props: Box::new(SliderProps {
                    label: Some(Cow::Borrowed("Radius")),
                    min: 0.0,
                    max: 10.0,
                    step: 1.0,
                    value: 5.0,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            }],
        }
    }

    fn panel_desc() -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(180.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("panel"),
                props: Box::new(PanelProps {
                    title: Cow::Borrowed("Panel"),
                    rect: Rect {
                        x: 20.0,
                        y: 20.0,
                        w: 180.0,
                        h: 100.0,
                    },
                    z_index: 0,
                    min_size: [120.0, 80.0],
                    titlebar_visible: true,
                    draggable: true,
                    resizable: true,
                    closable: false,
                    content: vec![],
                }),
            }],
        }
    }

    fn panel_with_input_desc() -> Desc {
        Desc::Container {
            id: Cow::Borrowed("root"),
            style: BoxStyle {
                width: Size::Fixed(320.0),
                height: Size::Fixed(240.0),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Widget {
                id: Cow::Borrowed("panel"),
                props: Box::new(PanelProps {
                    title: Cow::Borrowed("Panel"),
                    rect: Rect {
                        x: 20.0,
                        y: 20.0,
                        w: 240.0,
                        h: 160.0,
                    },
                    z_index: 0,
                    min_size: [120.0, 80.0],
                    titlebar_visible: true,
                    draggable: true,
                    resizable: true,
                    closable: false,
                    content: vec![Desc::Widget {
                        id: Cow::Borrowed("input"),
                        props: Box::new(TextInputProps {
                            label: Some(Cow::Borrowed("Prompt")),
                            value: Cow::Borrowed("hello"),
                            disabled: false,
                            size: Default::default(),
                            density: Default::default(),
                        }),
                    }],
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
                size: Default::default(),
                density: Default::default(),
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
                    label: Some(Cow::Borrowed("Radius")),
                    value,
                    min: 0.0,
                    max: 10.0,
                    step: 0.5,
                    precision: 2,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
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
                    label: Some(Cow::Borrowed("Mode")),
                    options: vec![
                        Cow::Borrowed("Normal"),
                        Cow::Borrowed("Multiply"),
                        Cow::Borrowed("Screen"),
                    ],
                    selected,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
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
        ctx.node_rect("input::field")
            .expect("text input field rect")
    }

    fn button_rect(ctx: &Context) -> Rect {
        ctx.node_rect("button").expect("button rect")
    }

    fn node_rect(ctx: &Context, node_name: &str) -> Rect {
        ctx.node_rect(node_name).expect("node rect")
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
    fn context_query_api_finds_nodes_without_exposing_tree() {
        let ctx = test_context("hello");

        let field_id = ctx
            .node_id_by_name("input::field")
            .expect("text input field id");

        assert!(ctx.node_exists("input::field"));
        assert!(!ctx.node_exists("missing"));
        assert_eq!(ctx.node_name(field_id), Some("input::field"));
        let by_name = ctx.node_rect("input::field").expect("rect by name");
        let by_node = ctx.node_rect_by_node(field_id).expect("rect by node");
        assert_eq!(by_name.x, by_node.x);
        assert_eq!(by_name.y, by_node.y);
        assert_eq!(by_name.w, by_node.w);
        assert_eq!(by_name.h, by_node.h);
        assert_eq!(ctx.node_root_widget_type(field_id), Some("TextInput"));
        assert!(ctx.node_is_text_input_field(field_id));
        assert!(ctx.node_rect("missing").is_none());
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
            outcome.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::TextChanged { id, value })]
                if id == "input" && value.is_empty()
        ));
        assert!(outcome.consumed);
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
        updated.controls.medium_regular.height = 52.0;

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

        let before_height = ctx.node_rect("input::field").expect("text input field").h;

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

        let after_height = ctx.node_rect("input::field").expect("text input field").h;

        assert_eq!(before_height, dark.controls.medium_regular.height);
        assert_eq!(after_height, updated.controls.medium_regular.height);
    }

    #[test]
    fn context_emits_click_through_gesture_session() {
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
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::Click { id })] if id == "button"
        ));
        assert!(output.consumed);
    }

    #[test]
    fn mouse_move_updates_hovered_widget_through_context_facade() {
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

        let _ = ctx.handle_event(&AppEvent::MouseMove {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
        });

        assert_eq!(ctx.hovered_widget_id(), Some("button"));
    }

    #[test]
    fn mouse_press_and_release_updates_capture_through_context_facade() {
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
        assert_eq!(ctx.captured_widget_id(), Some("button"));

        let _ = ctx.handle_event(&AppEvent::MouseRelease {
            x,
            y,
            button: MouseButton::Left,
        });
        assert_eq!(ctx.captured_widget_id(), None);
    }

    #[test]
    fn text_input_consumed_pointer_press_cancels_active_gesture_session() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_with_input_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 240.0,
            },
            &mut measurer,
            &theme,
        );
        let titlebar = node_rect(&ctx, "panel::titlebar");
        let field = node_rect(&ctx, "input::field");

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: titlebar.x + titlebar.w * 0.5,
            y: titlebar.y + titlebar.h * 0.5,
            button: MouseButton::Left,
        });
        assert!(ctx.gesture_session.is_active());

        let output = ctx.handle_event(&AppEvent::MousePress {
            x: field.x + field.w * 0.5,
            y: field.y + field.h * 0.5,
            button: MouseButton::Left,
        });

        assert!(output.consumed);
        assert!(!ctx.gesture_session.is_active());
        assert_eq!(ctx.focused_widget_id(), Some("input"));
    }

    #[test]
    fn unfocused_cancels_active_gesture_session() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let titlebar = node_rect(&ctx, "panel::titlebar");

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x: titlebar.x + titlebar.w * 0.5,
            y: titlebar.y + titlebar.h * 0.5,
            button: MouseButton::Left,
        });
        assert!(ctx.gesture_session.is_active());

        let output = ctx.handle_event(&AppEvent::Unfocused);

        assert!(!output.consumed);
        assert!(!ctx.gesture_session.is_active());
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
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DoubleClick { id })] if id == "button"
        ));
        assert!(output.consumed);
    }

    #[test]
    fn widget_event_canonicalizes_child_target_to_widget_id() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            toggle_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "toggle::track");

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
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::Click { id })] if id == "toggle"
        ));
    }

    #[test]
    fn context_emits_widget_drag_events_from_gesture_signal() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            slider_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 120.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "slider::track");
        let x = rect.x + rect.w * 0.5;
        let y = rect.y + rect.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let start = ctx.handle_event(&AppEvent::MouseMove { x: x + 20.0, y });
        let move_output = ctx.handle_event(&AppEvent::MouseMove { x: x + 30.0, y });
        let end = ctx.handle_event(&AppEvent::MouseRelease {
            x: x + 30.0,
            y,
            button: MouseButton::Left,
        });

        assert!(matches!(
            start.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DragStart { id, .. })] if id == "slider"
        ));
        assert!(matches!(
            move_output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DragMove { id, .. })] if id == "slider"
        ));
        assert!(matches!(
            end.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::DragEnd { id, .. })] if id == "slider"
        ));
    }

    #[test]
    fn context_emits_panel_drag_events_from_gesture_signal() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "panel::titlebar");
        let x = rect.x + rect.w * 0.5;
        let y = rect.y + rect.h * 0.5;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let start = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 20.0,
            y: y + 4.0,
        });
        let move_output = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 30.0,
            y: y + 8.0,
        });
        let end = ctx.handle_event(&AppEvent::MouseRelease {
            x: x + 30.0,
            y: y + 8.0,
            button: MouseButton::Left,
        });

        assert!(matches!(
            start.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::DragStart { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            move_output.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::DragMove { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            end.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::DragEnd { id, .. })] if id == "panel"
        ));
    }

    #[test]
    fn context_emits_panel_resize_events_from_gesture_signal() {
        let mut ctx = Context::new();
        let mut measurer = TextMeasurer::new();
        let theme = dark_theme();
        ctx.update(
            panel_desc(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
            },
            &mut measurer,
            &theme,
        );
        let rect = node_rect(&ctx, "panel");
        let x = rect.x + rect.w - 1.0;
        let y = rect.y + rect.h - 1.0;

        let _ = ctx.handle_event(&AppEvent::MousePress {
            x,
            y,
            button: MouseButton::Left,
        });
        let start = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 12.0,
            y: y + 12.0,
        });
        let move_output = ctx.handle_event(&AppEvent::MouseMove {
            x: x + 20.0,
            y: y + 20.0,
        });
        let end = ctx.handle_event(&AppEvent::MouseRelease {
            x: x + 20.0,
            y: y + 20.0,
            button: MouseButton::Left,
        });

        assert!(matches!(
            start.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::ResizeStart { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            move_output.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::ResizeMove { id, .. })] if id == "panel"
        ));
        assert!(matches!(
            end.events.as_slice(),
            [GuiEvent::Panel(PanelEvent::ResizeEnd { id, .. })] if id == "panel"
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
            ctx.node_name(node_id)
                .is_some_and(|id| id.starts_with("__overlay::test_popup"))
        }));
    }

    #[test]
    fn overlay_can_be_placed_at_pointer_position() {
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
            id: "point_popup".to_string(),
            anchor_id: "canvas_root".to_string(),
            restore_focus_id: None,
            placement: OverlayPlacement::AtPoint { x: 44.0, y: 52.0 },
            content: popup_content_desc(),
            offset_x: 3.0,
            offset_y: 4.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: false,
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

        let popup_rect = node_rect(&ctx, "__overlay::point_popup");
        assert_eq!(popup_rect.x, 47.0);
        assert_eq!(popup_rect.y, 56.0);
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
        ctx.clear_focus();

        let _ = ctx.handle_event(&AppEvent::KeyPress {
            key: Key::Escape,
            modifiers: Modifiers::default(),
        });

        assert!(!ctx.overlay_open());
        assert_eq!(ctx.focused_widget_id(), Some("button"));
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

        let output = ctx.handle_event(&AppEvent::ScrollPixel {
            x: rect.x + rect.w * 0.5,
            y: rect.y + rect.h * 0.5,
            delta_x: 0.0,
            delta_y: -24.0,
        });

        assert!(ctx.node_scroll_offset("scroll").expect("scroll node") > 0.0);
        assert!(output.consumed);
        assert!(output.events.is_empty());
    }

    #[test]
    fn unhandled_pointer_event_is_not_consumed() {
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

        let output = ctx.handle_event(&AppEvent::ScrollPixel {
            x: 300.0,
            y: 100.0,
            delta_x: 0.0,
            delta_y: -24.0,
        });

        assert!(!output.consumed);
        assert!(output.events.is_empty());
        assert!(output.effects.is_empty());
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
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::NumberChanged { id, value })]
                if id == "number" && (*value - 2.0).abs() < 0.0001
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
        assert!(output.events.is_empty());
        assert_eq!(
            ctx.systems
                .text_input_store()
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
            ctx.systems
                .text_input_store()
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
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::NumberChanged { id, value })]
                if id == "number" && (*value - 3.0).abs() < 0.0001
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
            ctx.systems
                .text_input_store()
                .runtime("number")
                .unwrap()
                .editor()
                .text(),
            "a1.50"
        );
        assert_eq!(
            ctx.systems
                .text_input_store()
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
        assert!(matches!(
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::SelectionChanged { id, selected })]
                if id == "dropdown" && *selected == 1
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
        let dropdown_id = ctx.node_id_by_name("dropdown").expect("dropdown id");
        ctx.request_focus(dropdown_id);

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
            output.events.as_slice(),
            [GuiEvent::Widget(WidgetEvent::SelectionChanged { id, selected })]
                if id == "dropdown" && *selected == 1
        ));
    }
}
