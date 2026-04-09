# Widget Props Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add typed Props system to the GUI framework, enabling interactive widgets (Slider, Toggle, etc.) with immutable configuration stored on tree nodes.

**Architecture:** Desc gains a Widget variant carrying `Box<dyn WidgetProps>`. PanelNode lifts style/decoration to node level; NodeKind becomes a simple discriminator. TextMeasurer extracted from Renderer so build()/reconcile stay pure. A new resolve phase handles text measurement before layout.

**Tech Stack:** Rust, glyphon (text shaping via cosmic-text), wgpu (GPU rendering)

**Spec:** `docs/superpowers/specs/2026-04-09-widget-props-design.md`

---

### Task 1: Extract TextMeasurer from TextPipeline

**Files:**
- Create: `gui/src/renderer/text_measurer.rs`
- Modify: `gui/src/renderer/pipeline/text.rs`
- Modify: `gui/src/renderer/renderer.rs`
- Modify: `gui/src/renderer/dispatch.rs`
- Modify: `gui/src/renderer/mod.rs`

- [ ] **Step 1: Create `text_measurer.rs` with FontSystem + buffer_cache + measure logic**

Extract the text measurement concern from `TextPipeline`. Move `FontSystem`, `buffer_cache`, `TextCacheKey`, `CachedBuffer`, and the `measure()` method into a standalone struct.

```rust
// gui/src/renderer/text_measurer.rs
use std::collections::HashMap;
use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};

#[derive(Hash, PartialEq, Eq, Clone)]
struct TextCacheKey {
    text: String,
    size_bits: u32,
}

impl TextCacheKey {
    fn new(text: &str, size: f32) -> Self {
        Self { text: text.to_string(), size_bits: size.to_bits() }
    }
}

struct CachedBuffer {
    buffer: Buffer,
    used: bool,
}

pub struct TextMeasurer {
    font_system: FontSystem,
    buffer_cache: HashMap<TextCacheKey, CachedBuffer>,
}

impl TextMeasurer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            buffer_cache: HashMap::new(),
        }
    }

    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn measure(&mut self, text: &str, size: f32) -> (f32, f32) {
        let key = TextCacheKey::new(text, size);
        if !self.buffer_cache.contains_key(&key) {
            let mut buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(size, size * 1.2),
            );
            buffer.set_size(&mut self.font_system, Some(f32::MAX), Some(size * 2.0));
            buffer.set_text(
                &mut self.font_system,
                text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffer_cache.insert(key.clone(), CachedBuffer { buffer, used: true });
        }
        let buffer = &self.buffer_cache[&key].buffer;
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height += run.line_height;
        }
        (width, height)
    }

    /// Mark all cache entries as unused. Called at start of frame.
    pub fn mark_unused(&mut self) {
        for entry in self.buffer_cache.values_mut() {
            entry.used = false;
        }
    }

    /// Get or create a cached buffer, marking it as used. Used by TextPipeline during prepare.
    pub fn get_or_create_buffer(&mut self, text: &str, size: f32) -> &Buffer {
        let key = TextCacheKey::new(text, size);
        if !self.buffer_cache.contains_key(&key) {
            let mut buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(size, size * 1.2),
            );
            buffer.set_size(&mut self.font_system, Some(f32::MAX), Some(size * 2.0));
            buffer.set_text(
                &mut self.font_system,
                text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffer_cache.insert(key.clone(), CachedBuffer { buffer, used: true });
        }
        let entry = self.buffer_cache.get_mut(&key).unwrap();
        entry.used = true;
        &entry.buffer
    }

    /// Remove unused cache entries.
    pub fn evict_unused(&mut self) {
        self.buffer_cache.retain(|_, v| v.used);
    }
}
```

- [ ] **Step 2: Refactor `TextPipeline` to use `TextMeasurer`**

Remove `font_system` and `buffer_cache` from `TextPipeline`. The pipeline receives `&mut TextMeasurer` in `prepare()` instead of owning these resources. Remove the `measure()` method from `TextPipeline`.

