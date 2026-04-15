use std::collections::HashMap;

use crate::renderer::{Point, Rect, TextMeasurer};
use crate::theme::{TextInputTheme, Theme};
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::number_input::{format_number, NumberInputProps};
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::TextEditState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFieldKind {
    TextInput,
    NumberInput,
}

#[derive(Clone, Debug)]
struct TextFieldSpec {
    external_text: String,
    kind: TextFieldKind,
    tokens: TextInputTheme,
}

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
    last_external_text: String,
    kind: TextFieldKind,
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
    fn new(value: &str, kind: TextFieldKind, tokens: TextInputTheme) -> Self {
        Self {
            editor: TextEditState::new(value),
            last_external_text: value.to_string(),
            kind,
            field_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: tokens.field_height,
            },
            content_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: tokens.field_height,
            },
            text_origin: Point { x: 0.0, y: 0.0 },
            text_height: tokens.value_size,
            scroll_x: 0.0,
            caret_stops: vec![(0, 0.0)],
            preedit: None,
            preedit_layout: None,
        }
    }

    pub fn kind(&self) -> TextFieldKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: TextFieldKind) {
        self.kind = kind;
    }

    pub fn external_text(&self) -> &str {
        &self.last_external_text
    }

    pub fn sync_external_text(&mut self, text: &str) {
        if text != self.last_external_text {
            self.clear_preedit();
            self.editor.set_text(text);
            self.last_external_text = text.to_string();
        }
    }

    pub fn revert_to_external(&mut self) {
        let external = self.last_external_text.clone();
        self.clear_preedit();
        self.editor.set_text(&external);
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
        tokens: TextInputTheme,
    ) {
        self.field_rect = field_rect;
        self.text_height = measurer
            .measure("Mg", tokens.value_size)
            .1
            .max(tokens.value_size);
        self.content_rect = Rect {
            x: field_rect.x + tokens.padding_x,
            y: field_rect.y,
            w: (field_rect.w - tokens.padding_x * 2.0).max(1.0),
            h: field_rect.h,
        };
        self.text_origin = Point {
            x: self.content_rect.x,
            y: value_rect
                .map(|rect| rect.y)
                .unwrap_or(field_rect.y + (field_rect.h - self.text_height) * 0.5),
        };
        self.caret_stops = caret_stops(&self.editor, measurer, tokens.value_size);
        self.preedit_layout = self
            .preedit
            .as_ref()
            .map(|preedit| preedit_layout(preedit, self, measurer, tokens.value_size));
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

    pub fn sync_with_tree(&mut self, tree: &Tree, measurer: &mut TextMeasurer, theme: &Theme) {
        let mut next = HashMap::new();

        for (_, node) in tree.iter() {
            let NodeKind::Widget(props) = &node.kind else {
                continue;
            };
            let Some(spec) = text_field_spec(props.as_ref(), theme) else {
                continue;
            };

            let widget_id = node.id.to_string();
            let mut runtime = self.runtimes.remove(&widget_id).unwrap_or_else(|| {
                TextInputRuntime::new(&spec.external_text, spec.kind, spec.tokens)
            });

            runtime.set_kind(spec.kind);
            runtime.sync_external_text(&spec.external_text);

            let field_rect = find_rect(tree, &format!("{}::field", widget_id)).unwrap_or(Rect {
                x: node.rect.x,
                y: node.rect.y,
                w: node.rect.w,
                h: spec.tokens.field_height,
            });
            let value_rect = find_rect(tree, &format!("{}::value", widget_id));
            runtime.sync_layout(field_rect, value_rect, measurer, spec.tokens);
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

    pub fn revert_unfocused_numbers(&mut self, focused_widget_id: Option<&str>) {
        for (widget_id, runtime) in &mut self.runtimes {
            if Some(widget_id.as_str()) != focused_widget_id
                && runtime.kind() == TextFieldKind::NumberInput
                && runtime.editor().text() != runtime.external_text()
            {
                runtime.revert_to_external();
            }
        }
    }

    pub fn focused_widget_id(&self, tree: &Tree, focused: Option<NodeId>) -> Option<String> {
        let focused = focused?;
        let node = tree.get(focused)?;
        let NodeKind::Widget(props) = &node.kind else {
            return None;
        };
        is_text_field_props(props.as_ref()).then(|| node.id.to_string())
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

fn caret_stops(
    editor: &TextEditState,
    measurer: &mut TextMeasurer,
    font_size: f32,
) -> Vec<(usize, f32)> {
    let text = editor.text();
    let mut stops = Vec::with_capacity(text.chars().count() + 1);
    stops.push((0, 0.0));

    for (byte_index, _) in text.char_indices().skip(1) {
        let width = measurer.measure(&text[..byte_index], font_size).0;
        stops.push((byte_index, width));
    }

    if text.is_empty() {
        return stops;
    }

    let width = measurer.measure(text, font_size).0;
    if stops.last().map(|(idx, _)| *idx) != Some(text.len()) {
        stops.push((text.len(), width));
    }
    stops
}

fn preedit_layout(
    preedit: &PreeditState,
    runtime: &TextInputRuntime,
    measurer: &mut TextMeasurer,
    font_size: f32,
) -> PreeditLayout {
    let start = clamp_text_index(runtime.editor.text(), preedit.range.0);
    let start_x = runtime.caret_offset(start);
    let width = measurer.measure(&preedit.text, font_size).0;
    let caret_byte = preedit
        .caret
        .map(|(_, end)| clamp_text_index(&preedit.text, end))
        .unwrap_or(preedit.text.len());
    let caret_prefix = &preedit.text[..caret_byte];
    let caret_x = start_x + measurer.measure(caret_prefix, font_size).0;

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

fn text_field_spec(
    props: &dyn crate::widget::props::WidgetProps,
    theme: &Theme,
) -> Option<TextFieldSpec> {
    if let Some(text_input) = props.as_any().downcast_ref::<TextInputProps>() {
        return Some(TextFieldSpec {
            external_text: text_input.value.to_string(),
            kind: TextFieldKind::TextInput,
            tokens: theme.components.text_input,
        });
    }

    props
        .as_any()
        .downcast_ref::<NumberInputProps>()
        .map(|number_input| TextFieldSpec {
            external_text: format_number(number_input.value, number_input.precision),
            kind: TextFieldKind::NumberInput,
            tokens: theme.components.number_input,
        })
}

fn is_text_field_props(props: &dyn crate::widget::props::WidgetProps) -> bool {
    props.as_any().downcast_ref::<TextInputProps>().is_some()
        || props.as_any().downcast_ref::<NumberInputProps>().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::dark_theme;
    use crate::widget::TextEditState;

    #[test]
    fn caret_moves_to_nearest_click_position() {
        let theme = dark_theme();
        let mut runtime = TextInputRuntime::new(
            "hello",
            TextFieldKind::TextInput,
            theme.components.text_input,
        );
        runtime.text_origin = Point { x: 10.0, y: 5.0 };
        runtime.caret_stops = vec![(0, 0.0), (1, 8.0), (2, 16.0), (3, 24.0)];
        runtime.editor = TextEditState::new("hey");

        runtime.set_caret_from_x(27.0);

        assert_eq!(runtime.editor.cursor(), 2);
    }

    #[test]
    fn selection_rect_uses_current_selection_range() {
        let theme = dark_theme();
        let mut runtime = TextInputRuntime::new(
            "hello",
            TextFieldKind::TextInput,
            theme.components.text_input,
        );
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
        let theme = dark_theme();
        let mut runtime = TextInputRuntime::new(
            "hello",
            TextFieldKind::TextInput,
            theme.components.text_input,
        );
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
        let theme = dark_theme();
        let mut store = TextInputStore::new();
        store.runtimes.insert(
            "a".into(),
            TextInputRuntime::new(
                "hello",
                TextFieldKind::TextInput,
                theme.components.text_input,
            ),
        );
        store.runtimes.insert(
            "b".into(),
            TextInputRuntime::new(
                "world",
                TextFieldKind::TextInput,
                theme.components.text_input,
            ),
        );
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
        let theme = dark_theme();
        let mut runtime = TextInputRuntime::new(
            "hello",
            TextFieldKind::TextInput,
            theme.components.text_input,
        );
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
        let theme = dark_theme();
        let mut runtime = TextInputRuntime::new(
            "hello",
            TextFieldKind::TextInput,
            theme.components.text_input,
        );
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
