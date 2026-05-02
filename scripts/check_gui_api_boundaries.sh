#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail=0

gate_label="gui-api gate"

check_no_match() {
    local label="$1"
    local pattern="$2"
    shift 2

    local matches
    if matches="$(rg -n "$pattern" "$@" 2>/dev/null)"; then
        printf '\n[%s] %s\n' "$gate_label" "$label"
        printf '%s\n' "$matches"
        fail=1
    fi
}

check_required_match() {
    local label="$1"
    local pattern="$2"
    shift 2

    if ! rg -n "$pattern" "$@" >/dev/null 2>&1; then
        printf '\n[%s] missing required boundary: %s\n' "$gate_label" "$label"
        printf 'pattern: %s\n' "$pattern"
        fail=1
    fi
}

check_missing_path() {
    local label="$1"
    local path="$2"

    if [[ -e "$path" ]]; then
        printf '\n[%s] removed path still exists: %s\n' "$gate_label" "$label"
        printf 'path: %s\n' "$path"
        fail=1
    fi
}

if ! command -v rg >/dev/null 2>&1; then
    printf '[%s] ripgrep (rg) is required\n' "$gate_label" >&2
    exit 1
fi

check_required_match "gui tree module remains crate-private" '^pub\(crate\) mod tree;' gui/src/lib.rs
check_required_match "gui runtime module remains crate-private" '^pub\(crate\) mod runtime;' gui/src/lib.rs
check_required_match "gui event module remains crate-private" '^pub\(crate\) mod event;' gui/src/lib.rs
check_required_match "gui text module remains crate-private" '^pub\(crate\) mod text;' gui/src/lib.rs
check_required_match "gui control facade is public" '^pub mod control;' gui/src/lib.rs
check_required_match "gui layout facade is public" '^pub mod layout;' gui/src/lib.rs
check_required_match "gui scene facade is public" '^pub mod scene;' gui/src/lib.rs
check_missing_path "old retained UI gate compatibility wrapper" scripts/check_retained_ui_gates.sh
check_missing_path "old widget module tree" gui/src/widget
check_missing_path "old ui convenience module" gui/src/ui.rs
check_missing_path "old desc tree module" gui/src/tree/desc.rs
check_missing_path "old desc build module" gui/src/tree/build
check_missing_path "old event gesture adapter module" gui/src/event/gesture_adapter.rs
check_missing_path "old text intrinsic module" gui/src/text/intrinsic.rs
check_missing_path "legacy full-tree desc reconciler" gui/src/tree/legacy_desc.rs
check_missing_path "legacy full-tree desc diff" gui/src/tree/diff.rs
check_missing_path "legacy Desc canvas node-card builder" gui/src/canvas/node_card.rs
check_missing_path "legacy Desc panel root composer" gui/src/panel/root.rs
check_missing_path "legacy Desc overlay composer" gui/src/overlay/builder.rs

check_no_match \
    "app must not import gui crate-private/internal modules" \
    'gui::(tree|ui|runtime|event|text|widget)(::|;|\{)' \
    app/src

check_no_match \
    "old widget module and event/action names must not be restored" \
    'widget|Widget' \
    app/src \
    gui/src

check_no_match \
    "legacy gui facade must not be restored" \
    'gui::legacy|crate::legacy|pub mod legacy|legacy::' \
    app/src \
    gui/src

check_no_match \
    "semantic roles must use ControlRole instead of string roles" \
    '\.with_semantic_role\("|from_semantic_role|semantic_role: Option<Cow' \
    app/src \
    gui/src

check_no_match \
    "template compiled/generated internals must not be public API" \
    'pub use compiled|pub (struct|enum) [A-Za-z0-9_]*Compiled|pub fn [a-z_]+_template\(' \
    gui/src/template

check_no_match \
    "removed canvas/panel helper modules must not be restored as public API" \
    'pub mod (connection_paint_cache|endpoint_cache|node_card|event|root)|pub\(crate\) mod (node_card|root)|CanvasConnectionPaintCache|CanvasEndpointCache' \
    gui/src/canvas \
    gui/src/panel