Key changes to `gui/src/renderer/pipeline/text.rs`:
- Remove `font_system`, `buffer_cache`, `TextCacheKey`, `CachedBuffer` from this file
- `TextPipeline::new()` no longer creates `FontSystem` — receives `&mut FontSystem` for initial atlas/cache setup
- `TextPipeline::prepare()` takes `&mut TextMeasurer` parameter — uses two-phase pattern: first call `measurer.mark_unused()` + `measurer.get_or_create_buffer()` for each text request (populating all buffers via `&mut`), then `measurer.evict_unused()`, then iterate `buffer_cache` with `&self` to build `TextArea` refs. Also uses `measurer.font_system()` for `text_renderer.prepare()`
- Remove `TextPipeline::measure()`

Note on borrow checker: `prepare()` must populate ALL buffers first (via `&mut TextMeasurer`), then build TextArea references (via `&TextMeasurer.buffer_cache`). This is the same two-phase pattern the current code already uses — first create/update buffers, then iterate them.

- [ ] **Step 3: Update `Renderer` to own `TextMeasurer`**

`gui/src/renderer/renderer.rs`:
- Add `text_measurer: TextMeasurer` field to `Renderer`
- `Renderer::new()` creates `TextMeasurer::new()`, passes `text_measurer.font_system()` to `TextPipeline::new()`
- `Renderer::measure_text()` delegates to `self.text_measurer.measure()`
- `Renderer::end_frame()` passes `&mut self.text_measurer` to dispatch — Rust allows borrowing separate struct fields simultaneously, so `&mut self.text_pipeline` and `&mut self.text_measurer` can coexist as arguments to `dispatch()`

- [ ] **Step 4: Update `dispatch.rs` — pass TextMeasurer through**

`gui/src/renderer/dispatch.rs`: add `text_measurer: &mut TextMeasurer` parameter to `dispatch()`. Pass it to `text_pipeline.prepare()`. Update the call in `Renderer::end_frame()` to pass `&mut self.text_measurer`.

- [ ] **Step 5: Export `TextMeasurer` from renderer module**

`gui/src/renderer/mod.rs`: add `pub mod text_measurer;` and `pub use text_measurer::TextMeasurer;`

- [ ] **Step 6: Verify compilation**

Run: `cargo build -p gui`

- [ ] **Step 7: Commit**

```bash
git add gui/src/renderer/text_measurer.rs gui/src/renderer/pipeline/text.rs gui/src/renderer/renderer.rs gui/src/renderer/dispatch.rs gui/src/renderer/mod.rs
git commit -m "refactor: extract TextMeasurer from TextPipeline"
```

---

### Task 2: Refactor PanelNode — lift style/decoration to node level

**Files:**
- Modify: `gui/src/panel/tree/node.rs`
- Modify: `gui/src/panel/tree/desc.rs`
- Modify: `gui/src/panel/tree/diff.rs`
- Modify: `gui/src/panel/tree/paint.rs`
- Modify: `gui/src/panel/tree/hit.rs`
- Modify: `gui/src/panel/tree/layout.rs`

- [ ] **Step 1: Refactor `node.rs`**

Move `style` and `decoration` from `NodeKind` variants into `PanelNode`. Simplify `NodeKind` to discriminator-only.

```rust
// gui/src/panel/tree/node.rs
use std::borrow::Cow;
use crate::widget::layout::{BoxStyle, Decoration, LeafKind};
use crate::renderer::Rect;

pub type NodeId = usize;

#[derive(PartialEq)]
pub enum NodeKind {
    Container,
    Leaf(LeafKind),
}

pub struct PanelNode {
    pub id: Cow<'static, str>,
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub kind: NodeKind,
    pub rect: Rect,
    pub children: Vec<NodeId>,
    pub scroll_offset: f32,
    pub content_height: f32,
}

impl PanelNode {
    pub fn props_match(&self, style: &BoxStyle, decoration: &Option<Decoration>, kind: &NodeKind) -> bool {
        self.style == *style && self.decoration == *decoration && self.kind == *kind
    }
}
```

