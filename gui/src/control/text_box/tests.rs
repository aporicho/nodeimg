use super::{TextBoxRuntime, TextBoxSpec, TextBoxValueKind};
use crate::control::{text_box_value_style, TextBoxFont, TextBoxMode};
use crate::renderer::{Rect, TextMeasurer};
use crate::text::TextLayoutCache;
use crate::theme::{dark_theme, Theme};

fn text_spec(
    theme: &Theme,
    external_text: &str,
    mode: TextBoxMode,
    font: TextBoxFont,
) -> TextBoxSpec {
    TextBoxSpec {
        external_text: external_text.to_string(),
        mode,
        value_kind: TextBoxValueKind::Text,
        tokens: theme.text_field_metrics(Default::default(), Default::default()),
        font,
        disabled: false,
    }
}

#[test]
fn single_line_scrolls_to_keep_caret_visible() {
    let theme = dark_theme();
    let spec = text_spec(&theme, "hello", TextBoxMode::SingleLine, TextBoxFont::Body);
    let mut runtime = TextBoxRuntime::new(&spec);
    let mut measurer = TextMeasurer::new();
    let mut layout_cache = TextLayoutCache::default();
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
        &mut layout_cache,
    );

    runtime.editor.move_end();
    runtime.ensure_cursor_visible();

    assert!(runtime.caret_rect().x <= runtime.content_rect.x + runtime.content_rect.w);
}

#[test]
fn multiline_desired_height_uses_wrapped_line_count() {
    let theme = dark_theme();
    let spec = text_spec(
        &theme,
        "a long line that should wrap only when the measured field is narrow",
        TextBoxMode::MultiLine { min_rows: 1 },
        TextBoxFont::Body,
    );
    let mut runtime = TextBoxRuntime::new(&spec);
    let mut measurer = TextMeasurer::new();
    let mut layout_cache = TextLayoutCache::default();

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
        &mut layout_cache,
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
        &mut layout_cache,
    );

    assert_eq!(wide_height, runtime.min_height);
    assert!(runtime.desired_height > wide_height);
}

#[test]
fn cursor_only_change_reuses_text_layout_cache() {
    let theme = dark_theme();
    let spec = text_spec(
        &theme,
        "cached multiline text",
        TextBoxMode::MultiLine { min_rows: 1 },
        TextBoxFont::Body,
    );
    let mut runtime = TextBoxRuntime::new(&spec);
    let mut measurer = TextMeasurer::new();
    let mut layout_cache = TextLayoutCache::default();
    let rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 180.0,
        h: runtime.min_height,
    };

    let first = runtime.sync_layout_with_style(
        rect,
        None,
        &mut measurer,
        &spec,
        text_box_value_style(&theme, spec.tokens, spec.font),
        &mut layout_cache,
    );
    runtime.editor.move_end();
    let second = runtime.sync_layout_with_style(
        rect,
        None,
        &mut measurer,
        &spec,
        text_box_value_style(&theme, spec.tokens, spec.font),
        &mut layout_cache,
    );

    assert!(!first.cache_hit);
    assert!(second.cache_hit);
    assert!(!second.desired_height_changed);
}
