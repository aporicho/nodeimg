# Phase 2: Local Layout And Text System

Date: 2026-04-28

## Goal

在 Phase 1 retained tree 和 dirty flags 之上，把布局从全树递归迁移为 relayout boundary 级别更新，并修正文本控件的基础语义：

```text
父容器宽度变化 -> 子文本可用宽度变化
文本可用宽度变化 -> 重新换行
换行行数变化 -> 文本控件高度变化
控件高度变化 -> 最近 relayout boundary 局部重排
```

Phase 2 的重点是正确的 layout/text 数据流，不解决 paint fragment 或 renderer backend batching。

## File Structure

新增和调整：

```text
gui/src/tree/layout/
  constraints.rs
  cache.rs
  dirty.rs
  boundary.rs
  intrinsic.rs

gui/src/text/
  layout.rs
  cache.rs
  line_break.rs
  metrics.rs

gui/src/canvas/
  endpoint_cache.rs

gui/src/tree/
  hit_order.rs
```

规则：

- 不新增平行的 `gui/src/layout/` 根模块。
- `tree/layout` 负责容器布局、constraints、relayout boundary、layout result cache。
- `text` 负责字形测量、换行、行盒、文本 intrinsic size。
- canvas endpoint 和 hit order 是 layout 的消费者，不反向修改 layout。

## Standard APIs

### Layout Constraints

```rust
pub enum SizeMode {
    Fixed(f32),
    Fill,
    Hug,
}

pub struct LayoutConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

pub struct LayoutInput {
    pub node: NodeId,
    pub constraints: LayoutConstraints,
    pub available_content_width: f32,
    pub style_revision: Revision,
    pub text_revision: Option<Revision>,
    pub children_revision: Revision,
}

pub struct LayoutOutput {
    pub rect: Rect,
    pub content_rect: Rect,
    pub intrinsic_width: f32,
    pub intrinsic_height: f32,
    pub baseline: Option<f32>,
}
```

统一语义：

- `Fixed`: 尺寸由节点属性或外部 resize 明确指定。
- `Fill`: 尺寸跟随父容器可用空间。
- `Hug`: 尺寸由内容 intrinsic size 决定。
- `TextArea` 宽度默认 `Fill`，高度默认 `Hug`，但受 `min_rows` 限制。

### Relayout Boundary

```rust
pub struct RelayoutBoundary {
    pub root: NodeId,
    pub reason: RelayoutBoundaryReason,
}

pub enum RelayoutBoundaryReason {
    Root,
    Panel,
    CanvasRoot,
    CanvasNodeCard,
    FixedConstraintTextField,
    Explicit,
}
```

Tree API：

```rust
Tree::nearest_relayout_boundary(node: NodeId) -> NodeId
Tree::mark_layout_dirty(node: NodeId, reason: LayoutDirtyReason)
Tree::take_layout_dirty() -> LayoutDirtyQueues
Context::flush_layout_dirty(root_rect, measurer, theme) -> LayoutFlushStats
```

传播规则：

```text
position x/y change      -> COMPOSITE + HIT, no layout
size change              -> LAYOUT + HIT + PAINT
text content change      -> TEXT_LAYOUT + PAINT
text wrapped height same -> no parent layout
text wrapped height diff -> LAYOUT bubbles to nearest relayout boundary
viewport pan             -> COMPOSITE, no layout
viewport size change     -> dirty boundaries whose constraints changed
```

### Text Layout API

```rust
pub struct TextLayoutRequest {
    pub text: TextRevision,
    pub style: TextStyleRevision,
    pub max_width: f32,
    pub min_rows: u16,
    pub padding: Insets,
}

pub struct TextLayoutResult {
    pub lines: Vec<TextLine>,
    pub wrapped_line_count: usize,
    pub content_width: f32,
    pub content_height: f32,
    pub field_height: f32,
    pub cursor_positions: CursorPositionMap,
}

pub struct TextLayoutCacheKey {
    pub text_revision: Revision,
    pub style_revision: Revision,
    pub max_width_bucket: LayoutScalar,
}
```

TextArea 高度公式：

```text
line_count = max(min_rows, wrapped_line_count)
field_height = padding.top + padding.bottom + line_count * line_height
```

文本换行要求：

- 必须按 char/grapheme 边界处理，禁止 byte index 切字符串。
- 长英文/无空格字符串必须在可用宽度内软换行。
- 光标、选区变化只更新 cursor/selection paint，不重新 wrap。
- content、font、line height、padding、available width 改变才重新 wrap。

## Migration Checklist

### 1. Layout Constraints 标准化

- [ ] 新增 `constraints.rs` 并定义 `SizeMode`、`LayoutConstraints`。
- [ ] 梳理现有 fixed/fill/hug/auto 字段，统一映射到 `SizeMode`。
- [ ] node card 的可 resize 宽高进入 `Fixed` 或外部 constraint，而不是临时 style patch。
- [ ] panel 和 node card 使用同一套 resize 后的 layout invalidation 规则。
- [ ] TextArea 宽度从父 content width 传入，不从自身文字长度反推。

验收：

- [ ] 父容器变宽后，TextArea 一行能容纳更多文字。
- [ ] 父容器变窄后，TextArea 重新 wrap 并按行数增高。
- [ ] node card 和 panel resize 行为使用同一组 layout dirty API。

### 2. Relayout Boundary