- [ ] **Step 2: Update `desc.rs` — `into_parts()` returns style/decoration separately**

Adjust `into_parts()` to return style and decoration as separate values so `create_from_desc` can set them on PanelNode:

```rust
pub(crate) fn into_parts(self) -> (Cow<'static, str>, Vec<Desc>, BoxStyle, Option<Decoration>, NodeKind) {
    match self {
        Desc::Container { id, style, decoration, children } => {
            (id, children, style, decoration, NodeKind::Container)
        }
        Desc::Leaf { id, style, kind } => {
            (id, Vec::new(), style, None, NodeKind::Leaf(kind))
        }
    }
}
```

- [ ] **Step 3: Update `diff.rs` — use node-level style/decoration**

In `reconcile_node()` and `create_from_desc()`, set `style` and `decoration` on the node directly. Use `props_match()` for change detection.

```rust
fn reconcile_node(tree: &mut PanelTree, node_id: NodeId, desc: Desc) {
    let (id, desc_children, new_style, new_decoration, new_kind) = desc.into_parts();

    if let Some(node) = tree.get_mut(node_id) {
        if !node.props_match(&new_style, &new_decoration, &new_kind) {
            node.style = new_style;
            node.decoration = new_decoration;
            node.kind = new_kind;
        }
    }

    // child reconciliation unchanged ...
}

fn create_from_desc(tree: &mut PanelTree, desc: Desc) -> NodeId {
    let (id, child_descs, style, decoration, kind) = desc.into_parts();
    let node_id = tree.insert(PanelNode {
        id,
        style,
        decoration,
        kind,
        rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
        children: Vec::new(),
        scroll_offset: 0.0,
        content_height: 0.0,
    });
    // recurse children unchanged ...
    node_id
}
```

- [ ] **Step 4: Update `paint.rs` — read `node.decoration` directly**

```rust
fn paint_node(tree: &PanelTree, node_id: NodeId, renderer: &mut Renderer) {
    let Some(node) = tree.get(node_id) else { return };
    let rect = node.rect;

    // Decoration: all node types
    if let Some(dec) = &node.decoration {
        renderer.draw_rect(rect, &RectStyle {
            color: dec.background.unwrap_or(Color::TRANSPARENT),
            border: dec.border,
            radius: dec.radius,
            shadow: None,
        });
    }

    // Leaf content
    if let NodeKind::Leaf(LeafKind::Text { content, font_size, color }) = &node.kind {
        renderer.draw_text(
            Point { x: rect.x, y: rect.y },
            content,
            &TextStyle { color: *color, size: *font_size },
        );
    }

    let children: Vec<NodeId> = tree.get(node_id).map(|n| n.children.clone()).unwrap_or_default();
    for child_id in children {
        paint_node(tree, child_id, renderer);
    }
}
```

- [ ] **Step 5: Update `hit.rs` — read `node.decoration` directly**

```rust
fn hit_test_node<'a>(tree: &'a PanelTree, node_id: NodeId, x: f32, y: f32) -> Option<&'a str> {
    let Some(node) = tree.get(node_id) else { return None };
    let r = &node.rect;
    if x < r.x || x > r.x + r.w || y < r.y || y > r.y + r.h {
        return None;
    }
    for &child_id in &node.children {
        if let Some(id) = hit_test_node(tree, child_id, x, y) {
            return Some(id);
        }
    }
    if node.decoration.is_some() {
        Some(node.id.as_ref())
    } else {
        None
    }
}
```

- [ ] **Step 6: Update `layout.rs` — read `node.style` directly**

