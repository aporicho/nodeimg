use crate::renderer::{TextMeasurer, TextStyle};

pub(crate) struct TextMetrics<'a> {
    measurer: &'a mut TextMeasurer,
    style: &'a TextStyle,
}

impl<'a> TextMetrics<'a> {
    pub(crate) fn new(measurer: &'a mut TextMeasurer, style: &'a TextStyle) -> Self {
        Self { measurer, style }
    }

    pub(crate) fn measure(&mut self, text: &str) -> (f32, f32) {
        self.measurer.measure_with_style(text, self.style)
    }

    pub(crate) fn line_height(&mut self) -> f32 {
        self.measure("Mg")
            .1
            .max(self.style.size * self.style.line_height)
    }
}
