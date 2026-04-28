use crate::renderer::{Point, Rect, TextMeasurer, TextStyle};

use super::line_break::grapheme_spans;
use super::metrics::TextMetrics;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TextLayoutPolicy {
    NoWrap,
    Wrap,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextLayoutResult {
    pub lines: Vec<TextLine>,
    pub line_height: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextLine {
    pub start: usize,
    pub end: usize,
    pub width: f32,
    pub y: f32,
    pub stops: Vec<(usize, f32)>,
}

impl TextLayoutResult {
    pub(crate) fn line_for_index(&self, index: usize) -> Option<&TextLine> {
        self.lines
            .iter()
            .find(|line| index >= line.start && index <= line.end)
            .or_else(|| self.lines.last())
    }

    pub(crate) fn caret_point(&self, index: usize) -> Point {
        let Some(line) = self.line_for_index(index) else {
            return Point { x: 0.0, y: 0.0 };
        };
        Point {
            x: line.x_for_index(index),
            y: line.y,
        }
    }

    pub(crate) fn nearest_index(&self, x: f32, y: f32) -> usize {
        let Some(line) = self.lines.iter().min_by(|lhs, rhs| {
            let lhs_distance = (lhs.y + self.line_height * 0.5 - y).abs();
            let rhs_distance = (rhs.y + self.line_height * 0.5 - y).abs();
            lhs_distance.total_cmp(&rhs_distance)
        }) else {
            return 0;
        };
        line.nearest_index(x)
    }

    pub(crate) fn selection_rects(&self, start: usize, end: usize) -> Vec<Rect> {
        self.lines
            .iter()
            .filter_map(|line| {
                let line_start = line.start.max(start);
                let line_end = line.end.min(end);
                if line_start >= line_end {
                    return None;
                }
                let x0 = line.x_for_index(line_start);
                let x1 = line.x_for_index(line_end);
                Some(Rect {
                    x: x0,
                    y: line.y,
                    w: (x1 - x0).max(1.0),
                    h: self.line_height.max(1.0),
                })
            })
            .collect()
    }
}

impl TextLine {
    pub(crate) fn x_for_index(&self, index: usize) -> f32 {
        let index = index.clamp(self.start, self.end);
        let mut last_x = 0.0;
        for (stop, x) in &self.stops {
            if *stop > index {
                break;
            }
            last_x = *x;
        }
        last_x
    }

    fn nearest_index(&self, x: f32) -> usize {
        self.stops
            .iter()
            .min_by(|(_, lhs_x), (_, rhs_x)| {
                let lhs_distance = (lhs_x - x).abs();
                let rhs_distance = (rhs_x - x).abs();
                lhs_distance.total_cmp(&rhs_distance)
            })
            .map(|(index, _)| *index)
            .unwrap_or(self.end)
    }
}

pub(crate) fn layout_text(
    text: &str,
    policy: TextLayoutPolicy,
    max_width: f32,
    measurer: &mut TextMeasurer,
    style: &TextStyle,
) -> TextLayoutResult {
    let mut metrics = TextMetrics::new(measurer, style);
    match policy {
        TextLayoutPolicy::NoWrap => layout_no_wrap_text(text, max_width, &mut metrics),
        TextLayoutPolicy::Wrap => layout_wrapped_text(text, max_width, &mut metrics),
    }
}

fn layout_no_wrap_text(
    text: &str,
    max_width: f32,
    metrics: &mut TextMetrics<'_>,
) -> TextLayoutResult {
    let line_height = metrics.line_height();
    let content_width = metrics.measure(text).0;
    let mut lines = Vec::new();
    push_line(
        &mut lines,
        text,
        0,
        text.len(),
        content_width,
        line_height,
        metrics,
    );

    TextLayoutResult {
        lines,
        line_height,
        width: max_width.max(1.0),
        height: line_height,
    }
}

fn layout_wrapped_text(
    text: &str,
    max_width: f32,
    metrics: &mut TextMetrics<'_>,
) -> TextLayoutResult {
    let line_height = metrics.line_height();
    let width = max_width.max(1.0);
    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let mut current_end = 0usize;
    let mut current_width = 0.0f32;

    for span in grapheme_spans(text) {
        if span.text == "\n" || span.text == "\r\n" {
            push_line(
                &mut lines,
                text,
                line_start,
                span.start,
                current_width,
                line_height,
                metrics,
            );
            line_start = span.end;
            current_end = line_start;
            current_width = 0.0;
            continue;
        }

        let grapheme_width = metrics.measure(span.text).0;
        let next_width = current_width + grapheme_width;
        if next_width > width && current_end > line_start {
            push_line(
                &mut lines,
                text,
                line_start,
                current_end,
                current_width,
                line_height,
                metrics,
            );
            line_start = span.start;
            current_end = span.end;
            current_width = grapheme_width;
        } else {
            current_end = span.end;
            current_width = next_width;
        }
    }

    push_line(
        &mut lines,
        text,
        line_start,
        text.len(),
        current_width,
        line_height,
        metrics,
    );

    let height = lines.len() as f32 * line_height;
    TextLayoutResult {
        lines,
        line_height,
        width,
        height,
    }
}

fn push_line(
    lines: &mut Vec<TextLine>,
    text: &str,
    start: usize,
    end: usize,
    width: f32,
    line_height: f32,
    metrics: &mut TextMetrics<'_>,
) {
    let y = lines.len() as f32 * line_height;
    let mut stops = Vec::new();
    stops.push((start, 0.0));
    let mut x = 0.0;
    for span in grapheme_spans(&text[start..end]) {
        let byte_end = start + span.end;
        x += metrics.measure(span.text).0;
        stops.push((byte_end, x));
    }
    if stops.last().map(|(index, _)| *index) != Some(end) {
        stops.push((end, width));
    } else if let Some((_, x)) = stops.last_mut() {
        *x = width;
    }

    lines.push(TextLine {
        start,
        end,
        width,
        y,
        stops,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Color, TextStyle};

    #[test]
    fn wrap_text_breaks_long_lines_by_width() {
        let mut measurer = TextMeasurer::new();
        let style = TextStyle::new(Color::WHITE, 12.0);
        let layout = layout_text(
            "abcdef",
            TextLayoutPolicy::Wrap,
            20.0,
            &mut measurer,
            &style,
        );

        assert!(layout.lines.len() > 1);
        assert!(layout.height >= layout.line_height * 2.0);
    }

    #[test]
    fn selection_rects_can_span_multiple_lines() {
        let mut measurer = TextMeasurer::new();
        let style = TextStyle::new(Color::WHITE, 12.0);
        let layout = layout_text(
            "hello\nworld",
            TextLayoutPolicy::Wrap,
            120.0,
            &mut measurer,
            &style,
        );

        assert_eq!(layout.selection_rects(1, 8).len(), 2);
    }

    #[test]
    fn no_wrap_layout_keeps_long_text_on_one_line() {
        let mut measurer = TextMeasurer::new();
        let style = TextStyle::new(Color::WHITE, 12.0);
        let layout = layout_text(
            "abcdef",
            TextLayoutPolicy::NoWrap,
            20.0,
            &mut measurer,
            &style,
        );

        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.height, layout.line_height);
        assert!(layout.lines[0].width > layout.width);
    }

    #[test]
    fn wrap_text_keeps_grapheme_clusters_intact() {
        let mut measurer = TextMeasurer::new();
        let style = TextStyle::new(Color::WHITE, 12.0);
        let text = "a👨‍👩‍👧‍👦e\u{301}b";
        let boundaries = grapheme_spans(text)
            .flat_map(|span| [span.start, span.end])
            .collect::<std::collections::BTreeSet<_>>();

        let layout = layout_text(text, TextLayoutPolicy::Wrap, 1.0, &mut measurer, &style);

        assert!(layout.lines.len() >= 3);
        for line in layout.lines {
            assert!(boundaries.contains(&line.start));
            assert!(boundaries.contains(&line.end));
            for (stop, _) in line.stops {
                assert!(boundaries.contains(&stop));
            }
        }
    }
}
