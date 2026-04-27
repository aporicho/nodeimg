# Phase 1: Retained Tree Foundation

Date: 2026-04-28

## Goal

建立 retained UI 的最小生产骨架，把当前“每帧重新生成完整 `Desc`、reconcile、同步所有 widget props”的路径，迁移成：

```text
TemplateRegistry 注册静态结构
TreeMutation 创建/删除/修改实例
TreeIndex O(1) 查询 live nodes
DirtyFlags 描述后续 layout/paint/hit/render 工作
FrameStats 量化每帧成本
legacy_desc 隔离未迁移旧路径
```

Phase 1 不解决所有 layout/paint/renderer 性能问题；它只确保后续阶段有稳定的 retained tree 和 dirty API。

## File Structure

新增和调整：

```text
gui/src/template/
  mod.rs
  id.rs
  registry.rs
  template.rs
  instantiate.rs

gui/src/tree/
  index.rs
  mutation.rs
  dirty.rs
  frame_stats.rs
  legacy_desc.rs

gui/src/tree/layout/
  dirty.rs
  cache.rs

gui/src/widget/state/
  text_box_registry.rs

gui/src/canvas/
  scene_model.rs
  scene_diff.rs
  template.rs
  legacy_desc.rs

app/src/workspace/
  scene_controller.rs
```

规则：

- 不新增 `gui/src/layout/`，避免和 `gui/src/tree/layout/` 形成双轨。
- `tree/diff.rs` 可以保留，但生产入口移动到 `tree/legacy_desc.rs`。
- `canvas/node_card.rs` 中旧 Desc 构建路径最终移动到 `canvas/legacy_desc.rs`。
- production retained path 禁止从 `legacy_desc` 调用。

## Standard APIs

### Template API

模板只负责创建稳定结构，不负责运行时 patch。

```rust
pub trait UiTemplate {
    fn id(&self) -> TemplateId;

    fn instantiate(
        &self,
        cx: &mut InstantiateCx,
        instance_id: InstanceId,
        parent: NodeId,
    ) -> NodeId;
}
```

第一批模板：

```text
TextBoxTemplate
CanvasRootTemplate
CanvasGridTemplate
CanvasNodeCardTemplate
ParamControlTemplate
PanelFrameTemplate
```

stable id 规则：

```text
{instance_id}
{instance_id}::field
{instance_id}::value
{instance_id}::caret
{instance_id}::selection
```

### Tree Index API

```rust
pub struct TreeIndex {
    stable_to_node: HashMap<StableId, NodeId>,
}
```

Tree 对外提供：

```rust
Tree::node_by_str(&self, stable_id: &str) -> Option<NodeId>
Tree::contains_stable_id(&self, stable_id: &str) -> bool
Tree::validate_index(&self) -> Result<(), TreeIndexError>
```

规则：

- index 只记录 live nodes。
- retained runtime 继续留在 runtime/state registry，不混入 live index。
- insert/remove/reparent 必须同步 index。
- duplicate stable id 是错误，debug/test 下必须暴露。

### Tree Mutation API

所有结构和属性修改必须走 mutation，不允许业务代码直接改 `TreeNode` 字段。

```rust
pub enum TreeMutation {
    MountTemplate {
        parent: NodeId,
        template: TemplateId,
        instance: InstanceId,
    },
    Unmount {
        node: NodeId,
    },
    SetRect {
        node: NodeId,
        rect: Rect,
    },
    SetStyle {
        node: NodeId,
        patch: StylePatch,
    },
    SetText {
        node: NodeId,
        value: String,
    },
    SetVisible {
        node: NodeId,
        visible: bool,
    },
    SetZIndex {
        node: NodeId,
        z_index: i32,
    },
}
```

统一入口：

```rust
Tree::apply_mutation(&mut self, mutation: TreeMutation) -> Invalidation
Tree::apply_mutations(&mut self, mutations: impl IntoIterator<Item = TreeMutation>)
```

### Dirty API

固定 dirty flags：

```text
STRUCTURE
STYLE
LAYOUT
TEXT_LAYOUT
PAINT
HIT
PAINT_ORDER
COMPOSITE
```

标准规则：

```text
SetText        -> TEXT_LAYOUT + PAINT
SetStyle color -> PAINT
SetRect size   -> LAYOUT + HIT + PAINT
SetRect x/y    -> COMPOSITE + HIT
SetVisible     -> PAINT + HIT
SetZIndex      -> PAINT_ORDER + HIT + PAINT
Mount/Unmount  -> STRUCTURE + LAYOUT + HIT + PAINT
```

Tree API：

```rust
Tree::mark_dirty(node, flags)
Tree::take_dirty() -> DirtyQueues
Tree::clear_dirty()
```

传播规则：

