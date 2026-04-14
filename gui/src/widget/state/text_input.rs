use std::collections::HashMap;

use crate::renderer::{Point, Rect, TextMeasurer};
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::text_input::{
    TextInputProps, TEXT_INPUT_FIELD_HEIGHT, TEXT_INPUT_FIELD_PADDING_X, TEXT_INPUT_VALUE_FONT_SIZE,
};
use crate::widget::TextEditState;

#[derive(Clone, Debug)]
struct PreeditState {
    text: String,
    range: (usize, usize),
    caret: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug)]
struct PreeditLayout {
    start_x: f32,
    width: f32,
    caret_x: f32,
}

pub struct TextInputRuntime {
    editor: TextEditState,
    field_rect: Rect,
    content_rect: Rect,
    text_origin: Point,
    text_height: f32,
    scroll_x: f32,
    caret_stops: Vec<(usize, f32)>,
    preedit: Option<PreeditState>,
    preedit_layout: Option<PreeditLayout>,
}

impl TextInputRuntime {
    fn new(value: &str) -> Self {
        Self {
            editor: TextEditState::new(value),
            field_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: TEXT_INPUT_FIELD_HEIGHT,
            },
            content_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: TEXT_INPUT_FIELD_HEIGHT,
            },
            text_origin: Point { x: 0.0, y: 0.0 },
            text_height: TEXT_INPUT_VALUE_FONT_SIZE,
            scroll_x: 0.0,
            caret_stops: vec![(0, 0.0)],
            preedit: None,
            preedit_layout: None,
        }
    }

    pub fn editor(&self) -> &TextEditState {
        &self.editor
    }

    pub fn editor_mut(&mut self) -> &mut TextEditState {
        &mut self.editor
    }

    pub fn has_preedit(&self) -> bool {
        self.preedit.is_some()
    }

    pub fn preedit_range(&self) -> Option<(usize, usize)> {
        self.preedit.as_ref().map(|preedit| preedit.range)
    }

    pub fn preedit_text(&self) -> Option<&str> {
        self.preedit.as_ref().map(|preedit| preedit.text.as_str())
    }

    pub fn preedit_start_x(&self) -> Option<f32> {
        self.preedit_layout
            .map(|layout| self.visible_content_x(layout.start_x))
    }

    pub fn preedit_width(&self) -> Option<f32> {
        self.preedit_layout.map(|layout| layout.width)
    }

    pub fn preedit_underline_rect(&self) -> Option<Rect> {
        let layout = self.preedit_layout?;
        Some(Rect {
            x: self.visible_content_x(layout.start_x),
            y: self.text_origin.y + self.text_height + 1.0,
            w: layout.width.max(1.0),
            h: 1.0,
        })
    }

    pub fn clip_rect(&self) -> Rect {
        self.content_rect
    }

    pub fn text_draw_origin(&self) -> Point {
        Point {
            x: self.visible_content_x(0.0),
            y: self.text_origin.y,
        }
    }

    pub fn clear_preedit(&mut self) {
        self.preedit = None;
        self.preedit_layout = None;
        self.clamp_scroll();
    }

    pub fn set_preedit(&mut self, text: &str, caret: Option<(usize, usize)>) {
        if text.is_empty() {
            self.clear_preedit();
            return;
        }

        let range = self
            .preedit
            .as_ref()
            .map(|preedit| preedit.range)
            .or_else(|| self.editor.selection_range())
            .unwrap_or((self.editor.cursor(), self.editor.cursor()));
        self.preedit = Some(PreeditState {
            text: text.to_string(),
            range,
            caret,
        });
    }

    pub fn set_caret_from_x(&mut self, x: f32) {
        let nearest = self.nearest_index_for_screen_x(x);
        self.editor.move_to(nearest);
        self.ensure_cursor_visible();
    }

    pub fn select_to_x(&mut self, x: f32) {
        let nearest = self.nearest_index_for_screen_x(x);
        self.editor.select_to(nearest);
        self.ensure_cursor_visible();
    }

    pub fn caret_rect(&self) -> Rect {
        let caret_x = self
            .preedit_layout
            .map(|layout| layout.caret_x)
            .unwrap_or_else(|| self.caret_offset(self.editor.cursor()));
        Rect {
            x: self.visible_content_x(caret_x),
            y: self.text_origin.y,
            w: 1.0,
            h: self.text_height.max(1.0),
        }
    }

    pub fn selection_rect(&self) -> Option<Rect> {
        if self.preedit.is_some() {
            return None;
        }
        let (start, end) = self.editor.selection_range()?;
        let start_x = self.caret_offset(start);
        let end_x = self.caret_offset(end);
        Some(Rect {
            x: self.visible_content_x(start_x),
            y: self.text_origin.y,
            w: (end_x - start_x).max(0.0),
            h: self.text_height.max(1.0),
        })
    }

    fn sync_layout(
        &mut self,
        field_rect: Rect,
        value_rect: Option<Rect>,
        measurer: &mut TextMeasurer,
    ) {
        self.field_rect = field_rect;
        self.text_height = measurer
            .measure("Mg", TEXT_INPUT_VALUE_FONT_SIZE)
            .1
            .max(TEXT_INPUT_VALUE_FONT_SIZE);
        self.content_rect = Rect {
            x: field_rect.x + TEXT_INPUT_FIELD_PADDING_X,
            y: field_rect.y,
            w: (field_rect.w - TEXT_INPUT_FIELD_PADDING_X * 2.0).max(1.0),
            h: field_rect.h,
        };
        self.text_origin = Point {
            x: self.content_rect.x,
            y: value_rect
                .map(|rect| rect.y)
                .unwrap_or(field_rect.y + (field_rect.h - self.text_height) * 0.5),
        };
        self.caret_stops = caret_stops(&self.editor, measurer);
        self.preedit_layout = self
            .preedit
            .as_ref()
            .map(|preedit| preedit_layout(preedit, self, measurer));
        self.clamp_scroll();
        self.ensure_cursor_visible();
    }

    fn caret_offset(&self, byte_index: usize) -> f32 {
        self.caret_stops
            .iter()
            .find_map(|(idx, x)| (*idx == byte_index).then_some(*x))
            .unwrap_or_else(|| self.caret_stops.last().map(|(_, x)| *x).unwrap_or(0.0))
    }

    fn nearest_index_for_screen_x(&self, x: f32) -> usize {
        let local_x = self.content_x_from_screen(x);
        self.caret_stops
            .iter()
            .min_by(|(_, lhs_x), (_, rhs_x)| {
                let lhs_distance = (lhs_x - local_x).abs();
                let rhs_distance = (rhs_x - local_x).abs();
                lhs_distance.total_cmp(&rhs_distance)
            })
            .map(|(idx, _)| *idx)
            .unwrap_or_else(|| self.editor.text().len())
    }

    fn content_x_from_screen(&self, x: f32) -> f32 {
        (x - self.text_origin.x + self.scroll_x).max(0.0)
    }

    fn visible_content_x(&self, content_x: f32) -> f32 {
        self.text_origin.x + content_x - self.scroll_x
    }

    fn ensure_cursor_visible(&mut self) {
        let view_width = self.content_rect.w.max(1.0);
        let caret_x = self.active_caret_offset();
        if caret_x < self.scroll_x {
            self.scroll_x = caret_x;
        } else if caret_x > self.scroll_x + view_width {
            self.scroll_x = (caret_x - view_width).max(0.0);
        }
        self.clamp_scroll();
    }

    fn active_caret_offset(&self) -> f32 {
        self.preedit_layout
            .map(|layout| layout.caret_x)
            .unwrap_or_else(|| self.caret_offset(self.editor.cursor()))
    }

    fn active_content_width(&self) -> f32 {
        let text_width = self.caret_stops.last().map(|(_, x)| *x).unwrap_or(0.0);
        let Some(preedit) = &self.preedit else {
            return text_width;
        };
        let start_x = self.caret_offset(preedit.range.0);
        let end_x = self.caret_offset(preedit.range.1);
        start_x + self.preedit_width().unwrap_or(0.0) + (text_width - end_x)
    }

    fn clamp_scroll(&mut self) {
        let max_scroll = (self.active_content_width() - self.content_rect.w).max(0.0);
        self.scroll_x = self.scroll_x.clamp(0.0, max_scroll);
    }
}

