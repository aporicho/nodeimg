# Phase 5: Cleanup And Compile-Time Templates

Date: 2026-04-28

## Goal

收口 Phase 1-4 的迁移，删除生产旧路径，并把静态 UI 结构提升为 compiled/static templates：

```text
编译/启动期：生成或注册 CompiledTemplate
运行期：TemplateRegistry mount instance
状态变化：TreeMutation patch instance state
渲染帧：dirty layout/paint/render cache flush
```

Phase 5 不是重写，而是收敛：前四阶段中所有临时兼容层必须被删除、转正为标准 API，或明确标记 legacy/test。

## File Structure

新增和调整：

```text
gui/src/template/
  compiled.rs
  generated.rs
  slots.rs
  revision.rs

gui/src/tree/
  retained_runtime.rs

docs/
  architecture/
    retained-ui-render-path.md

scripts/ or xtask/
  check_gui_api_boundaries
```

可能接入：

```text
build.rs
app/build.rs
```

规则：

- 如果项目已有 build-time panel generation，必须并入 `TemplateRegistry` 或迁移删除。
- 不允许存在两套互不兼容的 template/codegen 系统。
- generated/static template 输出集中在 `gui/src/template` 或 `OUT_DIR` include，不散落在 app/workspace 模块。

## Standard APIs

### Compiled Template

```rust
pub struct CompiledTemplate {
    pub id: TemplateId,
    pub revision: TemplateRevision,
    pub root: CompiledNode,
    pub slots: TemplateSlots,
    pub boundaries: BoundaryDeclarations,
}

pub struct TemplateInstance {
    pub template: TemplateId,
    pub instance: InstanceId,
    pub root_node: NodeId,
}
```

模板包含：

- static node structure
- default styles
- slot schema
- relayout/repaint boundary declarations
- stable id suffixes
- default accessibility/hit shape metadata

模板不包含：

- runtime text value
- focus/hover/selected
- cursor/selection
- scroll offsets
- measured layout
- paint fragments
- renderer resource handles
- live NodeId

### Template Registry

```rust
pub struct TemplateRegistry {
    templates: HashMap<TemplateId, CompiledTemplate>,
}

impl TemplateRegistry {
    pub fn register(&mut self, template: CompiledTemplate);
    pub fn instantiate(
        &self,
        tree: &mut Tree,
        template: TemplateId,
        instance: InstanceId,
        parent: NodeId,
        slots: SlotValues,
    ) -> TemplateInstance;
}
```

规则：

- `TemplateRegistry` 是唯一生产实例化入口。
- codegen 只生成 `CompiledTemplate`，不绕过 registry 直接改 tree。
- dev hot reload 可以生成 template 或 mutation，但不能成为第二套 renderer。

## Migration Checklist

### 1. Legacy API Deprecation

- [ ] 标记 `Context::update(desc, ...)` 为 legacy/prototype 入口。
- [ ] 标记 `WidgetProps::build()` 为 legacy desc 专用。
- [ ] 标记 `build_workspace_tree` 为 legacy。
- [ ] 标记 `node_card_from_render_view` 为 legacy。
- [ ] 标记 full-root `build_display_list` 为 legacy。
- [ ] 文档列出每个 legacy API 的最后调用点和删除条件。

验收：

- [ ] 生产 retained path 不直接调用上述 API。
- [ ] legacy API 有编译期 feature、模块路径或测试边界约束。

### 2. Compiled Template System

- [ ] 新增 `compiled.rs`。
- [ ] 新增 `slots.rs`。
- [ ] 新增 `revision.rs`。
- [ ] 将 Phase 1 手写 template 转成 `CompiledTemplate` 结构。
- [ ] `TextBoxTemplate`、`CanvasNodeCardTemplate`、`PanelFrameTemplate` 使用同一实例化入口。
- [ ] 模板 revision 变更能让 dev/test 检测不兼容 instance。
- [ ] static/generated template 不携带 runtime state。

验收：

- [ ] 多实例共享同一个 `CompiledTemplate`。
- [ ] 新增节点只创建新 `TemplateInstance`。
- [ ] 修改文本只 patch runtime state，不重新生成 template。

### 3. Build-Time Generation

- [ ] 审查现有 `build.rs` / generated panel 逻辑。
- [ ] 决定统一入口：现有生成系统并入 `TemplateRegistry`，或迁移删除。
- [ ] codegen 输出 `CompiledTemplate` 或等价静态结构。
- [ ] 生成代码路径固定，不能写入随机业务目录。
- [ ] 生成结果纳入 test/build 验证。

验收：

- [ ] build-time template 和手写 template 使用同一 API。
- [ ] dev/user 启动脚本不需要知道两套 UI build 系统。
- [ ] generated template 变更可通过 revision/debug 信息定位。