```text
LAYOUT 向上冒泡到最近 relayout boundary
PAINT 向上冒泡到最近 repaint boundary
COMPOSITE 不触发布局
```

### FrameStats API

```rust
pub struct FrameStats {
    widget_build_calls: usize,
    tree_nodes: usize,
    stable_id_lookups: usize,
    full_tree_scans: usize,
    reconcile_child_matches: usize,
    layout_nodes_visited: usize,
    text_measure_calls: usize,
    paint_nodes_visited: usize,
    paint_commands: usize,
    backend_commands: usize,
    text_batches: usize,
    render_passes: usize,
}
```

要求：

- 默认低开销。
- 测试可开启 deterministic stats。
- debug 日志可以按场景输出。
- stats 逻辑不能散落在业务层。

## Migration Checklist

### 1. 文档和统计

- [ ] 实现 `gui/src/tree/frame_stats.rs`。
- [ ] 接入 `Context::update`。
- [ ] 接入 `reconcile`。
- [ ] 接入 `layout`。
- [ ] 接入 `TextMeasurer`。
- [ ] 接入 `build_display_list`。
- [ ] 接入 `renderer::dispatch`。
- [ ] mouse move 输出每帧成本。
- [ ] text input 输出每帧成本。
- [ ] node drag 输出每帧成本。
- [ ] canvas pan 输出每帧成本。

验收：

- [ ] 不改变现有行为。
- [ ] 能量化每帧 build/layout/text/paint/render 成本。

### 2. TreeIndex

- [ ] 实现 `gui/src/tree/index.rs`。
- [ ] `Tree` insert 时注册 stable id。
- [ ] `Tree` remove 时注销 stable id。
- [ ] NodeId 复用前清理旧 index。
- [ ] `Tree::node_by_str` 改为 index lookup。
- [ ] runtime slot lookup 走 index。
- [ ] duplicate stable id 在 debug/test 下暴露。
- [ ] 删除 `node_by_str` 的全树扫描实现。
- [ ] 搜索所有 stable id 全树扫描调用并迁移到 index lookup。

验收：

- [ ] `Tree::validate_index()` 通过。
- [ ] `Tree::iter()` 不再用于 stable id 查询。

### 3. Reconcile O(n) 化并隔离 legacy

- [ ] `reconcile_children` old children 建 `HashMap<StableId, NodeId>`。
- [ ] new children 按 stable id O(1) 匹配。
- [ ] duplicate child id 直接报错或 debug assert。
- [ ] 新增 `gui/src/tree/legacy_desc.rs`。
- [ ] `Context::update(desc, ...)` 内部转到 legacy 模块。
- [ ] `tree/diff.rs` 不再作为推荐入口导出。
- [ ] retained 新代码禁止调用 `reconcile`。

验收：

- [ ] siblings 数量增加时 child matching 不再平方增长。
- [ ] 原有测试通过。

### 4. TreeMutation + DirtyFlags

- [ ] 实现 `gui/src/tree/mutation.rs`。
- [ ] 实现 `gui/src/tree/dirty.rs`。
- [ ] 接入 `TreeMutation::MountTemplate`。
- [ ] 接入 `TreeMutation::Unmount`。
- [ ] 接入 `TreeMutation::SetRect`。
- [ ] 接入 `TreeMutation::SetStyle`。
- [ ] 接入 `TreeMutation::SetText`。
- [ ] 接入 `TreeMutation::SetVisible`。
- [ ] 接入 `TreeMutation::SetZIndex`。
- [ ] canvas 新代码禁止直接修改 `TreeNode` 字段。
- [ ] panel 新代码禁止直接修改 `TreeNode` 字段。
- [ ] textbox 新代码禁止直接修改 `TreeNode` 字段。

验收：

- [ ] 每类 mutation 对应 dirty flags 有单测。
- [ ] dirty queue 可被 `Context` 消费。
- [ ] hover 不产生 layout dirty。

### 5. TemplateRegistry Skeleton

- [ ] 实现 `gui/src/template/id.rs`。
- [ ] 实现 `gui/src/template/registry.rs`。
- [ ] 实现 `gui/src/template/template.rs`。
- [ ] 实现 `gui/src/template/instantiate.rs`。
- [ ] 实现 `TextBoxTemplate`。
- [ ] 实现 `CanvasNodeCardTemplate`。
- [ ] 实现 `CanvasGridTemplate`。
- [ ] 模板创建稳定结构。
- [ ] 实例属性通过 `TreeMutation` 修改。
- [ ] 新模板不调用 `WidgetProps::build()`。
- [ ] `WidgetProps::build()` 只属于 legacy desc 路径。

验收：

- [ ] `TextBoxTemplate` mount 后 stable ids 完整。
- [ ] 多个 TextBox 实例共享同一模板定义。
- [ ] mount 后更新文本不重新 instantiate。