pub struct TextInputStore {
    runtimes: HashMap<String, TextInputRuntime>,
}

impl TextInputStore {
    pub fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }

    pub fn sync_with_tree(&mut self, tree: &Tree, measurer: &mut TextMeasurer) {
        let mut next = HashMap::new();

        for (_, node) in tree.iter() {
            let NodeKind::Widget(props) = &node.kind else {
                continue;
            };
            let Some(text_input) = props.as_any().downcast_ref::<TextInputProps>() else {
                continue;
            };

            let widget_id = node.id.to_string();
            let mut runtime = self
                .runtimes
                .remove(&widget_id)
                .unwrap_or_else(|| TextInputRuntime::new(text_input.value.as_ref()));

            if runtime.editor.text() != text_input.value.as_ref() {
                runtime.clear_preedit();
                runtime.editor.set_text(text_input.value.as_ref());
            }

            let field_rect = find_rect(tree, &format!("{}::field", widget_id)).unwrap_or(Rect {
                x: node.rect.x,
                y: node.rect.y,
                w: node.rect.w,
                h: TEXT_INPUT_FIELD_HEIGHT,
            });
            let value_rect = find_rect(tree, &format!("{}::value", widget_id));
            runtime.sync_layout(field_rect, value_rect, measurer);
            next.insert(widget_id, runtime);
        }

        self.runtimes = next;
    }

    pub fn runtime(&self, widget_id: &str) -> Option<&TextInputRuntime> {
        self.runtimes.get(widget_id)
    }

    pub fn runtime_mut(&mut self, widget_id: &str) -> Option<&mut TextInputRuntime> {
        self.runtimes.get_mut(widget_id)
    }

    pub fn clear_unfocused_preedit(&mut self, focused_widget_id: Option<&str>) {
        for (widget_id, runtime) in &mut self.runtimes {
            if Some(widget_id.as_str()) != focused_widget_id {
                runtime.clear_preedit();
            }
        }
    }

    pub fn focused_widget_id(&self, tree: &Tree, focused: Option<NodeId>) -> Option<String> {
        let focused = focused?;
        let node = tree.get(focused)?;
        let NodeKind::Widget(props) = &node.kind else {
            return None;
        };
        props
            .as_any()
            .downcast_ref::<TextInputProps>()
            .map(|_| node.id.to_string())
    }
}