### 4. Production Legacy Removal

- [ ] 迁移所有 production desc builder 调用。
- [ ] 删除或 feature-gate `legacy_desc` production exports。
- [ ] 删除 canvas render view builder 生产依赖。
- [ ] 删除 workspace full tree builder 生产依赖。
- [ ] 删除全树 stable id scan 生产依赖。
- [ ] 删除全树 control intrinsic scan 生产依赖。
- [ ] 删除 full-root paint generation 生产依赖。
- [ ] 删除 per-frame renderer resource recreate 生产依赖。

验收：

- [ ] grep gate 中 production path 无 legacy 调用。
- [ ] 没有无删除条件的 adapter。
- [ ] `mod.rs` public exports 只暴露 retained 标准 API。

### 5. Architecture Documentation

- [ ] 新增或更新 `docs/architecture/retained-ui-render-path.md`。
- [ ] 说明 app/workspace、template、tree、layout、paint、renderer 的边界。
- [ ] 说明 compile-time template 和 runtime instance 的区别。
- [ ] 说明 dirty flags 到 layout/paint/render 的数据流。
- [ ] 说明 TextArea 宽度/换行/高度的标准模型。
- [ ] 说明 panel 和 node card 共享 resize/container 能力。
- [ ] 旧 full rebuild 文档标记 legacy 或删除。

验收：

- [ ] 新工程师能从文档判断改动应该落在哪个模块。
- [ ] 文档不再把 full rebuild 描述为主路径。

### 6. Gates And CI

- [ ] 新增 retained UI grep gate 脚本或 xtask。
- [ ] gate 检查 `legacy_desc`、`WidgetProps::build`、`reconcile`、`build_display_list` 等生产调用。
- [ ] gate 检查禁止 backend 类型泄漏到 tree/widget。
- [ ] gate 检查 debug-only logging 不在 production hot path。
- [ ] perf smoke 输出关键 counters。

验收：

- [ ] CI 或本地 test 可以防止旧路径回流。
- [ ] 性能 counters 回归时有明确失败信号。

## Cleanup Gates

```text
rg "legacy_desc" gui/src app/src
rg "WidgetProps::build" gui/src app/src
rg "Context::update" gui/src app/src
rg "build_workspace_tree" app/src gui/src
rg "node_card_from_render_view" gui/src app/src
rg "build_display_list" gui/src app/src
rg "control_intrinsics" gui/src app/src
rg "create_buffer" gui/src/renderer
```

分类结果必须写入 Phase 5 checklist：

```text
deleted
migrated
test-only
legacy feature only
blocked with owner and removal date
```

## Test Plan

- [ ] `compiled_template_instantiates_stable_node_structure`
- [ ] `compiled_template_does_not_store_runtime_text_state`
- [ ] `template_instance_patch_text_does_not_reinstantiate`
- [ ] `template_revision_detects_incompatible_dev_change`
- [ ] `registry_is_only_production_instantiation_entry`
- [ ] `legacy_desc_not_imported_by_production_modules`
- [ ] `widget_build_not_called_in_retained_frame`
- [ ] `unchanged_frame_has_zero_tree_layout_paint_rebuilds`
- [ ] `workspace_add_remove_node_uses_template_instance`
- [ ] `dev_hot_reload_emits_template_or_mutation_not_renderer_path`

## Performance Acceptance

- [ ] unchanged frame: `widget_build_calls == 0`。
- [ ] unchanged frame: `layout_nodes_visited == 0`。
- [ ] unchanged frame: `fragments_rebuilt == 0`。
- [ ] unchanged frame: renderer uploads are zero or bounded small constant。
- [ ] node drag: no internal layout, no node internal paint rebuild, no geometry retessellation。
- [ ] TextArea same-height input: no parent layout。
- [ ] canvas pan/zoom: no node template instantiate, no node layout, no grid primitive expansion。

## Final Demo Scenario

- [ ] 打开包含大量节点的 workspace。
- [ ] 编辑 TextArea，观察 wrap、高度和 dirty counters。
- [ ] resize node card，确认 panel/node card 使用同一容器 resize 能力。
- [ ] drag node，确认不 rebuild internals。
- [ ] pan/zoom canvas，确认 grid/renderer counters 稳定。
- [ ] add/remove node，确认 mount/unmount instance 而非全树 build。
- [ ] 输出最终 frame stats，作为迁移验收记录。

## Assumptions

- Phase 5 可以保留测试和 prototype 的 legacy path，但生产路径不能依赖它。
- 编译期模板化不代表运行时无状态；运行时 instance 仍然是交互、布局、绘制和 renderer cache 的真实载体。
- 所有 Phase 1-4 中临时增加的 adapter，Phase 5 必须删除或转正，否则视为未完成。
