# Phase 3: Retained Paint

Date: 2026-04-28

## Goal

在 Phase 1 retained tree 和 Phase 2 local layout 正确之后，把 paint/display list 从“每帧全树生成”迁移为 repaint boundary + local paint fragment：

```text
visual dirty -> 找最近 repaint boundary
boundary subtree -> 重新记录 local paint fragment
unchanged boundary -> 复用 fragment
pan / drag / zoom -> 更新 composition transform
```

Phase 3 只解决 paint traversal 和 display command recording，不解决 GPU render pass、text batch 或 buffer upload，后者属于 Phase 4。

## File Structure

新增和调整：

```text
gui/src/tree/
  repaint.rs
  paint_cache.rs
  paint_order.rs

gui/src/paint/
  fragment.rs
  recorder.rs

gui/src/canvas/
  connection_paint_cache.rs
```

规则：

- repaint boundary 查询属于 `tree`。
- paint fragment payload 属于 `paint`。
- renderer 不反向访问 widget/tree 内部结构，只消费 retained scene 或 fragment commands。
- hit test 不依赖 paint fragment replay，继续使用 tree/hit shape cache。

## Standard APIs

### Repaint Boundary

```rust
pub struct RepaintBoundaryId(NodeId);

pub enum RepaintBoundaryReason {
    Root,
    PanelFrame,
    CanvasRoot,
    CanvasGridLayer,
    CanvasConnectionLayer,
    CanvasNodeCard,
    ActiveTextEditor,
    Overlay,
    Explicit,
}
```

Tree API：

```rust
Tree::is_repaint_boundary(node: NodeId) -> bool
Tree::nearest_repaint_boundary(node: NodeId) -> RepaintBoundaryId
Tree::mark_paint_dirty(node: NodeId, reason: PaintDirtyReason)
Tree::take_paint_dirty() -> PaintDirtyQueues
```

### Paint Fragment

```rust
pub struct PaintFragment {
    pub boundary: RepaintBoundaryId,
    pub revision: Revision,
    pub local_bounds: Rect,
    pub clips: Vec<ClipCommand>,
    pub commands: Vec<PaintCommand>,
}

pub struct PaintCache {
    fragments: HashMap<RepaintBoundaryId, PaintFragment>,
}
```

Paint API：

```rust
PaintCache::get(boundary) -> Option<&PaintFragment>
PaintCache::rebuild(boundary, tree, layout, theme, recorder) -> PaintFragment
PaintCache::evict_subtree(boundary)
Context::flush_paint_dirty(theme) -> PaintFlushStats
```

### Dirty Mapping

```text
STYLE visual change      -> PAINT nearest repaint boundary
TEXT_LAYOUT size same    -> PAINT text/editor boundary
TEXT_LAYOUT size changed -> LAYOUT first, then PAINT affected boundary
hover/focus/selected     -> PAINT affected visual boundary
position x/y change      -> COMPOSITE + HIT, no repaint
pan/zoom                 -> COMPOSITE, no repaint
z-index change           -> PAINT_ORDER + HIT + PAINT parent boundary
mount/unmount            -> evict affected subtree fragments
```

## Migration Checklist

### 1. Boundary Model

- [ ] 新增 `tree/repaint.rs`。
- [ ] root、panel、canvas root、grid layer、connection layer、node card、overlay 标记 repaint boundary。
- [ ] active text editor 的 caret/selection 独立为 boundary 或 overlay fragment。
- [ ] boundary 声明来自 template/style，不硬编码在 painter 内部。
- [ ] `PAINT` dirty 向最近 repaint boundary 冒泡。

验收：

- [ ] hover 一个节点不会 dirty 整个 canvas root。
- [ ] caret blink 不 repaint node card 背景。
- [ ] pan/zoom 不产生 repaint dirty。

### 2. Paint Fragment Recording

- [ ] 新增 `paint/fragment.rs`。
- [ ] 新增 `paint/recorder.rs` 或复用现有 display list recorder。
- [ ] fragment commands 使用 boundary-local 坐标。
- [ ] local clip、rounded clip、overflow clip 记录在 fragment 内。
- [ ] fragment composition 时应用外层 transform。
- [ ] 复用现有 display command schema，避免 Phase 3 新造 renderer command 双轨。

验收：