impl Default for TextInputStore {
    fn default() -> Self {
        Self::new()
    }
}

fn find_rect(tree: &Tree, node_id: &str) -> Option<Rect> {
    tree.iter()
        .find_map(|(_, node)| (node.id.as_ref() == node_id).then_some(node.rect))
}

fn caret_stops(editor: &TextEditState, measurer: &mut TextMeasurer) -> Vec<(usize, f32)> {
    let text = editor.text();
    let mut stops = Vec::with_capacity(text.chars().count() + 1);
    stops.push((0, 0.0));

    for (byte_index, _) in text.char_indices().skip(1) {
        let width = measurer
            .measure(&text[..byte_index], TEXT_INPUT_VALUE_FONT_SIZE)
            .0;
        stops.push((byte_index, width));
    }

    if text.is_empty() {
        return stops;
    }

    let width = measurer.measure(text, TEXT_INPUT_VALUE_FONT_SIZE).0;
    if stops.last().map(|(idx, _)| *idx) != Some(text.len()) {
        stops.push((text.len(), width));
    }
    stops
}

fn preedit_layout(
    preedit: &PreeditState,
    runtime: &TextInputRuntime,
    measurer: &mut TextMeasurer,
) -> PreeditLayout {
    let start = clamp_text_index(runtime.editor.text(), preedit.range.0);
    let start_x = runtime.caret_offset(start);
    let width = measurer
        .measure(&preedit.text, TEXT_INPUT_VALUE_FONT_SIZE)
        .0;
    let caret_byte = preedit
        .caret
        .map(|(_, end)| clamp_text_index(&preedit.text, end))
        .unwrap_or(preedit.text.len());
    let caret_prefix = &preedit.text[..caret_byte];
    let caret_x = start_x + measurer.measure(caret_prefix, TEXT_INPUT_VALUE_FONT_SIZE).0;

    PreeditLayout {
        start_x,
        width,
        caret_x,
    }
}