check_no_match \
    "legacy full-tree desc/reconcile boundary must not be restored" \
    'legacy_desc|node_card_from_render_view|Context::update|compose_desc|panel_root\(|pub use .*reconcile|pub\(crate\) use .*reconcile|mod diff|build_display_list\(' \
    app/src \
    gui/src

check_no_match \
    "OverlayRequest must use structured OverlayContent instead of Desc" \
    'content: Desc|OverlayRequest \{[^}]*content: .*Desc' \
    gui/src \
    app/src

check_no_match \
    "app must use Context capability APIs instead of legacy direct Context methods" \
    'gui\.(node_id_by_name|node_exists|node_rect|node_name|hit_test|resize_hit_at_screen_point|flush_layout_dirty|handle_event|handle_panel_event|ensure_panel_runtime|sync_canvas_node_layouts|export_canvas_node_layouts|import_canvas_node_layouts|move_canvas_node_by|resize_canvas_node_by|apply_canvas_node_sizing|canvas_port_group_view|toggle_canvas_port_group|select_canvas_node|clear_canvas_selection|is_canvas_node_selected|pending_canvas_connection|begin_pending_canvas_connection|update_pending_canvas_connection|end_pending_canvas_connection|cancel_pending_canvas_connection|hovered_canvas_port_id|set_hovered_canvas_port)' \
    app/src

check_no_match \
    "retained template and scene modules must not import legacy desc/build/reconcile paths" \
    '\bDesc\b|WidgetProps::build|reconcile\(|legacy_desc|node_card_from_render_view|build_workspace_tree' \
    gui/src/template \
    gui/src/canvas/scene_model.rs \
    gui/src/canvas/scene_diff.rs \
    app/src/workspace/scene_controller.rs

check_no_match \
    "retained mutation and runtime modules must not use widget builders as glue" \
    '\bDesc\b|WidgetProps::build|reconcile\(|build_display_list|node_card_from_render_view' \
    gui/src/tree/mutation.rs \
    gui/src/tree/retained_runtime.rs \
    gui/src/tree/dirty.rs \
    gui/src/control/state/text_box_registry.rs

check_no_match \
    "app production retained path must not call legacy desc builders" \
    '\bDesc\b|build_workspace_tree|node_card_from_render_view|legacy_desc|Context::update' \
    app/src/app_shell.rs \
    app/src/workspace/controller.rs \
    app/src/workspace/scene_controller.rs \
    app/src/workspace/node_palette.rs

check_no_match \
    "app production must use the ControlsApi intrinsic query only through query/update flow" \
    '(^|[^[:alnum:]_])control_intrinsics\(' \
    app/src/app_shell.rs \
    app/src/workspace/controller.rs \
    app/src/workspace/scene_controller.rs

check_no_match \
    "panel drag/resize must use ControlEvent, not a parallel PanelEvent channel" \
    'PanelEvent|GuiEvent::Panel' \
    app/src \
    gui/src

check_no_match \
    "control intrinsic API must not expose snapshot aliases" \
    'intrinsics_snapshot|control_intrinsics_snapshot' \
    app/src \
    gui/src

check_no_match \
    "app-facing ControlsApi must not expose text-box-specific dirty intrinsic state" \
    'text_box_dirty_intrinsics|take_text_box_dirty_intrinsics' \
    app/src \
    gui/src

check_no_match \
    "GestureArena must not take target identity in its constructor" \
    'GestureArena::new\([^)]' \
    gui/src/gesture \
    app/src

check_no_match \
    "backend resource creation must stay out of tree/template/control layers" \
    'create_buffer|wgpu::|BackendCommandEncoder|RenderPass' \
    gui/src/template \
    gui/src/tree \
    gui/src/control

check_no_match \
    "hittable must be an explicit bool, not an Option compatibility surface" \
    'hittable: Option|hittable: Some|hittable = Some|hittable=Some' \
    gui/src \
    app/src

check_no_match \
    "old frame stats must not be restored" \
    'widget_build_calls|full_tree_scans|reconcile_child_matches|full_root_paint_calls|record_full_tree_scan' \
    gui/src \
    app/src

if [[ "$fail" -ne 0 ]]; then
    printf '\nGUI API boundary gate failed\n' >&2
    exit 1
fi

printf '\nGUI API boundary gate passed\n'
