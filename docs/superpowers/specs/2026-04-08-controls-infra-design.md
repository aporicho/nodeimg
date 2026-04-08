# 控件基础设施设计

## Context

面板控件系统需要 12 种原子控件 + 3 种组合控件。当前 panel_tree 的布局是硬编码高度（`BUTTON_HEIGHT=40`），无法支撑真正的控件系统。需要补齐四块基础设施：文字度量、布局引擎、焦点管理、文字编辑。

布局引擎采用 Flexbox + 盒模型方案——和网页相同的心智模型，覆盖面板 UI 99% 的场景。

## 文件结构

```
gui/src/controls/
├── mod.rs
├── infra/
│   ├── mod.rs
│   ├── layout/
│   │   ├── mod.rs          公开 API：layout() + LayoutResult
│   │   ├── types.rs        LayoutBox、BoxStyle、Size、Direction、Align 等
│   │   ├── measure.rs      第一遍：自底向上计算期望尺寸
│   │   ├── arrange.rs      第二遍：自顶向下分配位置
│   │   └── scroll.rs       滚动状态管理
│   ├── focus.rs            焦点管理
│   └── text_edit.rs        文字编辑状态机
├── atoms/                  （后续）
└── composites/             （后续）
```

文字度量不单独建文件，在 `renderer/text.rs` 加 measure 方法。

---

## 1. 文字度量

在 TextPipeline 加 measure 方法，Renderer 暴露 `measure_text`。

```rust
// renderer/text.rs
impl TextPipeline {
    pub fn measure(&mut self, text: &str, size: f32) -> (f32, f32) {
        // 复用 buffer_cache（和 prepare 一样的 TextCacheKey 逻辑）
        // Buffer::new → set_size(f32::MAX, size*2) → set_text → shape_until_scroll
        // 遍历 layout_runs()：width = max(line_w)，height = sum(line_height)
    }
}

// renderer/renderer.rs
impl Renderer {
    pub fn measure_text(&mut self, text: &str, size: f32) -> (f32, f32) {
        self.text_pipeline.measure(text, size)
    }
}
```

### 改动文件
- `renderer/text.rs` — 加 `measure` 方法
- `renderer/renderer.rs` — 加 `measure_text` 方法

---

## 2. 布局引擎（controls/infra/layout.rs）

### 核心模型：统一的 LayoutBox

每个节点都是一个 Box，有样式和子节点。不区分容器和叶子。

```rust
pub struct LayoutBox {
    pub id: Option<&'static str>,
    pub style: BoxStyle,
    pub children: Vec<LayoutBox>,
}
```

### BoxStyle

```rust
pub struct BoxStyle {
    // ── 盒模型 ──
    pub padding: Edges,          // 内边距
    pub margin: Edges,           // 外边距
    pub width: Size,             // Fixed(f32) | Auto | Fill
    pub height: Size,

    // ── Flexbox ──
    pub direction: Direction,    // Row | Column
    pub gap: f32,                // 子元素间距
    pub align_items: Align,      // Start | Center | End | Stretch
    pub justify_content: Justify,// Start | Center | End | SpaceBetween
    pub flex_grow: f32,          // 0.0 = 不伸缩，>0 = 按比例分配剩余空间

    // ── 溢出 ──
    pub overflow: Overflow,      // Visible | Hidden | Scroll
}

pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

pub enum Size {
    Fixed(f32),
    Auto,       // 由内容撑开
    Fill,       // 占满剩余空间（等价于 flex_grow: 1）
}

pub enum Direction { Row, Column }

pub enum Align { Start, Center, End, Stretch }

pub enum Justify { Start, Center, End, SpaceBetween }

pub enum Overflow { Visible, Hidden, Scroll }
```

### 默认值

```rust
impl Default for BoxStyle {
    fn default() -> Self {
        Self {
            padding: Edges::ZERO,
            margin: Edges::ZERO,
            width: Size::Auto,
            height: Size::Auto,
            direction: Direction::Column,
            gap: 0.0,
            align_items: Align::Stretch,
            justify_content: Justify::Start,
            flex_grow: 0.0,
            overflow: Overflow::Visible,
        }
    }
}
```

### 两遍算法

**第一遍：measure（自底向上）**

计算每个节点的期望尺寸 `(desired_width, desired_height)`：

```
Leaf（无 children）:
  Auto → (0, 0)，由调用方在构建 LayoutBox 时通过 Fixed 指定
  Fixed(px) → (px, px)

Container（有 children）:
  主轴方向（如 Column 的高度）= sum(子期望高度) + gap*(n-1) + padding.top + padding.bottom
  交叉轴方向（如 Column 的宽度）= max(子期望宽度) + padding.left + padding.right
  Fixed → 覆盖对应维度
  Fill → 标记为需要弹性分配（此遍返回 0，arrange 时填充）
```

**第二遍：arrange（自顶向下）**

给定可用空间，计算每个节点的最终 Rect：

```
1. 扣除 margin → 可用矩形
2. 扣除 padding → 内容矩形
3. 沿主轴方向排列子节点：
   a. 分配固定/Auto 子节点的空间
   b. 剩余空间按 flex_grow 比例分配给 Fill 子节点
   c. 根据 justify_content 计算起始偏移
4. 交叉轴方向根据 align_items 对齐
5. overflow == Scroll 时记录 scroll 状态
```

