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
check_required_match "non-text controls use a dedicated interaction runtime" 'ControlInteractionSystem' gui/src/control/interaction/mod.rs gui/src/runtime/systems.rs
check_required_match "control spec module has a facade" '^pub use model::ControlSpec;' gui/src/control/spec/mod.rs
check_required_match "control layout module has a facade" '^pub use measure::' gui/src/control/layout/mod.rs
check_required_match "control mount module has a facade" '^pub\(crate\) use wrapper::' gui/src/control/mount/mod.rs
check_required_match "control kinds have a single registry" 'pub\(crate\) fn control_kind' gui/src/control/kinds/registry.rs
check_required_match "control kind descriptor API exists" 'struct ControlKindDescriptor' gui/src/control/kinds/descriptor.rs
check_required_match "control registry dispatches through descriptors" 'CONTROL_DESCRIPTORS' gui/src/control/kinds/registry.rs
check_required_match "workspace owns canvas text-box sync adaptation" 'ControlTextBoxSyncItem' app/src/workspace/control_sync.rs
check_required_match "tree node construction has a standard builder module" 'pub\(crate\) use builder::TreeNodeBuilder;' gui/src/tree/node_factory/mod.rs
check_required_match "tree facade exports the standard node builder internally" 'pub\(crate\) use node_factory::TreeNodeBuilder;' gui/src/tree/mod.rs
check_required_match "tree props module owns semantic role facade" '^pub\(crate\) use semantic_role::SemanticRole;' gui/src/tree/props/mod.rs
check_required_match "tree facade exports semantic role internally" '^pub\(crate\) use props::SemanticRole;' gui/src/tree/mod.rs
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
check_missing_path "old flat canvas retained node card module" gui/src/canvas/retained_node_card.rs
check_missing_path "legacy Desc panel root composer" gui/src/panel/root.rs
check_missing_path "old flat panel retained module" gui/src/panel/retained.rs
check_missing_path "legacy Desc overlay composer" gui/src/overlay/builder.rs
check_missing_path "old flat overlay retained module" gui/src/overlay/retained.rs
check_missing_path "old flat control text box state module" gui/src/control/state/text_box.rs
check_missing_path "old flat control text box system module" gui/src/control/systems/text_box.rs
check_missing_path "old flat control text box runtime module" gui/src/control/text_box/runtime.rs
check_missing_path "old control state module tree" gui/src/control/state
check_missing_path "old control systems module tree" gui/src/control/systems
check_missing_path "old control mapping module" gui/src/control/mapping.rs
check_missing_path "old control param layout module" gui/src/control/param_layout.rs
check_missing_path "old control painter module" gui/src/control/painter.rs
check_missing_path "old control-owned semantic role module" gui/src/control/role.rs
check_missing_path "old flat control spec module" gui/src/control/spec.rs
check_missing_path "old flat control layout module" gui/src/control/layout.rs
check_missing_path "old control template module tree" gui/src/control/templates
check_missing_path "old canvas retained control template module tree" gui/src/canvas/retained_node_card/controls
check_missing_path "old panel retained control composer" gui/src/panel/retained/controls.rs
check_missing_path "old panel retained text helper" gui/src/panel/retained/text.rs
check_missing_path "old flat renderer display backend module" gui/src/renderer/display_backend.rs
check_missing_path "old flat renderer prepare module" gui/src/renderer/prepare.rs
check_missing_path "old flat renderer dispatch module" gui/src/renderer/dispatch.rs
check_missing_path "old flat renderer core module" gui/src/renderer/core.rs
check_missing_path "old flat renderer svg vector module" gui/src/renderer/svg/vector.rs
check_missing_path "old flat Tree implementation module" gui/src/tree/tree.rs
check_missing_path "old flat tree hit query module" gui/src/tree/hit.rs
check_missing_path "old flat tree hit order module" gui/src/tree/hit_order.rs
check_missing_path "old flat tree leaf hit shape module" gui/src/tree/hit_shape.rs
check_missing_path "old flat tree interaction hit module" gui/src/tree/interaction_hit.rs
check_missing_path "old flat tree container shape module" gui/src/tree/shape.rs
check_missing_path "old flat tree paint module" gui/src/tree/paint.rs
check_missing_path "old flat tree layout arrange module" gui/src/tree/layout/arrange.rs
check_missing_path "old flat panel runtime module" gui/src/panel/runtime.rs
check_missing_path "old panel reducer test holder" gui/src/panel/reducer.rs
check_missing_path "old GUI business panel instances directory" gui/src/panel/instances
check_missing_path "old flat canvas runtime module" gui/src/canvas/runtime.rs

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
    "semantic roles must use tree SemanticRole instead of old ControlRole or string roles" \
    'ControlRole|\.with_semantic_role\("|from_semantic_role|semantic_role: Option<Cow' \
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
    gui/src/control/text_box/registry.rs

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
    "template payload must use panel facade instead of retained internals for panel data API" \
    'panel::retained::PanelFrameTemplateData' \
    gui/src/template/payload.rs

check_no_match \
    "template payload and overlay system must use overlay facade instead of retained internals for overlay data API" \
    'overlay::retained::DropdownOverlayTemplateData' \
    gui/src/template/payload.rs \
    gui/src/overlay/system.rs

check_no_match \
    "panel drag/resize must use ControlEvent, not a parallel PanelEvent channel" \
    'PanelEvent|GuiEvent::Panel' \
    app/src \
    gui/src

check_no_match \
    "app panels must be registered through app/src/panels registry" \
    'WorkspacePanelComposition|retained_panels\(' \
    app/src/workspace

check_no_match \
    "GUI panel content template must stay generic, not app-business-specific" \
    'PanelContentTemplate::(Toolbar|Preview|Engine)' \
    gui/src \
    app/src