fn clamp_text_index(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::TextEditState;

    #[test]
    fn caret_moves_to_nearest_click_position() {
        let mut runtime = TextInputRuntime::new("hello");
        runtime.text_origin = Point { x: 10.0, y: 5.0 };
        runtime.caret_stops = vec![(0, 0.0), (1, 8.0), (2, 16.0), (3, 24.0)];
        runtime.editor = TextEditState::new("hey");

        runtime.set_caret_from_x(27.0);

        assert_eq!(runtime.editor.cursor(), 2);
    }

    #[test]
    fn selection_rect_uses_current_selection_range() {
        let mut runtime = TextInputRuntime::new("hello");
        runtime.text_origin = Point { x: 10.0, y: 5.0 };
        runtime.text_height = 12.0;
        runtime.caret_stops = vec![(0, 0.0), (1, 8.0), (2, 16.0), (3, 24.0), (5, 40.0)];
        runtime.editor = TextEditState::new("hello");
        runtime.editor.move_home();
        runtime.editor.select_right();
        runtime.editor.select_right();

        let rect = runtime.selection_rect().unwrap();
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.w, 16.0);
        assert_eq!(rect.h, 12.0);
    }

    #[test]
    fn preedit_reuses_selection_range_until_commit() {
        let mut runtime = TextInputRuntime::new("hello");
        runtime.editor.move_home();
        runtime.editor.select_right();
        runtime.editor.select_right();

        runtime.set_preedit("ni", Some((2, 2)));
        assert_eq!(runtime.preedit_range(), Some((0, 2)));

        runtime.set_preedit("nihao", Some((5, 5)));
        assert_eq!(runtime.preedit_range(), Some((0, 2)));
    }

    #[test]
    fn clear_unfocused_preedit_only_keeps_focused_runtime() {
        let mut store = TextInputStore::new();
        store
            .runtimes
            .insert("a".into(), TextInputRuntime::new("hello"));
        store
            .runtimes
            .insert("b".into(), TextInputRuntime::new("world"));
        store.runtimes.get_mut("a").unwrap().set_preedit("ni", None);
        store
            .runtimes
            .get_mut("b")
            .unwrap()
            .set_preedit("hao", None);

        store.clear_unfocused_preedit(Some("b"));

        assert!(!store.runtime("a").unwrap().has_preedit());
        assert!(store.runtime("b").unwrap().has_preedit());
    }

    #[test]
    fn long_text_scrolls_to_keep_caret_visible() {
        let mut runtime = TextInputRuntime::new("hello");
        runtime.content_rect = Rect {
            x: 10.0,
            y: 5.0,
            w: 20.0,
            h: 20.0,
        };
        runtime.text_origin = Point { x: 10.0, y: 5.0 };
        runtime.caret_stops = vec![
            (0, 0.0),
            (1, 8.0),
            (2, 16.0),
            (3, 24.0),
            (4, 32.0),
            (5, 40.0),
        ];
        runtime.editor = TextEditState::new("hello");
        runtime.editor.move_end();

        runtime.ensure_cursor_visible();

        assert_eq!(runtime.scroll_x, 20.0);
        assert_eq!(runtime.caret_rect().x, 30.0);
    }

    #[test]
    fn hit_testing_accounts_for_scroll_offset() {
        let mut runtime = TextInputRuntime::new("hello");
        runtime.content_rect = Rect {
            x: 10.0,
            y: 5.0,
            w: 20.0,
            h: 20.0,
        };
        runtime.text_origin = Point { x: 10.0, y: 5.0 };
        runtime.scroll_x = 16.0;
        runtime.caret_stops = vec![
            (0, 0.0),
            (1, 8.0),
            (2, 16.0),
            (3, 24.0),
            (4, 32.0),
            (5, 40.0),
        ];
        runtime.editor = TextEditState::new("hello");

        runtime.set_caret_from_x(11.0);

        assert_eq!(runtime.editor.cursor(), 2);
    }
}
