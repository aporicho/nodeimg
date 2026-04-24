use super::catalog::{item_specs, PlaygroundItemId};
use super::ids;
use gui::renderer::Rect;
use std::collections::BTreeMap;

const MIN_TILE_WIDTH: f32 = 150.0;
const MIN_TILE_HEIGHT: f32 = 92.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlaygroundPosition {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl From<[f32; 2]> for PlaygroundPosition {
    fn from(value: [f32; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlaygroundSize {
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl From<[f32; 2]> for PlaygroundSize {
    fn from(value: [f32; 2]) -> Self {
        Self {
            width: value[0],
            height: value[1],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaygroundQuality {
    Fast,
    Balanced,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeveloperControlState {
    pub(crate) text_value: String,
    pub(crate) slider_value: f32,
    pub(crate) toggle_enabled: bool,
    pub(crate) checkbox_checked: bool,
    pub(crate) quality: PlaygroundQuality,
    pub(crate) dropdown_selected: usize,
    pub(crate) collapsible_open: bool,
    pub(crate) button_clicks: u32,
}

impl Default for DeveloperControlState {
    fn default() -> Self {
        Self {
            text_value: "Hello nodeimg".to_string(),
            slider_value: 5.0,
            toggle_enabled: true,
            checkbox_checked: true,
            quality: PlaygroundQuality::Balanced,
            dropdown_selected: 0,
            collapsible_open: true,
            button_clicks: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DeveloperModeState {
    placements: BTreeMap<PlaygroundItemId, PlaygroundPosition>,
    sizes: BTreeMap<PlaygroundItemId, PlaygroundSize>,
    drag: Option<PlaygroundDragSession>,
    resize: Option<PlaygroundResizeSession>,
    controls: DeveloperControlState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlaygroundDragSession {
    item: PlaygroundItemId,
    pointer_origin: [f32; 2],
    item_origin: PlaygroundPosition,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlaygroundResizeSession {
    item: PlaygroundItemId,
    pointer_origin: [f32; 2],
    size_origin: PlaygroundSize,
}

impl Default for DeveloperModeState {
    fn default() -> Self {
        let placements = item_specs()
            .iter()
            .map(|spec| (spec.id, spec.default_position.into()))
            .collect();
        let sizes = item_specs()
            .iter()
            .map(|spec| (spec.id, spec.size.into()))
            .collect();

        Self {
            placements,
            sizes,
            drag: None,
            resize: None,
            controls: DeveloperControlState::default(),
        }
    }
}

impl DeveloperModeState {
    pub(crate) fn controls(&self) -> &DeveloperControlState {
        &self.controls
    }

    pub(crate) fn position(&self, item: PlaygroundItemId) -> PlaygroundPosition {
        self.placements
            .get(&item)
            .copied()
            .unwrap_or_else(|| [0.0, 0.0].into())
    }

    pub(crate) fn size(&self, item: PlaygroundItemId) -> PlaygroundSize {
        self.sizes
            .get(&item)
            .copied()
            .unwrap_or_else(|| [MIN_TILE_WIDTH, MIN_TILE_HEIGHT].into())
    }

    pub(crate) fn start_drag(&mut self, id: &str, x: f32, y: f32) -> bool {
        let Some(item) = ids::item_from_tile_id(id) else {
            return false;
        };
        self.drag = Some(PlaygroundDragSession {
            item,
            pointer_origin: [x, y],
            item_origin: self.position(item),
        });
        true
    }

    pub(crate) fn drag(&mut self, id: &str, x: f32, y: f32) -> bool {
        let Some(session) = self.drag else {
            return false;
        };
        if ids::item_from_tile_id(id) != Some(session.item) {
            return false;
        }

        let next = PlaygroundPosition {
            x: (session.item_origin.x + x - session.pointer_origin[0]).max(0.0),
            y: (session.item_origin.y + y - session.pointer_origin[1]).max(0.0),
        };
        self.placements.insert(session.item, next);
        true
    }

    pub(crate) fn end_drag(&mut self, id: &str, x: f32, y: f32) -> bool {
        let moved = self.drag(id, x, y);
        self.drag = None;
        moved
    }

    pub(crate) fn start_resize(&mut self, id: &str, x: f32, y: f32) -> bool {
        let Some(item) = ids::item_from_tile_resize_handle_id(id) else {
            return false;
        };
        self.resize = Some(PlaygroundResizeSession {
            item,
            pointer_origin: [x, y],
            size_origin: self.size(item),
        });
        true
    }

    pub(crate) fn resize(&mut self, id: &str, x: f32, y: f32) -> bool {
        let Some(session) = self.resize else {
            return false;
        };
        if ids::item_from_tile_resize_handle_id(id) != Some(session.item) {
            return false;
        }

        let next = PlaygroundSize {
            width: (session.size_origin.width + x - session.pointer_origin[0]).max(MIN_TILE_WIDTH),
            height: (session.size_origin.height + y - session.pointer_origin[1])
                .max(MIN_TILE_HEIGHT),
        };
        self.sizes.insert(session.item, next);
        true
    }

    pub(crate) fn end_resize(&mut self, id: &str, x: f32, y: f32) -> bool {
        let resized = self.resize(id, x, y);
        self.resize = None;
        resized
    }

    pub(crate) fn apply_click(&mut self, id: &str) -> bool {
        match id {
            ids::CONTROL_BUTTON_ID => {
                self.controls.button_clicks = self.controls.button_clicks.saturating_add(1);
                true
            }
            ids::CONTROL_TOGGLE_ID => {
                self.controls.toggle_enabled = !self.controls.toggle_enabled;
                true
            }
            ids::CONTROL_CHECKBOX_ID => {
                self.controls.checkbox_checked = !self.controls.checkbox_checked;
                true
            }
            ids::CONTROL_RADIO_FAST_ID => {
                self.controls.quality = PlaygroundQuality::Fast;
                true
            }
            ids::CONTROL_RADIO_BALANCED_ID => {
                self.controls.quality = PlaygroundQuality::Balanced;
                true
            }
            ids::CONTAINER_COLLAPSIBLE_ID => {
                self.controls.collapsible_open = !self.controls.collapsible_open;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn apply_text_change(&mut self, id: &str, value: String) -> bool {
        if id != ids::CONTROL_TEXT_INPUT_ID {
            return false;
        }
        self.controls.text_value = value;
        true
    }

    pub(crate) fn apply_number_change(&mut self, id: &str, value: f32) -> bool {
        if id != ids::CONTROL_NUMBER_INPUT_ID {
            return false;
        }
        self.controls.slider_value = value.clamp(0.0, 10.0);
        true
    }

    pub(crate) fn apply_selection_change(&mut self, id: &str, selected: usize) -> bool {
        if id != ids::CONTROL_DROPDOWN_ID {
            return false;
        }
        self.controls.dropdown_selected = selected.min(2);
        true
    }

    pub(crate) fn reset_slider(&mut self) {
        self.controls.slider_value = 5.0;
    }

    pub(crate) fn update_slider_from_pointer(&mut self, track_rect: Rect, x: f32) {
        self.controls.slider_value = slider_value_from_x(track_rect, x, 0.0, 10.0, 0.1);
    }
}

fn slider_value_from_x(track_rect: Rect, x: f32, min: f32, max: f32, step: f32) -> f32 {
    let width = track_rect.w.max(1.0);
    let ratio = ((x - track_rect.x) / width).clamp(0.0, 1.0);
    let raw = min + (max - min) * ratio;
    let stepped = if step > 0.0 {
        ((raw - min) / step).round() * step + min
    } else {
        raw
    };
    stepped.clamp(min, max)
}