check_no_match \
    "controls must use unified ControlSpec API, not legacy param-control or panel-body names" \
    'ParamControlSpec|ParamControlMap|ParamControlMetrics|ParamControlKind|ParamControlHeight|ParamControlLayoutPolicy|param_control_(layout_policy|min_height|kind|template)|PARAM_CONTROL_TEMPLATE|PanelBodyNode|ControlSpecMap' \
    gui/src \
    app/src

check_no_match \
    "control module must not depend on canvas business/editor modules" \
    'crate::canvas|gui::canvas' \
    gui/src/control

check_no_match \
    "control registry must not import concrete control mount functions" \
    'mount_(button|color_control|file_path|group|image|label|number|read_only|select|slider|text|text_area|toggle)' \
    gui/src/control/kinds/registry.rs

check_no_match \
    "app shell must not own canvas text-box sync item construction" \
    'fn canvas_text_box_sync_items|ControlTextBoxSyncItem|canvas_node_stable_id' \
    app/src/app_shell.rs

check_no_match \
    "control value changes must use ControlValue and ValueChanged, not split value events" \
    'ControlEvent::(TextChanged|NumberChanged|SelectionChanged)|(TextChanged|NumberChanged|SelectionChanged)[[:space:]]*\{|ControlTextChanged' \
    gui/src \
    app/src

check_no_match \
    "control intrinsic API must not expose snapshot aliases" \
    'intrinsics_snapshot|control_intrinsics_snapshot' \
    app/src \
    gui/src

check_no_match \
    "external GUI modules must construct tree nodes through TreeNodeBuilder" \
    '(^[[:space:]]*|[({=,][[:space:]]*)TreeNode[[:space:]]*\{' \
    gui/src/control \
    gui/src/canvas \
    gui/src/panel \
    gui/src/overlay \
    gui/src/template \
    gui/src/context \
    gui/src/event \
    app/src

check_no_match \
    "external GUI modules must not fill TreeNode internal defaults directly" \
    'NodeProps::default|NodeLocalRuntime::default|NodeLayoutMeta::|NodePaintMeta::|NodeMutationMeta::|RuntimeSlots::default|StableId::from' \
    gui/src/control \
    gui/src/canvas \
    gui/src/panel \
    gui/src/overlay \
    gui/src/template \
    gui/src/context \
    gui/src/event \
    app/src

check_no_match \
    "local retained node factory extensions must not replace the standard TreeNodeBuilder API" \
    'TreeNodeExt|with_semantic_role|with_runtime_slot|with_layout_boundary|with_paint_boundary|with_rect_move_invalidation|with_owner' \
    gui/src/control \
    gui/src/canvas \
    gui/src/panel \
    gui/src/overlay

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
    "tree module must not depend on canvas or panel domains" \
    'crate::(canvas|panel)|super::(canvas|panel)' \
    gui/src/tree

check_no_match \
    "old flat tree hit modules must not be referenced" \
    '(mod (hit_order|hit_shape|interaction_hit|shape);|use (crate::tree|super)::(hit_order|hit_shape|interaction_hit|shape)|::(hit_order|hit_shape|interaction_hit|shape)::)' \
    gui/src/tree

check_no_match \
    "tree hit module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/tree/hit/mod.rs

check_no_match \
    "tree paint module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/tree/paint/mod.rs

check_no_match \
    "tree layout arrange module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/tree/layout/arrange/mod.rs

check_no_match \
    "canvas retained node card module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/canvas/retained_node_card/mod.rs

check_no_match \
    "control module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/control/mod.rs

check_no_match \
    "control spec module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/control/spec/mod.rs

check_no_match \
    "control layout module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/control/layout/mod.rs

check_no_match \
    "control mount module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/control/mount/mod.rs

check_no_match \
    "control kinds module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/control/kinds/mod.rs

check_no_match \
    "panel retained module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/panel/retained/mod.rs

check_no_match \
    "overlay retained module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/overlay/retained/mod.rs

check_no_match \
    "control text box module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/control/text_box/mod.rs

check_no_match \
    "control text box runtime module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/control/text_box/runtime/mod.rs

check_no_match \
    "renderer display backend module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/renderer/display_backend/mod.rs

check_no_match \
    "renderer prepare module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/renderer/prepare/mod.rs

check_no_match \
    "renderer dispatch module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/renderer/dispatch/mod.rs

check_no_match \
    "renderer core module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/renderer/core/mod.rs

check_no_match \
    "renderer svg vector module root must only declare and re-export submodules" \
    '^[[:space:]]*(pub[[:space:]]+)?(struct|enum|fn|impl)[[:space:]]' \
    gui/src/renderer/svg/vector/mod.rs

check_no_match \
    "Tree must not expose canvas or panel runtime APIs" \
    'fn (ensure_panel|panel_state|panel_state_mut|export_panel_layouts|import_panel_layouts|move_panel_by|resize_panel_by|show_panel|hide_panel|toggle_panel|bring_panel_to_front|start_panel_drag|move_panel_drag|end_panel_drag|start_panel_resize|move_panel_resize|end_panel_resize|sync_canvas_node_layouts|export_canvas_node_layouts|import_canvas_node_layouts|move_canvas_node_by|resize_canvas_node_by|ensure_canvas_node_min_size|apply_canvas_node_sizing|canvas_port_group_view|toggle_canvas_port_group|select_canvas_node|clear_canvas_selection|is_canvas_node_selected|pending_canvas_connection|begin_pending_canvas_connection|update_pending_canvas_connection|end_pending_canvas_connection|cancel_pending_canvas_connection|hovered_canvas_port_id|set_hovered_canvas_port)\b' \
    gui/src/tree

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