```rust
impl LayoutTree for PanelTree {
    type NodeId = NodeId;

    fn style(&self, node: NodeId) -> &BoxStyle {
        &self.get(node).unwrap().style
    }

    // children, set_rect, scroll_offset, set_content_height unchanged
}
```

- [ ] **Step 7: Verify compilation and demo still runs**

Run: `cargo build -p gui && cargo run -p gui`

- [ ] **Step 8: Commit**

```bash
git add gui/src/panel/tree/
git commit -m "refactor: lift style/decoration to PanelNode level, simplify NodeKind"
```

---

### Task 3: Move desc.rs and node.rs to widget/

**Files:**
- Move: `gui/src/panel/tree/desc.rs` → `gui/src/widget/desc.rs`
- Move: `gui/src/panel/tree/node.rs` → `gui/src/widget/node.rs`
- Modify: `gui/src/widget/mod.rs`
- Modify: `gui/src/panel/tree/mod.rs`
- Modify: `gui/src/panel/tree/diff.rs`
- Modify: `gui/src/panel/tree/paint.rs`
- Modify: `gui/src/panel/tree/hit.rs`
- Modify: `gui/src/panel/tree/layout.rs`
- Modify: `gui/src/panel/tree/tree.rs`

- [ ] **Step 1: Move files**

```bash
mv gui/src/panel/tree/desc.rs gui/src/widget/desc.rs
mv gui/src/panel/tree/node.rs gui/src/widget/node.rs
```

- [ ] **Step 2: Update `widget/mod.rs` — add desc and node modules**

```rust
pub mod atoms;
pub mod desc;
pub mod layout;
pub mod node;

mod action;
mod focus;
mod mapping;
mod state;
mod text_edit;

pub use layout::{BoxStyle, Decoration, LeafKind, LayoutTree, Size};
```

- [ ] **Step 3: Update `widget/desc.rs` imports**

Change `use super::node::NodeKind;` to `use super::node::NodeKind;` (already correct since both are now siblings under widget/).

Verify the `use crate::widget::layout::...` import paths are correct (they should be since both are in the widget module).

- [ ] **Step 4: Update `panel/tree/mod.rs` — re-export from widget**

```rust
mod diff;
mod hit;
mod layout;
mod paint;
mod tree;

pub use crate::widget::desc::Desc;
pub use crate::widget::node::NodeId;
pub use diff::reconcile;
pub use hit::hit_test;
pub use layout::{layout, scroll};
pub use paint::paint;
pub use tree::PanelTree;
```

- [ ] **Step 5: Update imports in panel/tree/ files**

In `diff.rs`, `paint.rs`, `hit.rs`, `layout.rs`, `tree.rs`: change `use super::desc::...` / `use super::node::...` to `use crate::widget::desc::...` / `use crate::widget::node::...`.

- [ ] **Step 6: Verify compilation**

Run: `cargo build -p gui`

- [ ] **Step 7: Commit**

```bash
git add gui/src/widget/desc.rs gui/src/widget/node.rs gui/src/widget/mod.rs gui/src/panel/tree/
git commit -m "refactor: move Desc and NodeKind to widget/ for sharing between panel and canvas"
```

---

### Task 4: Create widget/props.rs

**Files:**
- Create: `gui/src/widget/props.rs`
- Modify: `gui/src/widget/mod.rs`

- [ ] **Step 1: Create `props.rs`**

```rust
// gui/src/widget/props.rs
use std::any::Any;
use std::fmt;
use super::desc::Desc;
use super::layout::{BoxStyle, Decoration};

/// build() 的返回值。提供 Widget 节点的根样式、装饰和展开后的子树。
pub struct WidgetBuild {
    pub style: BoxStyle,
    pub decoration: Option<Decoration>,
    pub children: Vec<Desc>,
}

/// 控件配置 trait。每种控件实现此 trait。
pub trait WidgetProps: 'static {
    fn widget_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn WidgetProps>;
    fn props_eq(&self, other: &dyn WidgetProps) -> bool;
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
    fn build(&self, id: &str) -> WidgetBuild;
}

impl Clone for Box<dyn WidgetProps> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn WidgetProps> {
    fn eq(&self, other: &Self) -> bool {
        self.props_eq(other.as_ref())
    }
}

impl fmt::Debug for Box<dyn WidgetProps> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_fmt(f)
    }
}
```