- [ ] 节点拖拽时 fragment local commands 不变。
- [ ] 大圆角容器 clip 和视觉结果一致。
- [ ] nested transform 下 fragment bounds 正确。

### 3. Paint Cache

- [ ] 新增 `tree/paint_cache.rs`。
- [ ] cache key 使用 boundary revision、style revision、text paint revision、children paint revision。
- [ ] dirty boundary rebuild 时不访问 sibling boundary。
- [ ] mount/unmount 时 evict affected subtree fragment。
- [ ] fragment reused/rebuilt 计入 `FrameStats`。
- [ ] cache 不持有 renderer backend resource，backend resource 属于 Phase 4。

验收：

- [ ] hover old/new node 只 rebuild 两个 affected fragments。
- [ ] 输入 TextArea 只 rebuild active text/editor fragment 和必要父视觉 fragment。
- [ ] unchanged frame `fragments_rebuilt == 0`。

### 4. Paint Order Cache

- [ ] 新增 `tree/paint_order.rs`。
- [ ] 按 child order、z-index、stacking context revision 缓存 paint order。
- [ ] z-index change 只 invalid parent boundary paint order。
- [ ] hit order cache 和 paint order cache 可以共享 revision，但 API 分离。

验收：

- [ ] z-index 变化后视觉顺序和 hit 顺序一致。
- [ ] 无 z-index 变化时 paint traversal 不重新排序 children。

### 5. Canvas Paint Layers

- [ ] grid layer 独立 repaint boundary。
- [ ] connection layer 独立 repaint boundary。
- [ ] node card 每个实例独立 repaint boundary。
- [ ] node movement 只触发 connection layer 相关 path 更新，不 repaint node card internal fragment。
- [ ] 后续可以把 connection layer 细化成 per-edge dirty subset，但 Phase 3 至少隔离 node card repaint。

验收：

- [ ] drag node 不 repaint node card internal controls。
- [ ] drag node 可以更新 related connections。
- [ ] canvas pan 复用 grid/connection/node fragments。

### 6. Legacy Paint Isolation

- [ ] root-level full `build_display_list` 标记 legacy。
- [ ] retained production path 调用 `flush_paint_dirty` + fragment composition。
- [ ] full paint fallback 只允许在 legacy/test/prototype 入口。
- [ ] 删除没有删除 gate 的 paint adapter。

验收：

- [ ] retained canvas/panel path 不调用 full-root paint generation。
- [ ] legacy paint imports 有明确模块边界。

## Cleanup Gates

```text
rg "build_display_list" gui/src app/src
rg "paint_nodes_visited" gui/src
rg "repaint" gui/src/tree gui/src/paint
rg "DisplayList" gui/src
rg "legacy" gui/src/paint gui/src/tree gui/src/canvas
```

禁止留下：

- [ ] hover/caret 触发 root repaint。
- [ ] pan/zoom 触发 paint fragment rebuild。
- [ ] fragment 使用 global 坐标导致 drag 必须重画。
- [ ] paint/hit 共享同一份 display command 作为唯一真相。

## Test Plan

- [ ] `nearest_repaint_boundary_returns_node_card_for_node_child`
- [ ] `hover_marks_only_old_and_new_boundaries`
- [ ] `caret_blink_repaints_editor_boundary_only`
- [ ] `node_drag_marks_composite_not_repaint_for_node_card`
- [ ] `canvas_pan_reuses_node_fragments`
- [ ] `fragment_records_local_coordinates`
- [ ] `rounded_clip_replays_correctly_after_transform`
- [ ] `z_index_change_invalidates_parent_paint_order`
- [ ] `mount_evicts_affected_subtree_fragments`
- [ ] `unmount_evicts_removed_fragments`

## Performance Acceptance

- [ ] unchanged frame: `fragments_rebuilt == 0`。
- [ ] hover node in 100-node canvas: rebuilt fragments <= affected old/new boundaries。
- [ ] drag node: node card internal paint nodes visited == 0。
- [ ] text caret blink: canvas root paint nodes visited == 0。
- [ ] pan/zoom: no paint command recording for node card fragments。

## Assumptions

- Phase 3 可以继续输出现有 display commands 作为 fragment payload。
- Renderer backend 仍可临时 flatten fragments，但必须统计 flattened/reused 差异。
- Per-edge connection dirty 可以后续扩展；Phase 3 的最低要求是 connection layer 与 node card repaint 隔离。
