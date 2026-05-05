use crate::control::text_box::model::{TextBoxFont, TextBoxMode, TextBoxValueKind};
use crate::control::text_box::preedit::{PreeditLayout, PreeditState};
use crate::renderer::{Point, Rect};
use crate::text::layout::TextLayoutResult;
use crate::text::TextEditState;
use crate::theme::TextInputTheme;

pub(crate) struct TextBoxRuntime {
    pub(in crate::control::text_box) editor: TextEditState,
    pub(in crate::control::text_box) last_external_text: String,
    pub(in crate::control::text_box) mode: TextBoxMode,
    pub(in crate::control::text_box) value_kind: TextBoxValueKind,
    pub(in crate::control::text_box) tokens: TextInputTheme,
    pub(in crate::control::text_box) font: TextBoxFont,
    pub(in crate::control::text_box) disabled: bool,
    pub(in crate::control::text_box) field_rect: Rect,
    pub(in crate::control::text_box) content_rect: Rect,
    pub(in crate::control::text_box) text_origin: Point,
    pub(in crate::control::text_box) layout: TextLayoutResult,
    pub(in crate::control::text_box) min_height: f32,
    pub(in crate::control::text_box) desired_height: f32,
    pub(in crate::control::text_box) scroll_x: f32,
    pub(in crate::control::text_box) preedit: Option<PreeditState>,
    pub(in crate::control::text_box) preedit_layout: Option<PreeditLayout>,
}
