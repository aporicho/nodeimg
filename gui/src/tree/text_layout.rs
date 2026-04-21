use crate::renderer::{Point, Rect, TextStyle};
use crate::tree::layout::{TextAlign, TextLayout, TextOverflow};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedTextPaint {
    pub content: String,
    pub pos: Point,
    pub bounds: Option<Rect>,
}

pub(crate) fn resolve_text_paint(
    content: &str,
    style: &TextStyle,
    layout: TextLayout,
    rect: Rect,
    mut measure: impl FnMut(&str, &TextStyle) -> (f32, f32),
) -> ResolvedTextPaint {
    let (display, display_w) = match layout.overflow {
        TextOverflow::Visible | TextOverflow::Clip => {
            let width = measure(content, style).0;
            (content.to_string(), width)
        }
        TextOverflow::Ellipsis => ellipsize(content, style, rect.w, &mut measure),
    };

    let x_offset = match layout.align {
        TextAlign::Start => 0.0,
        TextAlign::Center => ((rect.w - display_w).max(0.0)) / 2.0,
        TextAlign::End => (rect.w - display_w).max(0.0),
    };

    ResolvedTextPaint {
        content: display,
        pos: Point {
            x: rect.x + x_offset,
            y: rect.y,
        },
        bounds: match layout.overflow {
            TextOverflow::Visible => None,
            TextOverflow::Clip | TextOverflow::Ellipsis => Some(rect),
        },
    }
}

fn ellipsize(
    content: &str,
    style: &TextStyle,
    max_w: f32,
    measure: &mut impl FnMut(&str, &TextStyle) -> (f32, f32),
) -> (String, f32) {
    if max_w <= 0.0 {
        return (String::new(), 0.0);
    }

    let full_w = measure(content, style).0;
    if full_w <= max_w {
        return (content.to_string(), full_w);
    }

    const ELLIPSIS: &str = "…";
    let ellipsis_w = measure(ELLIPSIS, style).0;
    if ellipsis_w > max_w {
        return (String::new(), 0.0);
    }

    let ends: Vec<usize> = content
        .char_indices()
        .map(|(index, ch)| index + ch.len_utf8())
        .collect();

    let mut low = 0usize;
    let mut high = ends.len();
    let mut best = ELLIPSIS.to_string();
    let mut best_w = ellipsis_w;

    while low <= high {
        let mid = low + (high - low) / 2;
        let candidate = if mid == 0 {
            ELLIPSIS.to_string()
        } else {
            format!("{}{}", &content[..ends[mid - 1]], ELLIPSIS)
        };
        let candidate_w = measure(&candidate, style).0;

        if candidate_w <= max_w {
            best = candidate;
            best_w = candidate_w;
            low = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }

    (best, best_w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Color;

    fn style() -> TextStyle {
        TextStyle::new(Color::WHITE, 12.0)
    }

    fn measure(text: &str, _style: &TextStyle) -> (f32, f32) {
        (text.chars().count() as f32 * 10.0, 12.0)
    }

    fn rect(width: f32) -> Rect {
        Rect {
            x: 10.0,
            y: 20.0,
            w: width,
            h: 16.0,
        }
    }

    #[test]
    fn default_text_layout_is_visible_start() {
        assert_eq!(
            TextLayout::default(),
            TextLayout {
                overflow: TextOverflow::Visible,
                align: TextAlign::Start,
            }
        );
    }

    #[test]
    fn visible_text_does_not_clip() {
        let resolved = resolve_text_paint(
            "hello",
            &style(),
            TextLayout::default(),
            rect(20.0),
            measure,
        );

        assert_eq!(resolved.content, "hello");
        assert_eq!(resolved.pos.x, 10.0);
        assert_eq!(resolved.bounds, None);
    }

    #[test]
    fn clip_text_keeps_content_and_sets_bounds() {
        let layout = TextLayout {
            overflow: TextOverflow::Clip,
            align: TextAlign::Start,
        };
        let bounds = rect(20.0);
        let resolved = resolve_text_paint("hello", &style(), layout, bounds, measure);

        assert_eq!(resolved.content, "hello");
        assert_eq!(resolved.bounds, Some(bounds));
    }

    #[test]
    fn ellipsis_keeps_content_when_it_fits() {
        let layout = TextLayout {
            overflow: TextOverflow::Ellipsis,
            align: TextAlign::Start,
        };
        let resolved = resolve_text_paint("hello", &style(), layout, rect(50.0), measure);

        assert_eq!(resolved.content, "hello");
        assert_eq!(resolved.bounds, Some(rect(50.0)));
    }

    #[test]
    fn ellipsis_truncates_to_available_width() {
        let layout = TextLayout {
            overflow: TextOverflow::Ellipsis,
            align: TextAlign::Start,
        };
        let resolved = resolve_text_paint("hello", &style(), layout, rect(30.0), measure);

        assert!(resolved.content.ends_with("…"));
        assert!(measure(&resolved.content, &style()).0 <= 30.0);
    }

    #[test]
    fn ellipsis_uses_empty_text_when_marker_does_not_fit() {
        let layout = TextLayout {
            overflow: TextOverflow::Ellipsis,
            align: TextAlign::Start,
        };
        let resolved = resolve_text_paint("hello", &style(), layout, rect(5.0), measure);

        assert_eq!(resolved.content, "");
    }

    #[test]
    fn center_and_end_align_final_display_text() {
        let center = resolve_text_paint(
            "hi",
            &style(),
            TextLayout {
                overflow: TextOverflow::Clip,
                align: TextAlign::Center,
            },
            rect(60.0),
            measure,
        );
        let end = resolve_text_paint(
            "hi",
            &style(),
            TextLayout {
                overflow: TextOverflow::Clip,
                align: TextAlign::End,
            },
            rect(60.0),
            measure,
        );

        assert_eq!(center.pos.x, 30.0);
        assert_eq!(end.pos.x, 50.0);
    }
}
