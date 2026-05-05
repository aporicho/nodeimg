use super::{painter::paint_text_box_leaf, TextBoxStore};
use crate::interaction::InteractionState;
use crate::tree::paint_target::PaintTarget;
use crate::tree::{TextLeafPaintOverride, TextLeafPaintRequest};

pub(crate) struct TextBoxPaintOverride<'a> {
    interaction: &'a InteractionState,
    text_boxes: &'a TextBoxStore,
}

impl<'a> TextBoxPaintOverride<'a> {
    pub(crate) fn new(interaction: &'a InteractionState, text_boxes: &'a TextBoxStore) -> Self {
        Self {
            interaction,
            text_boxes,
        }
    }
}

impl TextLeafPaintOverride for TextBoxPaintOverride<'_> {
    fn paint_text_leaf(
        &self,
        target: &mut dyn PaintTarget,
        request: TextLeafPaintRequest<'_>,
    ) -> bool {
        paint_text_box_leaf(target, request, self.interaction, self.text_boxes)
    }
}