- [ ] **Step 2: Export from `widget/mod.rs`**

Add `pub mod props;` to `gui/src/widget/mod.rs`.

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p gui`

- [ ] **Step 4: Commit**

```bash
git add gui/src/widget/props.rs gui/src/widget/mod.rs
git commit -m "feat: add WidgetProps trait and WidgetBuild struct"
```

---

### Task 5: Add Widget variant to Desc and NodeKind

**Files:**
- Modify: `gui/src/widget/desc.rs`
- Modify: `gui/src/widget/node.rs`

- [ ] **Step 1: Add Widget to Desc**

Add the Widget variant and update `id()`. Keep `into_parts()` for now — Task 6 will remove it when rewriting reconcile.

```rust
// In Desc enum, add:
Widget {
    id: Cow<'static, str>,
    props: Box<dyn WidgetProps>,
},

// Update id():
Desc::Widget { id, .. } => id,

// into_parts() stays unchanged — it only handles Container/Leaf.
// Widget paths go through the new reconcile logic in Task 6.
```

- [ ] **Step 2: Add Widget to NodeKind**

```rust
#[derive(PartialEq)]
pub enum NodeKind {
    Container,
    Leaf(LeafKind),
    Widget(Box<dyn WidgetProps>),
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p gui`

Expect warnings about unused Widget variant — that's fine, we'll use it in the next task.

- [ ] **Step 4: Commit**

```bash
git add gui/src/widget/desc.rs gui/src/widget/node.rs
git commit -m "feat: add Widget variant to Desc and NodeKind"
```

---

### Task 6: Update reconcile for Widget variant

**Files:**
- Modify: `gui/src/panel/tree/diff.rs`
- Modify: `gui/src/widget/desc.rs` (remove `into_parts()`)

- [ ] **Step 1: Remove `into_parts()` from `desc.rs`**

Delete the `into_parts()` method from `Desc` in `gui/src/widget/desc.rs`. The new reconcile code matches Desc variants directly.

- [ ] **Step 2: Rewrite reconcile to match Desc variants directly**

Match each Desc variant explicitly. Add Widget handling that calls `props.build()` and reconciles children.

```rust
use crate::widget::desc::Desc;
use crate::widget::node::{NodeId, NodeKind, PanelNode};
use super::tree::PanelTree;
use crate::renderer::Rect;

pub fn reconcile(tree: &mut PanelTree, desc: Desc) {
    if let Some(root_id) = tree.root() {
        reconcile_node(tree, root_id, desc);
    } else {
        let root_id = create_from_desc(tree, desc);
        tree.set_root(root_id);
    }
}

fn reconcile_node(tree: &mut PanelTree, node_id: NodeId, desc: Desc) {
    match desc {
        Desc::Container { id: _, style, decoration, children } => {
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &decoration, &NodeKind::Container) {
                    node.style = style;
                    node.decoration = decoration;
                    node.kind = NodeKind::Container;
                }
            }
            reconcile_children(tree, node_id, children);
        }
        Desc::Leaf { id: _, style, kind } => {
            let new_kind = NodeKind::Leaf(kind);
            if let Some(node) = tree.get_mut(node_id) {
                if !node.props_match(&style, &None, &new_kind) {
                    node.style = style;
                    node.decoration = None;
                    node.kind = new_kind;
                }
            }
        }
        Desc::Widget { id, props } => {
            let props_changed = tree.get(node_id)
                .map(|n| match &n.kind {
                    NodeKind::Widget(old) => !old.props_eq(props.as_ref()),
                    _ => true,
                })
                .unwrap_or(true);

            if props_changed {
                let wb = props.build(&id);
                if let Some(node) = tree.get_mut(node_id) {
                    node.style = wb.style;
                    node.decoration = wb.decoration;
                    node.kind = NodeKind::Widget(props);
                }
                reconcile_children(tree, node_id, wb.children);
            }
        }
    }
}