### 6. TextBox / TextArea retained path

- [ ] 新增 `gui/src/widget/state/text_box_registry.rs`。
- [ ] 记录 text box instance id -> runtime。
- [ ] 只同步 dirty text box。
- [ ] 替代 `sync_with_tree` 全树扫描所有 text box 的生产路径。
- [ ] text input event 直接修改 `TextBoxRuntime`。
- [ ] 文本变化触发 `TreeMutation::SetText`。
- [ ] 高度不变时只标记 `TEXT_LAYOUT + PAINT`。
- [ ] 高度变化时追加 `LAYOUT`。
- [ ] TextArea 不再通过每帧 props value 同步文本。
- [ ] 临时 sizing debug 日志删除或降级为 stats。
- [ ] `TextBoxStore::sync_with_tree` 全树扫描路径移到 legacy。

验收：

- [ ] 输入文字不调用全量 widget build。
- [ ] 输入一行内变化不触发父节点 layout。
- [ ] 高度变化只 dirty 最近相关父级。
- [ ] 多行 wrap 不走 O(n^2) substring measure 的路径列入 Phase 2 必修项。

### 7. CanvasSceneModel

- [ ] 新增 `gui/src/canvas/scene_model.rs`。
- [ ] 新增 `gui/src/canvas/scene_diff.rs`。
- [ ] 新增 `gui/src/canvas/template.rs`。
- [ ] 新增 `app/src/workspace/scene_controller.rs`。
- [ ] 保存当前 canvas node instances。
- [ ] 保存当前 connection instances。
- [ ] engine graph version 变化时 diff。
- [ ] add node -> `MountTemplate(CanvasNodeCardTemplate)`。
- [ ] remove node -> `Unmount(node instance)`。
- [ ] drag node -> `SetRect` x/y only -> `COMPOSITE + HIT`。
- [ ] resize node -> `SetRect` size -> `LAYOUT + HIT + PAINT`。
- [ ] select node -> visual/style mutation -> `PAINT`。
- [ ] text changed -> `SetText` -> `TEXT_LAYOUT + PAINT`。
- [ ] `WorkspaceController::canvas_node_render_views` 标记 legacy。
- [ ] `node_card_from_render_view` 移到 `canvas/legacy_desc.rs`。
- [ ] `build_workspace_tree` 不再负责生成所有 node desc。

验收：

- [ ] mouse move 不生成所有 node desc。
- [ ] drag node 不 rebuild node internals。
- [ ] resize node 只 dirty 目标 node。
- [ ] add/remove node 只影响对应 instance。

## Cleanup Gates

```text
rg "node_by_str" gui/src app/src
rg "reconcile\\(" gui/src app/src
rg "build_user_desc" app/src
rg "node_card_from_render_view" gui/src app/src
rg "WidgetProps::build" gui/src app/src
```

禁止留下：

- [ ] 新的全树 stable id 扫描。
- [ ] 新的每帧 widget build。
- [ ] 新的业务代码直接改 `TreeNode`。
- [ ] 新的 Desc 主路径。
- [ ] 没有删除条件的 adapter。

## Test Plan

- [ ] `tree_index_registers_and_removes_live_nodes`
- [ ] `tree_index_rejects_duplicate_stable_ids`
- [ ] `runtime_slot_lookup_uses_tree_index`
- [ ] `reconcile_children_matches_by_key_in_linear_time`
- [ ] `tree_mutation_set_text_marks_text_layout_and_paint`
- [ ] `tree_mutation_set_rect_position_marks_composite_not_layout`
- [ ] `tree_mutation_set_rect_size_marks_layout_hit_paint`
- [ ] `text_area_input_does_not_call_widget_build`
- [ ] `text_area_same_height_input_does_not_layout_parent`
- [ ] `canvas_node_drag_does_not_layout_node_internals`
- [ ] `canvas_node_resize_marks_only_target_subtree`
- [ ] `canvas_scene_add_node_mounts_one_instance`
- [ ] `canvas_scene_remove_node_unmounts_one_instance`

## Performance Acceptance

- [ ] 100 nodes mouse move: `widget_build_calls == 0` for retained path。
- [ ] 100 nodes mouse move: `layout_nodes_visited == 0` unless cursor state requires layout。
- [ ] node drag: no internal node layout。
- [ ] TextArea input same height: no full-tree layout。
- [ ] canvas pan: no node template instantiate。
- [ ] canvas pan: no node internal layout。

## Assumptions

- Phase 1 的“编译期 build”定义为：结构模板注册一次，frame hot path 不 build 结构。
- `Desc` 暂时保留，但只能通过 `legacy_desc` 进入。
- Renderer batching、retained paint、grid shader 放到后续阶段；Phase 1 先让 UI 树更新路径不再全量重建。
