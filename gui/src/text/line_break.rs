use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GraphemeSpan<'a> {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: &'a str,
}

pub(crate) fn grapheme_spans(text: &str) -> impl Iterator<Item = GraphemeSpan<'_>> {
    text.grapheme_indices(true)
        .map(|(start, grapheme)| GraphemeSpan {
            start,
            end: start + grapheme.len(),
            text: grapheme,
        })
}
