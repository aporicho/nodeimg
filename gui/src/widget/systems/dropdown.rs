use crate::context::{OverlayPlacement, OverlayRequest};
use crate::output::{FrameworkOutput, OutputBuilder, WidgetEvent};
use crate::shell::{AppEvent, Key, MouseButton};
use crate::tree::{hit_test, NodeId, NodeKind, Tree};
use crate::widget::atoms::button::ButtonProps;
use crate::widget::atoms::dropdown::DropdownProps;
use crate::widget::atoms::label::{LabelProps, LabelVariant};
use crate::widget::frameworks::group::GroupProps;
use crate::widget::frameworks::list_view::ListViewProps;
use crate::widget::state::dropdown::{DropdownRuntime, OpenDropdown};
use crate::widget::systems::OverlaySystemCx;
use std::borrow::Cow;

const DROPDOWN_RUNTIME_ID: &str = "__dropdown_runtime";

pub(crate) struct DropdownSystem;

impl DropdownSystem {
    pub fn new() -> Self {
        Self
    }

    pub fn sync_with_tree(
        &mut self,
        tree: &mut Tree,
        overlay_system: &crate::overlay::OverlaySystem,
    ) {
        let open_id = tree
            .ensure_runtime_slot_by_stable_id::<DropdownRuntime>(DROPDOWN_RUNTIME_ID)
            .open
            .as_ref()
            .map(|open| open.id.clone());

        if !overlay_system.is_open() {
            tree.ensure_runtime_slot_by_stable_id::<DropdownRuntime>(DROPDOWN_RUNTIME_ID)
                .open = None;
            return;
        }

        if let Some(open_id) = open_id {
            if dropdown_props(tree, &open_id).is_none() {
                tree.ensure_runtime_slot_by_stable_id::<DropdownRuntime>(DROPDOWN_RUNTIME_ID)
                    .open = None;
            }
        }
    }

    pub fn handle_event(
        &mut self,
        mut cx: OverlaySystemCx<'_>,
        event: &AppEvent,
    ) -> FrameworkOutput {
        if !cx.overlay_open() {
            cx.dropdown_runtime_mut().open = None;
        }

        match *event {
            AppEvent::MouseRelease { x, y, button } if button == MouseButton::Left => {
                if let Some((dropdown_id, index)) = overlay_option_hit(cx.tree(), x, y) {
                    cx.close_overlay();
                    cx.dropdown_runtime_mut().open = None;
                    return selection_output(dropdown_id, index);
                }

                if let Some(dropdown_id) = dropdown_field_hit(cx.tree(), x, y) {
                    self.toggle_dropdown(&mut cx, &dropdown_id);
                    return FrameworkOutput::consumed();
                }
                FrameworkOutput::default()
            }
            AppEvent::KeyPress { key, .. } => self.handle_key(&mut cx, key),
            AppEvent::Unfocused => {
                cx.dropdown_runtime_mut().open = None;
                cx.close_overlay_no_focus_restore();
                FrameworkOutput::default()
            }
            _ => FrameworkOutput::default(),
        }
    }

    fn handle_key(&mut self, cx: &mut OverlaySystemCx<'_>, key: Key) -> FrameworkOutput {
        if cx.dropdown_runtime().open.is_none() {
            let Some(dropdown_id) = focused_dropdown_id(cx.tree(), cx.focused_node()) else {
                return FrameworkOutput::default();
            };
            let Some(props) = dropdown_props(cx.tree(), &dropdown_id) else {
                return FrameworkOutput::default();
            };
            let highlighted = props.selected.min(props.options.len().saturating_sub(1));
            match key {
                Key::Enter | Key::Space | Key::Down => {
                    let request = build_request(&dropdown_id, highlighted, props);
                    cx.open_overlay(request);
                    cx.dropdown_runtime_mut().open = Some(OpenDropdown {
                        id: dropdown_id,
                        highlighted,
                    });
                    return FrameworkOutput::consumed();
                }
                _ => {}
            }
            return FrameworkOutput::default();
        }

        let Some(open) = cx.dropdown_runtime().open.clone() else {
            return FrameworkOutput::default();
        };
        let Some(props) = dropdown_props(cx.tree(), &open.id) else {
            return FrameworkOutput::default();
        };

        match key {
            Key::Up => {
                if open.highlighted > 0 {
                    let highlighted = open.highlighted - 1;
                    let request = build_request(&open.id, highlighted, props);
                    cx.open_overlay(request);
                    cx.dropdown_runtime_mut().open = Some(OpenDropdown {
                        id: open.id,
                        highlighted,
                    });
                }
                FrameworkOutput::consumed()
            }
            Key::Down => {
                if open.highlighted + 1 < props.options.len() {
                    let highlighted = open.highlighted + 1;
                    let request = build_request(&open.id, highlighted, props);
                    cx.open_overlay(request);
                    cx.dropdown_runtime_mut().open = Some(OpenDropdown {
                        id: open.id,
                        highlighted,
                    });
                }
                FrameworkOutput::consumed()
            }
            Key::Enter | Key::Space => {
                let selected = open.highlighted;
                let id = open.id;
                cx.close_overlay();
                cx.dropdown_runtime_mut().open = None;
                selection_output(id, selected)
            }
            _ => FrameworkOutput::default(),
        }
    }

