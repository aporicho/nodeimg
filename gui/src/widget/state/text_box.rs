use std::collections::HashMap;

use crate::renderer::{Point, Rect, TextMeasurer, TextStyle};
use crate::runtime::ControlIntrinsic;
use crate::text::layout::{layout_text, TextLayoutPolicy, TextLayoutResult};
use crate::theme::{TextInputTheme, Theme};
use crate::tree::{NodeId, NodeKind, Tree};
use crate::widget::atoms::number_input::{format_number, NumberInputProps};
use crate::widget::atoms::text_area::TextAreaProps;
use crate::widget::atoms::text_box::{
    text_box_value_style, TextBoxFont, TextBoxMode, TextBoxProps,
};
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::TextEditState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TextBoxValueKind {
    Text,
    Number {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        precision: usize,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct TextBoxSpec {
    pub(crate) external_text: String,
    pub(crate) mode: TextBoxMode,
    pub(crate) value_kind: TextBoxValueKind,
    pub(crate) tokens: TextInputTheme,
    pub(crate) font: TextBoxFont,
    pub(crate) disabled: bool,
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

pub(crate) struct TextBoxRuntime {
    editor: TextEditState,
    last_external_text: String,
    mode: TextBoxMode,
    value_kind: TextBoxValueKind,
    field_rect: Rect,
    content_rect: Rect,
    text_origin: Point,
    layout: TextLayoutResult,
    min_height: f32,
    desired_height: f32,
    scroll_x: f32,
    preedit: Option<PreeditState>,
    preedit_layout: Option<PreeditLayout>,
}

impl TextBoxRuntime {
    fn new(spec: &TextBoxSpec) -> Self {
        let line_height = spec.tokens.value_size * 1.2;
        let min_height = min_height_for_mode(spec.mode, line_height, spec.tokens);
        Self {
            editor: TextEditState::new(&spec.external_text),
            last_external_text: spec.external_text.clone(),
            mode: spec.mode,
            value_kind: spec.value_kind,
            field_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: min_height,
            },
            content_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: min_height,
            },
            text_origin: Point { x: 0.0, y: 0.0 },
            layout: TextLayoutResult {
                lines: Vec::new(),
                line_height,
                width: 1.0,
                height: line_height,
            },
            min_height,
            desired_height: min_height,
            scroll_x: 0.0,
            preedit: None,
            preedit_layout: None,
        }
    }

    fn sync_spec(&mut self, spec: &TextBoxSpec) {
        self.mode = spec.mode;
        self.value_kind = spec.value_kind;
        if self.is_multiline() {
            self.scroll_x = 0.0;
        }
    }

    pub(crate) fn value_kind(&self) -> TextBoxValueKind {
        self.value_kind
    }

    pub(crate) fn is_multiline(&self) -> bool {
        matches!(self.mode, TextBoxMode::MultiLine { .. })
    }

    pub(crate) fn is_number(&self) -> bool {
        matches!(self.value_kind, TextBoxValueKind::Number { .. })
    }

    pub(crate) fn external_text(&self) -> &str {
        &self.last_external_text
    }

    pub(crate) fn editor(&self) -> &TextEditState {
        &self.editor
    }

    pub(crate) fn editor_mut(&mut self) -> &mut TextEditState {
        &mut self.editor
    }

    pub(crate) fn sync_external_text(&mut self, text: &str, allow_override: bool) -> bool {
        if text == self.last_external_text {
            return false;
        }

        let current_matches_external = self.editor.text() == self.last_external_text;
        if allow_override || current_matches_external || self.editor.text() == text {
            self.clear_preedit();
            self.editor.set_text(text);
        }
        self.last_external_text = text.to_string();
        true
    }

    pub(crate) fn revert_to_external(&mut self) {
        let external = self.last_external_text.clone();
        self.clear_preedit();
        self.editor.set_text(&external);
    }

    pub(crate) fn clear_preedit(&mut self) {
        self.preedit = None;
        self.preedit_layout = None;
        self.clamp_scroll();
    }

    pub(crate) fn has_preedit(&self) -> bool {
        self.preedit.is_some()
    }

    pub(crate) fn preedit_text(&self) -> Option<&str> {
        self.preedit.as_ref().map(|preedit| preedit.text.as_str())
    }

    pub(crate) fn preedit_range(&self) -> Option<(usize, usize)> {
        self.preedit.as_ref().map(|preedit| preedit.range)
    }

    pub(crate) fn set_preedit(&mut self, text: &str, caret: Option<(usize, usize)>) {
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

    fn sync_layout_with_style(
        &mut self,
        field_rect: Rect,
        value_rect: Option<Rect>,
        measurer: &mut TextMeasurer,
        spec: &TextBoxSpec,
        value_style: TextStyle,
    ) {
        self.field_rect = field_rect;
        match spec.mode {
            TextBoxMode::SingleLine => {
                self.content_rect = Rect {
                    x: field_rect.x + spec.tokens.padding_x,
                    y: field_rect.y,
                    w: (field_rect.w - spec.tokens.padding_x * 2.0).max(1.0),
                    h: field_rect.h,
                };
                self.layout = layout_text(
                    self.editor.text(),
                    TextLayoutPolicy::NoWrap,
                    self.content_rect.w,
                    measurer,
                    &value_style,
                );
                self.text_origin = Point {
                    x: self.content_rect.x,
                    y: value_rect
                        .map(|rect| rect.y)
                        .unwrap_or(field_rect.y + (field_rect.h - self.layout.line_height) * 0.5),
                };
                self.min_height = field_rect.h;
                self.desired_height = field_rect.h;
                self.preedit_layout = self
                    .preedit
                    .as_ref()
                    .map(|preedit| preedit_layout(preedit, self, measurer, &value_style));
                self.clamp_scroll();
                self.ensure_cursor_visible();
            }
            TextBoxMode::MultiLine { min_rows } => {
                self.content_rect = Rect {
                    x: field_rect.x + spec.tokens.padding_x,
                    y: field_rect.y + spec.tokens.padding_y,
                    w: (field_rect.w - spec.tokens.padding_x * 2.0).max(1.0),
                    h: (field_rect.h - spec.tokens.padding_y * 2.0).max(1.0),
                };
                self.text_origin = Point {
                    x: self.content_rect.x,
                    y: self.content_rect.y,
                };
                self.layout = layout_text(
                    self.editor.text(),
                    TextLayoutPolicy::Wrap,
                    self.content_rect.w,
                    measurer,
                    &value_style,
                );
                self.min_height =
                    min_rows.max(1) as f32 * self.layout.line_height + spec.tokens.padding_y * 2.0;
                self.desired_height =
                    (self.layout.height + spec.tokens.padding_y * 2.0).max(self.min_height);
                self.scroll_x = 0.0;
                self.preedit_layout = None;
            }
        }
    }

    pub(crate) fn current_size(&self) -> [f32; 2] {
        [self.field_rect.w, self.field_rect.h]
    }

    pub(crate) fn min_size(&self) -> [f32; 2] {
        [self.field_rect.w, self.min_height]
    }

    pub(crate) fn desired_size(&self) -> [f32; 2] {
        [self.field_rect.w, self.desired_height]
    }

    pub(crate) fn clip_rect(&self) -> Rect {
        self.content_rect
    }

    pub(crate) fn text_draw_origin(&self) -> Point {
        Point {
            x: self.visible_content_x(0.0),
            y: self.text_origin.y,
        }
    }

    pub(crate) fn layout(&self) -> &TextLayoutResult {
        &self.layout
    }

    pub(crate) fn visible_line_origin(&self, line_y: f32) -> Point {
        Point {
            x: self.visible_content_x(0.0),
            y: self.text_origin.y + line_y,
        }
    }

    pub(crate) fn caret_rect(&self) -> Rect {
        let caret_x = self
            .preedit_layout
            .map(|layout| layout.caret_x)
            .unwrap_or_else(|| self.caret_offset(self.editor.cursor()));
        let point = if self.is_multiline() {
            self.layout.caret_point(self.editor.cursor())
        } else {
            Point { x: caret_x, y: 0.0 }
        };
        Rect {
            x: self.visible_content_x(point.x),
            y: self.text_origin.y + point.y,
            w: 1.0,
            h: self.layout.line_height.max(1.0),
        }
    }

    pub(crate) fn selection_rects(&self) -> Vec<Rect> {
        if self.preedit.is_some() {
            return Vec::new();
        }
        let Some((start, end)) = self.editor.selection_range() else {
            return Vec::new();
        };
        self.layout
            .selection_rects(start, end)
            .into_iter()
            .map(|rect| Rect {
                x: self.visible_content_x(rect.x),
                y: self.text_origin.y + rect.y,
                w: rect.w,
                h: rect.h,
            })
            .collect()
    }

    pub(crate) fn preedit_start_x(&self) -> Option<f32> {
        self.preedit_layout
            .map(|layout| self.visible_content_x(layout.start_x))
    }

    pub(crate) fn preedit_width(&self) -> Option<f32> {
        self.preedit_layout.map(|layout| layout.width)
    }

    pub(crate) fn preedit_underline_rect(&self) -> Option<Rect> {
        let layout = self.preedit_layout?;
        Some(Rect {
            x: self.visible_content_x(layout.start_x),
            y: self.text_origin.y + self.layout.line_height + 1.0,
            w: layout.width.max(1.0),
            h: 1.0,
        })
    }

    pub(crate) fn preedit_origin_and_text(&self) -> Option<(Point, &str)> {
        let preedit = self.preedit.as_ref()?;
        let start = clamp_text_index(self.editor.text(), preedit.range.0);
        let point = self.layout.caret_point(start);
        let caret_text = preedit
            .caret
            .map(|(_, end)| &preedit.text[..clamp_text_index(&preedit.text, end)])
            .unwrap_or(preedit.text.as_str());
        let caret_offset = self.layout_width_for_text(caret_text);
        Some((
            Point {
                x: self.visible_content_x(point.x + caret_offset),
                y: self.text_origin.y + point.y,
            },
            preedit.text.as_str(),
        ))
    }

    pub(crate) fn set_caret_from_point(&mut self, x: f32, y: f32) {
        let index = self.nearest_index_for_point(x, y);
        self.editor.move_to(index);
        self.ensure_cursor_visible();
    }

    pub(crate) fn select_to_point(&mut self, x: f32, y: f32) {
        let index = self.nearest_index_for_point(x, y);
        self.editor.select_to(index);
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_left(&mut self, extend_selection: bool) {
        if extend_selection {
            self.editor.select_left();
        } else {
            self.editor.move_left();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_right(&mut self, extend_selection: bool) {
        if extend_selection {
            self.editor.select_right();
        } else {
            self.editor.move_right();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_home(&mut self, extend_selection: bool) {
        if extend_selection {
            self.editor.select_to(0);
        } else {
            self.editor.move_home();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_end(&mut self, extend_selection: bool) {
        let text_end = self.editor.text().len();
        if extend_selection {
            self.editor.select_to(text_end);
        } else {
            self.editor.move_end();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_vertical(&mut self, direction: i32, extend_selection: bool) {
        if !self.is_multiline() {
            return;
        }
        let cursor = self.editor.cursor();
        let Some(line_index) = self
            .layout
            .lines
            .iter()
            .position(|line| cursor >= line.start && cursor <= line.end)
        else {
            return;
        };
        let target = (line_index as i32 + direction)
            .clamp(0, self.layout.lines.len().saturating_sub(1) as i32)
            as usize;
        if target == line_index {
            return;
        }
        let x = self.layout.caret_point(cursor).x;
        let y = self.layout.lines[target].y;
        let index = self.layout.nearest_index(x, y);
        if extend_selection {
            self.editor.select_to(index);
        } else {
            self.editor.move_to(index);
        }
    }

    fn nearest_index_for_point(&self, x: f32, y: f32) -> usize {
        let local_x = if self.is_multiline() {
            x - self.text_origin.x
        } else {
            x - self.text_origin.x + self.scroll_x
        };
        let local_y = y - self.text_origin.y;
        self.layout
            .nearest_index(local_x.max(0.0), local_y.max(0.0))
    }

    fn caret_offset(&self, byte_index: usize) -> f32 {
        self.layout
            .line_for_index(byte_index)
            .map(|line| line.x_for_index(byte_index))
            .unwrap_or(0.0)
    }

    fn visible_content_x(&self, content_x: f32) -> f32 {
        if self.is_multiline() {
            self.text_origin.x + content_x
        } else {
            self.text_origin.x + content_x - self.scroll_x
        }
    }

    fn ensure_cursor_visible(&mut self) {
        if self.is_multiline() {
            return;
        }
        let view_width = self.content_rect.w.max(1.0);
        let caret_x = self
            .preedit_layout
            .map(|layout| layout.caret_x)
            .unwrap_or_else(|| self.caret_offset(self.editor.cursor()));
        if caret_x < self.scroll_x {
            self.scroll_x = caret_x;
        } else if caret_x > self.scroll_x + view_width {
            self.scroll_x = (caret_x - view_width).max(0.0);
        }
        self.clamp_scroll();
    }

    fn active_content_width(&self) -> f32 {
        let text_width = self
            .layout
            .lines
            .first()
            .map(|line| line.width)
            .unwrap_or(0.0);
        let Some(preedit) = &self.preedit else {
            return text_width;
        };
        let start_x = self.caret_offset(preedit.range.0);
        let end_x = self.caret_offset(preedit.range.1);
        start_x + self.preedit_width().unwrap_or(0.0) + (text_width - end_x)
    }

    fn clamp_scroll(&mut self) {
        if self.is_multiline() {
            self.scroll_x = 0.0;
            return;
        }
        let max_scroll = (self.active_content_width() - self.content_rect.w).max(0.0);
        self.scroll_x = self.scroll_x.clamp(0.0, max_scroll);
    }

    fn layout_width_for_text(&self, text: &str) -> f32 {
        let chars = text.chars().count() as f32;
        chars * self.layout.line_height * 0.5
    }
}

pub(crate) struct TextBoxStore {
    runtimes: HashMap<String, TextBoxRuntime>,
}

impl TextBoxStore {
    pub(crate) fn new() -> Self {
        Self {
            runtimes: HashMap::new(),
        }
    }

    pub(crate) fn sync_with_tree(
        &mut self,
        tree: &Tree,
        measurer: &mut TextMeasurer,
        theme: &Theme,
        focused_widget_id: Option<&str>,
    ) {
        let mut next = HashMap::new();

        for (_, node) in tree.iter() {
            let NodeKind::Widget(props) = &node.kind else {
                continue;
            };
            let Some(spec) = text_box_spec(props.as_ref(), theme) else {
                continue;
            };

            let widget_id = node.id.to_string();
            let mut runtime = self
                .runtimes
                .remove(&widget_id)
                .unwrap_or_else(|| TextBoxRuntime::new(&spec));

            runtime.sync_spec(&spec);
            let allow_override = Some(widget_id.as_str()) != focused_widget_id;
            let external_text_changed =
                runtime.sync_external_text(&spec.external_text, allow_override);

            let field_rect = find_rect(tree, &format!("{widget_id}::field")).unwrap_or(Rect {
                x: node.rect.x,
                y: node.rect.y,
                w: node.rect.w,
                h: fallback_height(spec.mode, spec.tokens),
            });
            let value_rect = find_rect(tree, &format!("{widget_id}::value"));
            let value_style = text_box_value_style(theme, spec.tokens, spec.font);
            runtime.sync_layout_with_style(field_rect, value_rect, measurer, &spec, value_style);
            log_text_box_sizing(
                widget_id.as_str(),
                &runtime,
                &spec,
                Some(widget_id.as_str()) == focused_widget_id,
                external_text_changed,
                value_rect,
            );
            next.insert(widget_id, runtime);
        }

        self.runtimes = next;
    }

    pub(crate) fn runtime(&self, widget_id: &str) -> Option<&TextBoxRuntime> {
        self.runtimes.get(widget_id)
    }

    pub(crate) fn runtime_mut(&mut self, widget_id: &str) -> Option<&mut TextBoxRuntime> {
        self.runtimes.get_mut(widget_id)
    }

    pub(crate) fn clear_unfocused_preedit(&mut self, focused_widget_id: Option<&str>) {
        for (widget_id, runtime) in &mut self.runtimes {
            if Some(widget_id.as_str()) != focused_widget_id {
                runtime.clear_preedit();
            }
        }
    }

    pub(crate) fn revert_unfocused_numbers(&mut self, focused_widget_id: Option<&str>) {
        for (widget_id, runtime) in &mut self.runtimes {
            if Some(widget_id.as_str()) != focused_widget_id
                && runtime.is_number()
                && runtime.editor().text() != runtime.external_text()
            {
                runtime.revert_to_external();
            }
        }
    }

    pub(crate) fn focused_widget_id(&self, tree: &Tree, focused: Option<NodeId>) -> Option<String> {
        let focused = focused?;
        let node = tree.get(focused)?;
        let NodeKind::Widget(props) = &node.kind else {
            return None;
        };
        is_text_box_props(props.as_ref()).then(|| node.id.to_string())
    }

    pub(crate) fn control_intrinsics(&self) -> Vec<ControlIntrinsic> {
        let intrinsics = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.is_multiline())
            .map(|(widget_id, runtime)| ControlIntrinsic {
                widget_id: widget_id.clone(),
                current_size: runtime.current_size(),
                min_size: runtime.min_size(),
                desired_size: runtime.desired_size(),
                affects_parent_width: false,
                affects_parent_height: true,
            })
            .collect::<Vec<_>>();

        for intrinsic in &intrinsics {
            let height_delta = intrinsic.desired_size[1] - intrinsic.current_size[1];
            let log_at_debug = height_delta.abs() > 0.5;
            if log_at_debug {
                tracing::debug!(
                    target: "gui::widget::text_box_sizing",
                    widget_id = %intrinsic.widget_id,
                    current_w = intrinsic.current_size[0],
                    current_h = intrinsic.current_size[1],
                    min_w = intrinsic.min_size[0],
                    min_h = intrinsic.min_size[1],
                    desired_w = intrinsic.desired_size[0],
                    desired_h = intrinsic.desired_size[1],
                    height_delta,
                    affects_parent_height = intrinsic.affects_parent_height,
                    "emit text box control intrinsic with height delta"
                );
            } else {
                tracing::trace!(
                    target: "gui::widget::text_box_sizing",
                    widget_id = %intrinsic.widget_id,
                    current_w = intrinsic.current_size[0],
                    current_h = intrinsic.current_size[1],
                    min_w = intrinsic.min_size[0],
                    min_h = intrinsic.min_size[1],
                    desired_w = intrinsic.desired_size[0],
                    desired_h = intrinsic.desired_size[1],
                    affects_parent_height = intrinsic.affects_parent_height,
                    "emit text box control intrinsic"
                );
            }
        }

        intrinsics
    }
}

impl Default for TextBoxStore {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn text_box_spec(
    props: &dyn crate::widget::props::WidgetProps,
    theme: &Theme,
) -> Option<TextBoxSpec> {
    if let Some(text_input) = props.as_any().downcast_ref::<TextInputProps>() {
        return Some(TextBoxSpec {
            external_text: text_input.value.to_string(),
            mode: TextBoxMode::SingleLine,
            value_kind: TextBoxValueKind::Text,
            tokens: theme.text_field_metrics(text_input.size, text_input.density),
            font: TextBoxFont::Body,
            disabled: text_input.disabled,
        });
    }

    if let Some(text_area) = props.as_any().downcast_ref::<TextAreaProps>() {
        return Some(TextBoxSpec {
            external_text: text_area.value.to_string(),
            mode: TextBoxMode::MultiLine {
                min_rows: text_area.min_rows,
            },
            value_kind: TextBoxValueKind::Text,
            tokens: theme.text_field_metrics(text_area.size, text_area.density),
            font: TextBoxFont::Body,
            disabled: text_area.disabled,
        });
    }

    if let Some(number_input) = props.as_any().downcast_ref::<NumberInputProps>() {
        return Some(TextBoxSpec {
            external_text: format_number(number_input.value, number_input.precision),
            mode: TextBoxMode::SingleLine,
            value_kind: TextBoxValueKind::Number {
                value: number_input.value,
                min: number_input.min,
                max: number_input.max,
                step: number_input.step,
                precision: number_input.precision,
            },
            tokens: theme.text_field_metrics(number_input.size, number_input.density),
            font: TextBoxFont::Mono,
            disabled: number_input.disabled,
        });
    }

    props
        .as_any()
        .downcast_ref::<TextBoxProps>()
        .map(|text_box| TextBoxSpec {
            external_text: text_box.value.to_string(),
            mode: text_box.mode,
            value_kind: TextBoxValueKind::Text,
            tokens: theme.text_field_metrics(text_box.size, text_box.density),
            font: text_box.font,
            disabled: text_box.disabled,
        })
}

pub(crate) fn is_text_box_props(props: &dyn crate::widget::props::WidgetProps) -> bool {
    props.as_any().downcast_ref::<TextInputProps>().is_some()
        || props.as_any().downcast_ref::<TextAreaProps>().is_some()
        || props.as_any().downcast_ref::<NumberInputProps>().is_some()
        || props.as_any().downcast_ref::<TextBoxProps>().is_some()
}

fn find_rect(tree: &Tree, node_id: &str) -> Option<Rect> {
    tree.iter()
        .find_map(|(_, node)| (node.id.as_ref() == node_id).then_some(node.rect))
}

fn fallback_height(mode: TextBoxMode, tokens: TextInputTheme) -> f32 {
    match mode {
        TextBoxMode::SingleLine => tokens.field_height,
        TextBoxMode::MultiLine { min_rows } => {
            min_rows.max(1) as f32 * tokens.value_size * 1.2 + tokens.padding_y * 2.0
        }
    }
}

fn min_height_for_mode(mode: TextBoxMode, line_height: f32, tokens: TextInputTheme) -> f32 {
    match mode {
        TextBoxMode::SingleLine => tokens.field_height,
        TextBoxMode::MultiLine { min_rows } => {
            min_rows.max(1) as f32 * line_height + tokens.padding_y * 2.0
        }
    }
}

fn log_text_box_sizing(
    widget_id: &str,
    runtime: &TextBoxRuntime,
    spec: &TextBoxSpec,
    focused: bool,
    external_text_changed: bool,
    value_rect: Option<Rect>,
) {
    let TextBoxMode::MultiLine { min_rows } = spec.mode else {
        return;
    };

    let line_count = runtime.layout.lines.len();
    let height_delta = runtime.desired_height - runtime.field_rect.h;
    let editor_dirty = runtime.editor.text() != runtime.last_external_text;
    let should_debug = external_text_changed || editor_dirty || height_delta.abs() > 0.5;

    if should_debug {
        tracing::debug!(
            target: "gui::widget::text_box_sizing",
            widget_id,
            focused,
            external_text_changed,
            editor_dirty,
            text_bytes = runtime.editor.text().len(),
            text_chars = runtime.editor.text().chars().count(),
            min_rows,
            line_count,
            line_height = runtime.layout.line_height,
            layout_w = runtime.layout.width,
            layout_h = runtime.layout.height,
            field_x = runtime.field_rect.x,
            field_y = runtime.field_rect.y,
            field_w = runtime.field_rect.w,
            field_h = runtime.field_rect.h,
            content_w = runtime.content_rect.w,
            content_h = runtime.content_rect.h,
            min_h = runtime.min_height,
            desired_h = runtime.desired_height,
            height_delta,
            value_rect_w = value_rect.map(|rect| rect.w),
            value_rect_h = value_rect.map(|rect| rect.h),
            "sync multiline text box layout"
        );
    } else {
        tracing::trace!(
            target: "gui::widget::text_box_sizing",
            widget_id,
            focused,
            text_bytes = runtime.editor.text().len(),
            text_chars = runtime.editor.text().chars().count(),
            min_rows,
            line_count,
            line_height = runtime.layout.line_height,
            layout_w = runtime.layout.width,
            layout_h = runtime.layout.height,
            field_w = runtime.field_rect.w,
            field_h = runtime.field_rect.h,
            content_w = runtime.content_rect.w,
            content_h = runtime.content_rect.h,
            min_h = runtime.min_height,
            desired_h = runtime.desired_height,
            "sync multiline text box layout"
        );
    }
}

fn preedit_layout(
    preedit: &PreeditState,
    runtime: &TextBoxRuntime,
    measurer: &mut TextMeasurer,
    style: &TextStyle,
) -> PreeditLayout {
    let start = clamp_text_index(runtime.editor.text(), preedit.range.0);
    let start_x = runtime.caret_offset(start);
    let width = measurer.measure_with_style(&preedit.text, style).0;
    let caret_byte = preedit
        .caret
        .map(|(_, end)| clamp_text_index(&preedit.text, end))
        .unwrap_or(preedit.text.len());
    let caret_prefix = &preedit.text[..caret_byte];
    let caret_x = start_x + measurer.measure_with_style(caret_prefix, style).0;

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
    use crate::theme::dark_theme;

    #[test]
    fn single_line_scrolls_to_keep_caret_visible() {
        let theme = dark_theme();
        let props = TextBoxProps {
            label: None,
            value: std::borrow::Cow::Borrowed("hello"),
            disabled: false,
            size: Default::default(),
            density: Default::default(),
            mode: TextBoxMode::SingleLine,
            font: TextBoxFont::Body,
        };
        let spec = text_box_spec(&props, &theme).unwrap();
        let mut runtime = TextBoxRuntime::new(&spec);
        let mut measurer = TextMeasurer::new();
        runtime.sync_layout_with_style(
            Rect {
                x: 10.0,
                y: 5.0,
                w: 32.0,
                h: 20.0,
            },
            None,
            &mut measurer,
            &spec,
            text_box_value_style(&theme, spec.tokens, spec.font),
        );

        runtime.editor.move_end();
        runtime.ensure_cursor_visible();

        assert!(runtime.caret_rect().x <= runtime.content_rect.x + runtime.content_rect.w);
    }

    #[test]
    fn multiline_desired_height_uses_wrapped_line_count() {
        let theme = dark_theme();
        let props = TextBoxProps {
            label: None,
            value: std::borrow::Cow::Borrowed(
                "a long line that should wrap only when the measured field is narrow",
            ),
            disabled: false,
            size: Default::default(),
            density: Default::default(),
            mode: TextBoxMode::MultiLine { min_rows: 1 },
            font: TextBoxFont::Body,
        };
        let spec = text_box_spec(&props, &theme).unwrap();
        let mut runtime = TextBoxRuntime::new(&spec);
        let mut measurer = TextMeasurer::new();

        runtime.sync_layout_with_style(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1000.0,
                h: runtime.min_height,
            },
            None,
            &mut measurer,
            &spec,
            text_box_value_style(&theme, spec.tokens, spec.font),
        );
        let wide_height = runtime.desired_height;

        runtime.sync_layout_with_style(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 64.0,
                h: runtime.min_height,
            },
            None,
            &mut measurer,
            &spec,
            text_box_value_style(&theme, spec.tokens, spec.font),
        );

        assert_eq!(wide_height, runtime.min_height);
        assert!(runtime.desired_height > wide_height);
    }

    #[test]
    fn control_intrinsics_only_include_multiline_text_boxes() {
        let theme = dark_theme();
        let mut tree = Tree::new();
        let single = crate::ui::widget(
            "single",
            TextBoxProps {
                label: None,
                value: std::borrow::Cow::Borrowed("one"),
                disabled: false,
                size: Default::default(),
                density: Default::default(),
                mode: TextBoxMode::SingleLine,
                font: TextBoxFont::Body,
            },
        )
        .build();
        let multi = crate::ui::widget(
            "multi",
            TextBoxProps {
                label: None,
                value: std::borrow::Cow::Borrowed("one two three four five six"),
                disabled: false,
                size: Default::default(),
                density: Default::default(),
                mode: TextBoxMode::MultiLine { min_rows: 1 },
                font: TextBoxFont::Body,
            },
        )
        .build();
        let desc = crate::ui::column("root").child(single).child(multi).build();
        crate::tree::reconcile(
            &mut tree,
            desc,
            crate::widget::props::WidgetBuildCx {
                theme: &theme,
                force_rebuild: false,
            },
        );
        let root = tree.root().unwrap();
        let mut measurer = TextMeasurer::new();
        crate::tree::layout(
            &mut tree,
            root,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 200.0,
            },
            &mut |text, style| measurer.measure_with_style(text, style),
        );
        let mut store = TextBoxStore::new();
        store.sync_with_tree(&tree, &mut measurer, &theme, None);

        let intrinsics = store.control_intrinsics();

        assert_eq!(intrinsics.len(), 1);
        assert_eq!(intrinsics[0].widget_id, "multi");
    }
}
