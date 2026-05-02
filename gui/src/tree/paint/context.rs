use crate::animation::AnimationStore;
use crate::control::state::TextBoxStore;
use crate::interaction::InteractionState;
use crate::theme::Theme;

pub(crate) struct PaintCx<'a> {
    pub(crate) interaction: Option<&'a InteractionState>,
    pub(crate) text_boxes: Option<&'a TextBoxStore>,
    pub(crate) animations: Option<&'a AnimationStore>,
    pub(crate) theme: &'a Theme,
}