    fn toggle_dropdown(&mut self, cx: &mut OverlaySystemCx<'_>, dropdown_id: &str) {
        if cx
            .dropdown_runtime()
            .open
            .as_ref()
            .is_some_and(|open| open.id == dropdown_id)
        {
            cx.close_overlay();
            cx.dropdown_runtime_mut().open = None;
            return;
        }

        let Some(props) = dropdown_props(cx.tree(), dropdown_id) else {
            return;
        };
        let highlighted = props.selected.min(props.options.len().saturating_sub(1));
        let request = build_request(dropdown_id, highlighted, props);
        cx.open_overlay(request);
        cx.dropdown_runtime_mut().open = Some(OpenDropdown {
            id: dropdown_id.to_string(),
            highlighted,
        });
    }
}

impl Default for DropdownSystem {
    fn default() -> Self {
        Self::new()
    }
}

fn selection_output(id: String, selected: usize) -> FrameworkOutput {
    OutputBuilder::new()
        .widget(WidgetEvent::SelectionChanged { id, selected })
        .finish()
}

fn build_request(dropdown_id: &str, highlighted: usize, props: &DropdownProps) -> OverlayRequest {
    let items = props
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let prefix = if index == props.selected {
                "✓ "
            } else if index == highlighted {
                "› "
            } else {
                "  "
            };
            crate::tree::Desc::Widget(crate::widget::WidgetDesc::new(
                Cow::Owned(format!("__dropdown_option::{}::{}", dropdown_id, index)),
                ButtonProps {
                    label: Cow::Owned(format!("{prefix}{option}")),
                    icon: None,
                    disabled: false,
                    size: props.size,
                    density: props.density,
                },
            ))
        })
        .collect();

    OverlayRequest {
        id: format!("dropdown::{}", dropdown_id),
        anchor_id: format!("{}::field", dropdown_id),
        restore_focus_id: Some(dropdown_id.to_string()),
        placement: OverlayPlacement::BelowStart,
        content: crate::tree::Desc::Widget(crate::widget::WidgetDesc::new(
            Cow::Owned(format!("{}::popup_group", dropdown_id)),
            GroupProps {
                title: props
                    .label
                    .clone()
                    .unwrap_or_else(|| Cow::Borrowed("Options")),
                content: vec![
                    crate::tree::Desc::Widget(crate::widget::WidgetDesc::new(
                        Cow::Owned(format!("{}::popup_hint", dropdown_id)),
                        LabelProps {
                            text: Cow::Borrowed("Use mouse or Up/Down + Enter"),
                            variant: LabelVariant::Caption,
                            muted: true,
                        },
                    )),
                    crate::tree::Desc::Widget(crate::widget::WidgetDesc::new(
                        Cow::Owned(format!("{}::popup_list", dropdown_id)),
                        ListViewProps {
                            height: 140.0,
                            items,
                        },
                    )),
                ],
            },
        )),
        offset_x: 0.0,
        offset_y: 8.0,
        match_anchor_width: true,
        dismiss_on_escape: true,
        dismiss_on_outside_click: true,
        restore_focus_to_anchor: true,
    }
}

fn dropdown_props<'a>(tree: &'a Tree, dropdown_id: &str) -> Option<&'a DropdownProps> {
    let (_, node) = tree
        .iter()
        .find(|(_, node)| node.id.as_ref() == dropdown_id)?;
    let NodeKind::Widget(props) = &node.kind else {
        return None;
    };
    props.as_any().downcast_ref::<DropdownProps>()
}

fn dropdown_field_hit(tree: &Tree, x: f32, y: f32) -> Option<String> {
    let root = tree.root()?;
    hit_test(tree, root, x, y).iter().find_map(|node_id| {
        let node = tree.get(node_id)?;
        let node_id = node.id.as_ref();
        let dropdown_id = node_id.strip_suffix("::field")?;
        dropdown_props(tree, dropdown_id).map(|_| dropdown_id.to_string())
    })
}

fn overlay_option_hit(tree: &Tree, x: f32, y: f32) -> Option<(String, usize)> {
    let root = tree.root()?;
    hit_test(tree, root, x, y).iter().find_map(|node_id| {
        let node = tree.get(node_id)?;
        let node_id = node.id.as_ref();
        let suffix = node_id.strip_prefix("__dropdown_option::")?;
        let (dropdown_id, index) = suffix.rsplit_once("::")?;
        let index = index.parse::<usize>().ok()?;
        Some((dropdown_id.to_string(), index))
    })
}

fn focused_dropdown_id(tree: &Tree, focused: Option<NodeId>) -> Option<String> {
    let focused = focused?;
    let node = tree.get(focused)?;
    let NodeKind::Widget(props) = &node.kind else {
        return None;
    };
    props
        .as_any()
        .downcast_ref::<DropdownProps>()
        .map(|_| node.id.to_string())
}
