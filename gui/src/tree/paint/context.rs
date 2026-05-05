use super::TextLeafPaintOverride;
use crate::animation::AnimationStore;
use crate::theme::Theme;

pub(crate) struct PaintCx<'a> {
    pub(crate) text_override: Option<&'a dyn TextLeafPaintOverride>,
    pub(crate) animations: Option<&'a AnimationStore>,
    pub(crate) theme: &'a Theme,
}