### 输出

```rust
pub struct LayoutResult {
    pub rects: Vec<(&'static str, Rect)>,  // id → 最终位置
    pub scroll_areas: Vec<ScrollArea>,      // 可滚动区域信息
}

pub struct ScrollArea {
    pub id: &'static str,
    pub viewport: Rect,      // 可见区域
    pub content_height: f32,  // 内容总高度
    pub offset: f32,          // 当前滚动偏移
}
```

### 公开 API

```rust
pub fn layout(root: &LayoutBox, available: Rect) -> LayoutResult;
```

### 与 panel_tree 集成

panel_tree/layout.rs 改为：
1. Desc → LayoutBox（控件用 renderer.measure_text 算尺寸，设为 Fixed）
2. 调用 `controls::infra::layout::layout(root, content_rect)`
3. 把 LayoutResult.rects 写回 PanelNode.rect

### 新增文件
- `controls/infra/layout/mod.rs` — 公开 API
- `controls/infra/layout/types.rs` — 类型定义
- `controls/infra/layout/measure.rs` — measure 遍
- `controls/infra/layout/arrange.rs` — arrange 遍
- `controls/infra/layout/scroll.rs` — 滚动状态

### 改动文件
- `panel_tree/layout.rs` — 调用新布局引擎

---

## 3. 焦点管理（controls/infra/focus.rs）

```rust
pub struct FocusState {
    focused: Option<&'static str>,
    focusable: Vec<&'static str>,  // Tab 顺序
}

impl FocusState {
    pub fn new() -> Self;

    /// 每帧由布局阶段更新（遍历 Desc 树收集输入类控件 ID）
    pub fn set_focusable(&mut self, ids: Vec<&'static str>);

    pub fn focus(&mut self, id: &'static str);
    pub fn blur(&mut self);
    pub fn is_focused(&self, id: &str) -> bool;
    pub fn focused(&self) -> Option<&str>;

    /// Tab 循环
    pub fn tab_next(&mut self);
    pub fn tab_prev(&mut self);  // Shift+Tab
}
```

focusable 列表在布局阶段构建——遍历 Desc 树时收集 TextInput、Dropdown 等输入类控件 ID，按出现顺序排列。

### 新增文件
- `controls/infra/focus.rs`

---

## 4. 文字编辑（controls/infra/text_edit.rs）

TextInput 和 TextArea 共用的编辑状态机。纯逻辑，不涉及渲染。

```rust
pub struct TextEditState {
    text: String,
    cursor: usize,              // 光标位置（字符索引）
    selection_anchor: Option<usize>,  // 选择锚点
}

impl TextEditState {
    pub fn new(text: &str) -> Self;

    // ── 读取 ──
    pub fn text(&self) -> &str;
    pub fn cursor(&self) -> usize;
    pub fn has_selection(&self) -> bool;
    pub fn selection_range(&self) -> Option<(usize, usize)>;  // (start, end) 有序
    pub fn selected_text(&self) -> &str;

    // ── 编辑 ──
    pub fn insert_char(&mut self, ch: char);
    pub fn insert_str(&mut self, s: &str);
    pub fn backspace(&mut self);
    pub fn delete(&mut self);
    pub fn set_text(&mut self, text: &str);  // 外部设值

    // ── 光标移动 ──
    pub fn move_left(&mut self);
    pub fn move_right(&mut self);
    pub fn move_home(&mut self);
    pub fn move_end(&mut self);
    pub fn move_to(&mut self, pos: usize);  // 点击定位

    // ── 选择 ──
    pub fn select_left(&mut self);   // Shift+Left
    pub fn select_right(&mut self);  // Shift+Right
    pub fn select_all(&mut self);    // Ctrl/Cmd+A
    pub fn select_to(&mut self, pos: usize);  // Shift+Click

    // ── 剪贴板（返回操作意图，不直接访问系统剪贴板）──
    pub fn copy(&self) -> Option<String>;      // 返回选中文字
    pub fn cut(&mut self) -> Option<String>;   // 返回选中文字并删除
    pub fn paste(&mut self, text: &str);       // 粘贴
}
```

### 新增文件
- `controls/infra/text_edit.rs`

---

## 实现顺序

1. 文字度量（renderer/text.rs + renderer.rs）
2. 布局类型（controls/infra/layout/types.rs）
3. measure 遍（controls/infra/layout/measure.rs）
4. arrange 遍（controls/infra/layout/arrange.rs）
5. 滚动状态（controls/infra/layout/scroll.rs）
6. 布局 API（controls/infra/layout/mod.rs）
7. panel_tree/layout.rs 改造——调用新布局引擎
8. 焦点管理（controls/infra/focus.rs）
9. 文字编辑（controls/infra/text_edit.rs）
10. mod.rs 文件（controls/mod.rs + controls/infra/mod.rs）

每步完成后 `cargo build -p gui` 验证。

## 验证

1. 编译通过
2. demo 运行正常——面板内按钮用新布局引擎排列，视觉效果不变
3. `renderer.measure_text("Hello", 16.0)` 返回合理的 (width, height)
4. 布局引擎：Column + Row + gap + padding 组合正确计算位置
