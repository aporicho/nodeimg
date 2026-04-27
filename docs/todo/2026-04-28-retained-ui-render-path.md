# Retained UI Render Path: Five Phase Plan

Date: 2026-04-28

## Goal

把 GUI 渲染路径从“每帧重新生成完整 `Desc`、reconcile、全树 layout、全树 paint、全量 renderer prepare”，迁移到 retained UI 模型：

```text
编译/启动期：注册静态结构模板
运行期：创建模板实例
状态变化：只 patch 实例属性
渲染帧：只处理 dirty 边界和 dirty 资源
```

这份文件是总览索引。每个阶段的详细执行计划放在同目录的 phase 文档里。

## Phase Documents

- [Phase 1: Retained Tree Foundation](2026-04-28-retained-ui-render-path-phase-1-foundation.md)
- [Phase 2: Local Layout And Text System](2026-04-28-retained-ui-render-path-phase-2-local-layout-text.md)
- [Phase 3: Retained Paint](2026-04-28-retained-ui-render-path-phase-3-retained-paint.md)
- [Phase 4: Renderer Hot Path](2026-04-28-retained-ui-render-path-phase-4-renderer-hot-path.md)
- [Phase 5: Cleanup And Compile-Time Templates](2026-04-28-retained-ui-render-path-phase-5-cleanup-template-codegen.md)

## Phase Map

```text
Phase 1: retained instance tree + mutation + dirty flags + stats
Phase 2: relayout boundaries + text wrap/cache/auto-height + hit/endpoint support caches
Phase 3: repaint boundaries + local paint fragments + retained display composition
Phase 4: renderer backend caches + grid command + text batching + geometry/upload reuse
Phase 5: old path deletion + compiled templates + docs/tests/performance gates
```

## Non-Negotiable Architecture Rules

- 结构模板只创建一次；新增节点是 mount 同一个模板的新实例。
- 删除节点是 unmount 实例，不是重新 build 全树。
- 文本、位置、尺寸、hover、focus、selected 都是实例属性变化。
- layout、paint、hit、render prepare 由 dirty flags 精确驱动。
- `Desc`、`WidgetProps::build()`、全树扫描和全量 reconcile 只能留在明确的 legacy/test/prototype 边界。
- 任何临时 adapter 必须有 owner、迁移条件和删除条件。
- 新模块必须按树、模板、布局、绘制、renderer、业务 scene controller 分层，不能跨层拿内部结构走捷径。

## Global Module Boundaries

```text
app/src/workspace/
  owns semantic workspace model and emits scene mutations

gui/src/template/
  owns reusable static UI structure and template instantiation

gui/src/tree/
  owns retained live instance tree, indexes, mutations, dirty queues, stats

gui/src/tree/layout/
  owns constraints, relayout boundaries, layout caches

gui/src/text/
  owns text shaping/wrapping/layout caches

gui/src/paint/
  owns paint fragment payloads and paint command recording types

gui/src/renderer/
  owns backend preparation, batching, GPU caches and upload lifecycle
```

## Global Cleanup Gates

每个阶段完成时都要分类检查这些入口：

```text
rg "node_by_str" gui/src app/src
rg "reconcile\\(" gui/src app/src
rg "build_user_desc" app/src gui/src
rg "node_card_from_render_view" gui/src app/src
rg "WidgetProps::build" gui/src app/src
rg "legacy_desc" gui/src app/src
```

分类规则：

```text
production path: 必须迁移或删除
legacy path: 必须位于明确 legacy 模块，并且有删除 gate
tests/prototypes: 可以保留，但不能作为生产推荐入口导出
```

## Global Done Definition

- [ ] 未变化帧不发生 widget build、tree reconcile、layout rebuild、paint rebuild。
- [ ] TextArea 输入只影响文本 runtime、text layout dirty、必要时局部 layout dirty。
- [ ] node drag 不 rebuild 节点内部结构、不 relayout 节点内部控件、不 repaint 节点内部 fragment。
- [ ] canvas pan/zoom 不实例化节点、不重新布局节点、不展开 grid primitive。
- [ ] renderer prepare 成本跟 dirty resources 相关，而不是跟整棵 scene 大小线性相关。
- [ ] 生产路径没有无删除条件的 `legacy_desc`、adapter、全树扫描、全量 build。
- [ ] 架构文档、todo checklist、测试和性能 counters 全部对齐当前实现。

## Assumptions

- “编译期 build”定义为：静态结构模板可以在编译/启动期生成和注册，但运行时实例仍然持有状态、布局、绘制和 renderer cache。
- 真正的 `build.rs`、proc macro 或 generated template 属于 Phase 5；Phase 1 先建立 runtime API，使后续编译化不产生第二套系统。
- 五个阶段必须按顺序推进；后续阶段不能用 renderer cache 掩盖 retained tree、layout、paint 的错误。
