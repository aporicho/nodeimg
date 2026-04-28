#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail=0

check_no_match() {
    local label="$1"
    local pattern="$2"
    shift 2

    local matches
    if matches="$(rg -n "$pattern" "$@" 2>/dev/null)"; then
        printf '\n[retained-ui gate] %s\n' "$label"
        printf '%s\n' "$matches"
        fail=1
    fi
}

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
    gui/src/widget/state/text_box_registry.rs

check_no_match \
    "app production retained path must not call legacy desc builders" \
    '\bDesc\b|build_workspace_tree|node_card_from_render_view|legacy_desc|OverlayRequest|Context::update' \
    app/src/app_shell.rs \
    app/src/workspace/controller.rs \
    app/src/workspace/scene_controller.rs \
    app/src/workspace/node_palette.rs

check_no_match \
    "app production must consume dirty control intrinsics instead of full intrinsic export" \
    '(^|[^[:alnum:]_])control_intrinsics\(' \
    app/src/app_shell.rs \
    app/src/workspace/controller.rs \
    app/src/workspace/scene_controller.rs

check_no_match \
    "backend resource creation must stay out of tree/template/widget layers" \
    'create_buffer|wgpu::|BackendCommandEncoder|RenderPass' \
    gui/src/template \
    gui/src/tree \
    gui/src/widget

printf '\n[retained-ui gate] known legacy boundary inventory\n'
rg -n 'Context::update|build_workspace_tree|node_card_from_render_view|legacy_desc|(^|[^[:alnum:]_])control_intrinsics\(|build_display_list' \
    gui/src app/src || true

if [[ "$fail" -ne 0 ]]; then
    printf '\nretained UI gate failed\n' >&2
    exit 1
fi

printf '\nretained UI gate passed\n'
