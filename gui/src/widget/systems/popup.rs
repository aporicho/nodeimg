use crate::shell::{AppEvent, MouseButton};
use crate::tree::layout::{BoxStyle, Position, Size};
use crate::tree::{Desc, Tree};
use crate::widget::state::InteractionStore;
use std::borrow::Cow;

const OVERLAY_ROOT_ID: &str = "__overlay_root";

#[derive(Debug, Clone, Copy)]
pub enum OverlayPlacement {
    BelowStart,
}

pub struct OverlayRequest {
    pub id: String,
    pub anchor_id: String,
    pub restore_focus_id: Option<String>,
    pub placement: OverlayPlacement,
    pub content: Desc,
    pub offset_x: f32,
    pub offset_y: f32,
    pub match_anchor_width: bool,
    pub dismiss_on_escape: bool,
    pub dismiss_on_outside_click: bool,
    pub restore_focus_to_anchor: bool,
}

struct OverlayState {
    request: OverlayRequest,
    last_x: f32,
    last_y: f32,
    last_width: Option<f32>,
}

pub(crate) struct PopupSystem {
    current: Option<OverlayState>,
}

impl PopupSystem {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn open(&mut self, tree: &Tree, request: OverlayRequest) {
        let (x, y, width) = anchor_layout(tree, &request)
            .map(|(rect_x, rect_y, rect_w, rect_h)| {
                placement_xy(&request, rect_x, rect_y, rect_w, rect_h)
            })
            .unwrap_or((0.0, 0.0, None));
        self.current = Some(OverlayState {
            request,
            last_x: x,
            last_y: y,
            last_width: width,
        });
    }

    pub fn close(&mut self, tree: &Tree, interaction: &mut InteractionStore) {
        let Some(state) = self.current.take() else {
            return;
        };
        if state.request.restore_focus_to_anchor {
            let restore_id = state
                .request
                .restore_focus_id
                .as_deref()
                .unwrap_or(&state.request.anchor_id);
            if let Some(node_id) = find_node_id_by_str(tree, restore_id) {
                interaction.focus(node_id);
            }
        }
    }

    pub fn close_no_focus_restore(&mut self) {
        self.current = None;
    }

    pub fn compose_desc(
        &mut self,
        tree: &Tree,
        base_desc: Desc,
        viewport: crate::renderer::Rect,
    ) -> Desc {
        let Some(overlay) = self.overlay_desc(tree, viewport) else {
            return base_desc;
        };

        Desc::Container {
            id: Cow::Borrowed("__context_root"),
            style: BoxStyle {
                width: Size::Fixed(viewport.w),
                height: Size::Fixed(viewport.h),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![base_desc, overlay],
        }
    }

    pub fn handle_event(
        &mut self,
        tree: &Tree,
        interaction: &mut InteractionStore,
        event: &AppEvent,
    ) -> bool {
        let Some(state) = &self.current else {
            return false;
        };

        match *event {
            AppEvent::KeyPress {
                key: crate::shell::Key::Escape,
                ..
            } if state.request.dismiss_on_escape => {
                self.close(tree, interaction);
                true
            }
            AppEvent::MousePress { x, y, button }
                if button == MouseButton::Left && state.request.dismiss_on_outside_click =>
            {
                if !self.hit_overlay(tree, x, y) {
                    self.close_no_focus_restore();
                }
                false
            }
            _ => false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.current.is_some()
    }

    fn overlay_desc(&mut self, tree: &Tree, viewport: crate::renderer::Rect) -> Option<Desc> {
        let state = self.current.as_mut()?;

        if let Some((rect_x, rect_y, rect_w, rect_h)) = anchor_layout(tree, &state.request) {
            let (x, y, width) = placement_xy(&state.request, rect_x, rect_y, rect_w, rect_h);
            state.last_x = x;
            state.last_y = y;
            state.last_width = width;
        }

        Some(Desc::Container {
            id: Cow::Borrowed(OVERLAY_ROOT_ID),
            style: BoxStyle {
                position: Position::Absolute { x: 0.0, y: 0.0 },
                width: Size::Fixed(viewport.w),
                height: Size::Fixed(viewport.h),
                hittable: Some(false),
                ..BoxStyle::default()
            },
            decoration: None,
            children: vec![Desc::Container {
                id: Cow::Owned(format!("__overlay::{}", state.request.id)),
                style: BoxStyle {
                    position: Position::Absolute {
                        x: state.last_x,
                        y: state.last_y,
                    },
                    width: state.last_width.map(Size::Fixed).unwrap_or(Size::Auto),
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                decoration: None,
                children: vec![clone_desc(&state.request.content)],
            }],
        })
    }

    fn hit_overlay(&self, tree: &Tree, x: f32, y: f32) -> bool {
        let Some(root) = tree.root() else {
            return false;
        };
        let prefix = self
            .current
            .as_ref()
            .map(|state| format!("__overlay::{}", state.request.id));
        let Some(prefix) = prefix else {
            return false;
        };
        crate::tree::hit_test(tree, root, x, y)
            .iter()
            .filter_map(|node_id| tree.get(node_id))
            .any(|node| node.id.as_ref().starts_with(&prefix))
    }
}

impl Default for PopupSystem {
    fn default() -> Self {
        Self::new()
    }
}

fn anchor_layout(tree: &Tree, request: &OverlayRequest) -> Option<(f32, f32, f32, f32)> {
    let node_id = find_node_id_by_str(tree, &request.anchor_id)?;
    let node = tree.get(node_id)?;
    Some((node.rect.x, node.rect.y, node.rect.w, node.rect.h))
}

fn placement_xy(
    request: &OverlayRequest,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> (f32, f32, Option<f32>) {
    match request.placement {
        OverlayPlacement::BelowStart => (
            x + request.offset_x,
            y + h + request.offset_y,
            request.match_anchor_width.then_some(w),
        ),
    }
}

fn find_node_id_by_str(tree: &Tree, node_id: &str) -> Option<crate::tree::NodeId> {
    tree.iter()
        .find_map(|(id, node)| (node.id.as_ref() == node_id).then_some(id))
}

fn clone_desc(desc: &Desc) -> Desc {
    match desc {
        Desc::Container {
            id,
            style,
            decoration,
            children,
        } => Desc::Container {
            id: id.clone(),
            style: style.clone(),
            decoration: decoration.clone(),
            children: children.iter().map(clone_desc).collect(),
        },
        Desc::Leaf { id, style, kind } => Desc::Leaf {
            id: id.clone(),
            style: style.clone(),
            kind: kind.clone(),
        },
        Desc::Widget { id, props } => Desc::Widget {
            id: id.clone(),
            props: props.clone_box(),
        },
    }
}
