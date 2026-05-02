use super::retained_lookup::retained_prefixes;
use super::system::TextBoxSystem;
use crate::control::systems::SystemCx;
use crate::output::FrameworkOutput;
use crate::renderer::Point;

impl TextBoxSystem {
    pub(super) fn handle_mouse_press(
        &mut self,
        cx: &SystemCx<'_>,
        x: f32,
        y: f32,
    ) -> Option<FrameworkOutput> {
        self.active_drag_text_box = None;
        let control_id = self.text_box_id_at(cx, x, y)?;
        let point = self.text_box_pointer_point(cx, &control_id, x, y);
        let runtime = self.store.text_box_mut(&control_id)?;
        runtime.clear_preedit();
        runtime.set_caret_from_point(point.x, point.y);
        self.active_drag_text_box = Some(control_id);
        Some(FrameworkOutput::consumed())
    }

    pub(super) fn handle_mouse_move(
        &mut self,
        cx: &SystemCx<'_>,
        x: f32,
        y: f32,
    ) -> Option<FrameworkOutput> {
        let control_id = self.active_drag_text_box.clone()?;
        if self.captured_control_id_for(cx).as_deref() != Some(control_id.as_str()) {
            return None;
        }
        let point = self.text_box_pointer_point(cx, &control_id, x, y);
        let runtime = self.store.text_box_mut(&control_id)?;
        runtime.select_to_point(point.x, point.y);
        Some(FrameworkOutput::consumed())
    }

    pub(super) fn handle_mouse_release(&mut self) -> FrameworkOutput {
        let consumed = self.active_drag_text_box.is_some();
        self.active_drag_text_box = None;
        FrameworkOutput::default().with_consumed(consumed)
    }

    fn text_box_id_at(&self, cx: &SystemCx<'_>, x: f32, y: f32) -> Option<String> {
        cx.hit_chain(x, y).iter().find_map(|node_id| {
            let name = cx.node_name(node_id)?;
            if self.store.text_box(name).is_some() {
                return Some(name.to_string());
            }
            retained_prefixes(name).find_map(|prefix| {
                self.store
                    .text_box(prefix)
                    .is_some()
                    .then(|| prefix.to_string())
            })
        })
    }

    fn text_box_pointer_point(&self, cx: &SystemCx<'_>, control_id: &str, x: f32, y: f32) -> Point {
        let field_id = format!("{control_id}::field");
        cx.node_id_by_name(&field_id)
            .or_else(|| cx.node_id_by_name(control_id))
            .and_then(|node_id| cx.screen_to_node_layout_point(node_id, x, y))
            .unwrap_or(Point { x, y })
    }
}