fn reconcile_children(tree: &mut PanelTree, node_id: NodeId, desc_children: Vec<Desc>) {
    let old_children: Vec<NodeId> = tree
        .get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();

    let mut new_children = Vec::new();
    let mut old_map: Vec<(NodeId, String)> = old_children
        .iter()
        .filter_map(|&id| tree.get(id).map(|n| (id, n.id.as_ref().to_owned())))
        .collect();

    for child_desc in desc_children {
        let child_id_str = child_desc.id();
        if let Some(pos) = old_map.iter().position(|(_, id)| id == child_id_str) {
            let (existing_id, _) = old_map.remove(pos);
            reconcile_node(tree, existing_id, child_desc);
            new_children.push(existing_id);
        } else {
            let new_id = create_from_desc(tree, child_desc);
            new_children.push(new_id);
        }
    }

    for (old_id, _) in old_map {
        tree.remove(old_id);
    }

    if let Some(node) = tree.get_mut(node_id) {
        node.children = new_children;
    }
}

fn create_from_desc(tree: &mut PanelTree, desc: Desc) -> NodeId {
    let (id, style, decoration, kind, child_descs) = match desc {
        Desc::Container { id, style, decoration, children } => {
            (id, style, decoration, NodeKind::Container, children)
        }
        Desc::Leaf { id, style, kind } => {
            (id, style, None, NodeKind::Leaf(kind), Vec::new())
        }
        Desc::Widget { id, props } => {
            let wb = props.build(&id);
            (id, wb.style, wb.decoration, NodeKind::Widget(props), wb.children)
        }
    };

    let node_id = tree.insert(PanelNode {
        id,
        style,
        decoration,
        kind,
        rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
        children: Vec::new(),
        scroll_offset: 0.0,
        content_height: 0.0,
    });

    let child_ids: Vec<NodeId> = child_descs
        .into_iter()
        .map(|d| create_from_desc(tree, d))
        .collect();

    if let Some(node) = tree.get_mut(node_id) {
        node.children = child_ids;
    }

    node_id
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p gui`

- [ ] **Step 4: Commit**

```bash
git add gui/src/widget/desc.rs gui/src/panel/tree/diff.rs
git commit -m "feat: reconcile supports Widget variant with props.build() expansion"
```

---

### Task 7: Create layout/resolve.rs and add iter_mut to PanelTree

**Files:**
- Create: `gui/src/widget/layout/resolve.rs`
- Modify: `gui/src/widget/layout/mod.rs`
- Modify: `gui/src/panel/tree/tree.rs`

- [ ] **Step 1: Add `iter_mut()` to `PanelTree`**

Add to `gui/src/panel/tree/tree.rs`:

```rust
pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PanelNode> {
    self.nodes.iter_mut().filter_map(|n| n.as_mut())
}
```

- [ ] **Step 2: Create `resolve.rs`**

```rust
// gui/src/widget/layout/resolve.rs
use crate::widget::node::NodeKind;
use crate::widget::layout::{LeafKind, Size};
use crate::renderer::TextMeasurer;
use crate::panel::tree::PanelTree;

/// 解析叶子节点的内容尺寸。遍历树，用 TextMeasurer 将 Auto 尺寸的 Text 叶子量好。
pub fn resolve(tree: &mut PanelTree, measurer: &mut TextMeasurer) {
    for node in tree.iter_mut() {
        if let NodeKind::Leaf(LeafKind::Text { content, font_size, .. }) = &node.kind {
            if node.style.width == Size::Auto {
                let (w, h) = measurer.measure(content, *font_size);
                node.style.width = Size::Fixed(w);
                node.style.height = Size::Fixed(h);
            }
        }
    }
}
```

- [ ] **Step 3: Export resolve from layout module**

Add `pub mod resolve;` to `gui/src/widget/layout/mod.rs`.

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p gui`

- [ ] **Step 5: Commit**

```bash
git add gui/src/widget/layout/resolve.rs gui/src/widget/layout/mod.rs gui/src/panel/tree/tree.rs
git commit -m "feat: add layout/resolve phase for text measurement"
```

---

### Task 8: Create ButtonProps (first atom)

**Files:**
- Modify: `gui/src/widget/atoms/button.rs`
- Modify: `gui/src/widget/atoms/mod.rs`

- [ ] **Step 1: Implement ButtonProps**

Rewrite `gui/src/widget/atoms/button.rs`:

```rust
use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use crate::widget::desc::Desc;
use crate::widget::layout::{BoxStyle, Decoration, LeafKind, Size};
use crate::widget::props::{WidgetBuild, WidgetProps};

#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    pub label: Cow<'static, str>,
    pub icon: Option<Cow<'static, str>>,
    pub disabled: bool,
}

impl WidgetProps for ButtonProps {
    fn widget_type(&self) -> &'static str { "Button" }
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn WidgetProps> { Box::new(self.clone()) }
    fn props_eq(&self, other: &dyn WidgetProps) -> bool {
        other.as_any().downcast_ref::<Self>().map_or(false, |o| self == o)
    }
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
    fn build(&self, id: &str) -> WidgetBuild {
        // TODO: actual build logic in future task
        todo!("ButtonProps::build")
    }
}
```

- [ ] **Step 2: Update `atoms/mod.rs`**

```rust
pub mod button;
```

(Already declares button, but verify it compiles with the new content.)

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p gui`

- [ ] **Step 4: Commit**

```bash
git add gui/src/widget/atoms/button.rs gui/src/widget/atoms/mod.rs
git commit -m "feat: add ButtonProps with WidgetProps impl"
```

---

### Task 9: Create remaining atom Props

**Files:**
- Create: `gui/src/widget/atoms/slider.rs`
- Create: `gui/src/widget/atoms/toggle.rs`
- Create: `gui/src/widget/atoms/text_input.rs`
- Create: `gui/src/widget/atoms/dropdown.rs`
- Modify: `gui/src/widget/atoms/mod.rs`

- [ ] **Step 1: Create `slider.rs`**

Same pattern as ButtonProps:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    pub label: Cow<'static, str>,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value: f32,
    pub disabled: bool,
}
```

Plus full `WidgetProps` impl with `build()` → `todo!()`.

- [ ] **Step 2: Create `toggle.rs`**

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleProps {
    pub label: Cow<'static, str>,
    pub value: bool,
    pub disabled: bool,
}
```

Plus full `WidgetProps` impl.

- [ ] **Step 3: Create `text_input.rs`**

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputProps {
    pub label: Cow<'static, str>,
    pub value: Cow<'static, str>,
    pub disabled: bool,
}
```

Plus full `WidgetProps` impl.

- [ ] **Step 4: Create `dropdown.rs`**

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownProps {
    pub label: Cow<'static, str>,
    pub options: Vec<Cow<'static, str>>,
    pub selected: usize,
    pub disabled: bool,
}
```

Plus full `WidgetProps` impl.

- [ ] **Step 5: Update `atoms/mod.rs`**

```rust
pub mod button;
pub mod dropdown;
pub mod slider;
pub mod text_input;
pub mod toggle;
```

- [ ] **Step 6: Verify compilation**

Run: `cargo build -p gui`

- [ ] **Step 7: Commit**

```bash
git add gui/src/widget/atoms/
git commit -m "feat: add SliderProps, ToggleProps, TextInputProps, DropdownProps"
```

---

### Task 10: Update demo.rs to use Widget + ButtonProps

**Files:**
- Modify: `gui/src/demo.rs`

- [ ] **Step 1: Implement ButtonProps.build() with actual logic**

Go back to `gui/src/widget/atoms/button.rs` and implement `build()`:

```rust
fn build(&self, id: &str) -> WidgetBuild {
    use crate::renderer::Color;
    let text_color = Color { r: 0.094, g: 0.094, b: 0.106, a: 1.0 };
    let font_size = 12.0;

    WidgetBuild {
        style: BoxStyle {
            height: Size::Auto,
            direction: crate::widget::layout::Direction::Row,
            align_items: crate::widget::layout::Align::Center,
            justify_content: crate::widget::layout::Justify::Center,
            padding: crate::widget::layout::Edges::symmetric(8.0, 16.0),
            ..BoxStyle::default()
        },
        decoration: None, // decoration set by caller via wrapper or theme
        children: vec![
            Desc::Leaf {
                id: Cow::Owned(format!("{id}::label")),
                style: BoxStyle {
                    width: Size::Auto,
                    height: Size::Auto,
                    ..BoxStyle::default()
                },
                kind: LeafKind::Text {
                    content: self.label.to_string(),
                    font_size,
                    color: text_color,
                },
            },
        ],
    }
}
```

- [ ] **Step 2: Rewrite `demo.rs` to use `Desc::Widget` + `ButtonProps`**

Replace the `button()` function and `build_view()`:

```rust
fn build_view(active: Option<&str>) -> Desc {
    Desc::Container {
        id: Cow::Borrowed("__root"),
        style: BoxStyle {
            direction: Direction::Column,
            gap: 8.0,
            ..BoxStyle::default()
        },
        decoration: None,
        children: vec![
            Desc::Widget {
                id: Cow::Borrowed("btn_a"),
                props: Box::new(ButtonProps {
                    label: "Button A".into(),
                    icon: None,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("btn_b"),
                props: Box::new(ButtonProps {
                    label: "Button B".into(),
                    icon: None,
                    disabled: false,
                }),
            },
        ],
    }
}
```

Update `DemoApp::update()` — remove renderer from `build_view` call:

```rust
fn update(&mut self, renderer: &mut Renderer, ctx: &mut AppContext) {
    // cursor logic unchanged...
    let desc = build_view(self.active_button.as_deref());
    reconcile(&mut self.tree, desc);
}
```

Update `DemoApp::render()` — call resolve before layout. Add `pub fn text_measurer(&mut self) -> &mut TextMeasurer` to `Renderer` to expose it:

```rust
fn render(&mut self, renderer: &mut Renderer, ctx: &AppContext) {
    if let (Some(frame), Some(root)) = (self.layer.get("demo"), self.tree.root()) {
        let content_rect = Rect { ... };
        resolve::resolve(&mut self.tree, renderer.text_measurer());
        layout(&mut self.tree, root, content_rect);
    }
    // rest unchanged...
}
```

- [ ] **Step 3: Verify compilation and demo runs**

Run: `cargo build -p gui && cargo run -p gui`

The demo should display two buttons rendered via the Widget Props system. Text should be measured by resolve, not in build_view.

- [ ] **Step 4: Commit**

```bash
git add gui/src/widget/atoms/button.rs gui/src/demo.rs gui/src/renderer/renderer.rs
git commit -m "feat: demo uses Widget Props system with ButtonProps"
```

- [ ] **Step 5: Update `LeafKind::Text` doc comment**

In `gui/src/widget/layout/types.rs`, update the comment on `LeafKind::Text` (line 135):

Change from: `/// 文本图元。尺寸在创建时由 measure_text() 预算，存入 BoxStyle 的 width/height。`
To: `/// 文本图元。尺寸在 resolve 阶段由 TextMeasurer 解析，创建时使用 Size::Auto。`

- [ ] **Step 6: Commit doc update**

```bash
git add gui/src/widget/layout/types.rs
git commit -m "docs: update LeafKind::Text comment to reflect resolve phase"
```