- [ ] 新增 `boundary.rs`。
- [ ] root、panel、canvas root、canvas node card、fixed-constraint text field 标记 boundary。
- [ ] `LAYOUT` dirty 冒泡到最近 boundary。
- [ ] `TEXT_LAYOUT` 先在文本节点内部消费，只有 output size 变化才追加 `LAYOUT`。
- [ ] layout flush 只访问 dirty boundary subtree。
- [ ] full layout 函数保留为 legacy/test fallback。

验收：

- [ ] node drag 不访问 node card 内部 layout。
- [ ] TextArea 输入但高度不变时不 layout parent。
- [ ] TextArea 高度变化时只 relayout 当前 node card boundary。

### 3. Layout Cache

- [ ] 新增 `cache.rs`。
- [ ] cache key 包含 node revision、style revision、children revision、constraint key。
- [ ] cache value 包含 rect、content rect、intrinsic size、baseline。
- [ ] size/children/style revision 不变时复用 layout output。
- [ ] transform/position change 不清 layout cache。
- [ ] cache invalidation 由 dirty flags 驱动，不能靠每帧扫描。

验收：

- [ ] pan/zoom 不清理 layout cache。
- [ ] node position change 不重新计算子控件 layout。
- [ ] resize 只使目标 boundary 的相关 cache 失效。

### 4. Text Layout Cache

- [ ] 新增或重构 `gui/src/text/layout.rs`。
- [ ] 新增 `TextLayoutCache`。
- [ ] 删除生产路径中 O(n^2) substring measure。
- [ ] 所有字符串切分使用 char/grapheme 边界。
- [ ] `TextArea` 使用父容器 content width 作为 max width。
- [ ] `min_rows` 固定 5 行时，初始高度为 5 行，不额外多一行。
- [ ] wrapped line count 未变化时不触发父 layout。
- [ ] 光标/选区移动不触发 wrap。

验收：

- [ ] 长连续英文输入自动换行。
- [ ] 宽度变宽后 wrapped line count 减少，高度同步降低。
- [ ] 宽度变窄后 wrapped line count 增加，高度同步增加。
- [ ] 不再出现 byte index panic。

### 5. Control Intrinsic Migration

- [ ] 现有 `control_intrinsics` 扫描路径标记 legacy。
- [ ] `TextBoxRegistry` 或 text runtime 维护 dirty intrinsic set。
- [ ] intrinsic size 由控件输出，不由父节点每帧扫描猜测。
- [ ] 控件 intrinsic change 只在值变化时向父 boundary 报告。
- [ ] 删除临时日志，保留 stats counters。

验收：

- [ ] 输入同一行内文字不反复触发父 intrinsic scan。
- [ ] 多 TextArea 场景只重新测量 dirty 控件。

### 6. Canvas Endpoint And Hit Support Caches

- [ ] 新增 `canvas/endpoint_cache.rs`。
- [ ] node rect、port rect、canvas transform 改变时更新 affected endpoints。
- [ ] connection path 使用 endpoint cache，不扫描所有节点布局。
- [ ] 新增 `tree/hit_order.rs` 或同等模块。
- [ ] hit order 根据 child order/z revision 更新。
- [ ] 复杂形状 hit 继续使用 shape/bounds 信息，不依赖 paint list。

验收：

- [ ] 拖动一个节点只更新相关 connection endpoints。
- [ ] z-index 变化更新 hit order cache。
- [ ] 大圆角容器 hit 和视觉区域一致。

## Cleanup Gates

```text
rg "substring" gui/src/text gui/src/widget
rg "byte index" gui/src
rg "control_intrinsics" gui/src app/src
rg "sync_with_tree" gui/src
rg "layout(" gui/src/tree gui/src/widget gui/src/canvas
```

禁止留下：

- [ ] TextArea 宽度由文本内容撑开。
- [ ] TextArea 高度永远多一行。
- [ ] 每帧全树 intrinsic scan。
- [ ] production text wrap 使用 byte index。
- [ ] node card resize 和 panel resize 两套不同 invalidation 逻辑。

## Test Plan

- [ ] `text_area_wraps_long_unbroken_text_to_parent_width`
- [ ] `text_area_uses_parent_width_after_node_resize`
- [ ] `text_area_height_is_min_rows_without_extra_line`
- [ ] `text_area_height_grows_only_when_line_count_grows`
- [ ] `text_area_height_shrinks_when_parent_width_expands`
- [ ] `cursor_move_does_not_rewrap_text`
- [ ] `text_insert_same_line_does_not_layout_parent`
- [ ] `text_insert_new_wrapped_line_layouts_nearest_boundary`
- [ ] `node_drag_does_not_relayout_node_children`
- [ ] `node_resize_relayouts_node_boundary_only`
- [ ] `panel_and_node_card_share_resize_invalidation_path`
- [ ] `endpoint_cache_updates_only_edges_connected_to_moved_node`
- [ ] `hit_order_cache_updates_on_z_index_change`

## Performance Acceptance

- [ ] 100 TextArea same-line input: only 1 text layout request for active field。
- [ ] 100 nodes drag: `layout_nodes_visited == 0` for node internals。
- [ ] node resize: layout visits bounded by target node card subtree。
- [ ] canvas pan: no layout cache invalidation。
- [ ] long text wrap: measure calls grow approximately linear with text length。

## Assumptions

- Phase 2 可以保留 full layout fallback，但 retained production path 必须优先走 dirty boundary flush。
- Spatial index 如果 hit test 仍然是主要瓶颈，可以作为 Phase 2 末尾扩展；不是 Phase 2 的第一优先级。
- Text shaping 继续复用现有 renderer/text backend 能力，不在 Phase 2 引入新字体渲染 backend。
