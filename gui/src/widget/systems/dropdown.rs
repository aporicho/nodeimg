use crate::context::{OverlayPlacement, OverlayRequest};
use crate::output::{FrameworkOutput, OutputBuilder, WidgetEvent};
use crate::shell::{AppEvent, Key, MouseButton};
use crate::tree::{hit_test, NodeId, NodeKind, Tree};
use crate::ui;
use crate::widget::atoms::dropdown::{DropdownOptionProps, DropdownProps};
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
            AppEvent::MouseRelease {
                x,
                y,
                button: MouseButton::Left,
            } => {
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
            ui::widget(
                format!("__dropdown_option::{}::{}", dropdown_id, index),
                DropdownOptionProps {
                    label: option.clone(),
                    selected: index == props.selected,
                    highlighted: index == highlighted,
                    disabled: false,
                    size: props.size,
                    density: props.density,
                },
            )
            .build()
        })
        .collect();

    OverlayRequest {
        id: format!("dropdown::{}", dropdown_id),
        anchor_id: format!("{}::field", dropdown_id),
        restore_focus_id: Some(dropdown_id.to_string()),
        placement: OverlayPlacement::BelowStart,
        content: ui::widget(
            format!("{}::popup_group", dropdown_id),
            GroupProps {
                title: props.label.clone().unwrap_or(Cow::Borrowed("Options")),
                content: vec![
                    ui::widget(
                        format!("{}::popup_hint", dropdown_id),
                        LabelProps {
                            text: Cow::Borrowed("Use mouse or Up/Down + Enter"),
                            variant: LabelVariant::Caption,
                            muted: true,
                        },
                    )
                    .build(),
                    ui::widget(
                        format!("{}::popup_list", dropdown_id),
                        ListViewProps {
                            height: 140.0,
                            items,
                        },
                    )
                    .build(),
                ],
            },
        )
        .build(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::{names, IconId};
    use crate::theme::{ControlSize, Density};
    use crate::tree::Desc;
    use crate::widget::frameworks::group::GroupProps;
    use crate::widget::frameworks::list_view::ListViewProps;

    #[test]
    fn dropdown_popup_options_use_icon_markers_not_text_prefixes() {
        let props = DropdownProps {
            label: Some(Cow::Borrowed("Mode")),
            options: vec![Cow::Borrowed("Normal"), Cow::Borrowed("Multiply")],
            selected: 0,
            disabled: false,
            size: ControlSize::Small,
            density: Density::Compact,
        };

        let request = build_request("blend", 1, &props);
        let options = popup_option_props(&request.content);

        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Normal");
        assert!(options[0].selected);
        assert!(!options[0].highlighted);
        assert_eq!(
            options[0].marker_icon().map(IconId::from),
            Some(IconId::from(names::CHECK))
        );

        assert_eq!(options[1].label, "Multiply");
        assert!(!options[1].selected);
        assert!(options[1].highlighted);
        assert_eq!(
            options[1].marker_icon().map(IconId::from),
            Some(IconId::from(names::NAV_ARROW_RIGHT))
        );

        for option in options {
            assert!(!option.label.starts_with('✓'));
            assert!(!option.label.starts_with('›'));
        }
    }

    fn popup_option_props(content: &Desc) -> Vec<&DropdownOptionProps> {
        let Desc::Widget(group_widget) = content else {
            panic!("dropdown popup content should be a group widget");
        };
        let group = group_widget
            .props()
            .as_any()
            .downcast_ref::<GroupProps>()
            .expect("popup content should carry GroupProps");
        let Desc::Widget(list_widget) = &group.content[1] else {
            panic!("popup second child should be a list widget");
        };
        let list = list_widget
            .props()
            .as_any()
            .downcast_ref::<ListViewProps>()
            .expect("popup list should carry ListViewProps");
        list.items
            .iter()
            .map(|item| {
                let Desc::Widget(option_widget) = item else {
                    panic!("list item should be a dropdown option widget");
                };
                option_widget
                    .props()
                    .as_any()
                    .downcast_ref::<DropdownOptionProps>()
                    .expect("list item should carry DropdownOptionProps")
            })
            .collect()
    }
}
