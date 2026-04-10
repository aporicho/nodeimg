# 预览面板设计

## 概述

浮动面板，显示图片预览。支持图片缩放/平移、面板拖拽移动、面板 resize。

## 结构

```
preview.rs
├── PreviewPanel
│   ├── panel: FloatingPanel     — 拖拽移动 + resize
│   ├── image_handle: Option<image::Handle>  — 加载的图片
│   ├── offset: Vector           — 图片内平移偏移
│   ├── scale: f32               — 图片缩放比例
│   └── view() / update()
```

## 图片渲染

使用 iced `canvas::Program`，在 `draw()` 中用 `frame.draw_image()` 绘制图片。需在 `gui/Cargo.toml` 的 iced features 中添加 `"image"`。

缩放/平移逻辑和节点画布一致：
- 滚轮缩放（以光标为中心）
- 左键拖拽平移（canvas 区域内，标题栏下方）

## FloatingPanel 扩展

现有 `FloatingPanel` 只支持拖拽移动。扩展：
- `FloatingPanel::new(size)` — 构造时传入初始 size
- 新增 `size: Size` 字段，存储面板宽高
- 面板右下角 resize handle，拖拽时更新 size
- 新增 `ResizeStart` / `ResizeMove(Point)` / `ResizeEnd` 事件
- `is_resizing()` 方法，和 `is_dragging()` 互斥
- drag 和 resize 不会同时发生

## 面板布局

- 默认位置：右上角。使用 `float()` + `translate()` 定位，初始 offset 使预览面板出现在右上角（根据窗口尺寸计算）
- 默认大小：300x250
- 浮在画布上方，和工具栏同层放在 `stack!` 中
- 面板结构：标题栏（28px 高，显示 "Preview"）+ canvas 内容区
- drag 和 resize 的全屏捕获层：在 `lib.rs` 中统一处理，`FloatingPanel::is_interacting()` 返回 drag 或 resize 任一激活状态

## 图片来源

启动时加载 `assets/test/test.png`。路径相对于 cargo workspace root。文件不存在时面板显示空白。

## 交互

| 操作 | 区域 | 行为 |
|------|------|------|
| 左键拖拽 | 标题栏（顶部 28px） | 移动面板 |
| 左键拖拽 | 右下角 resize handle | 调整面板大小 |
| 滚轮 | canvas 内容区 | 缩放图片 |
| 左键拖拽 | canvas 内容区 | 平移图片 |

## 文件变更

| 文件 | 变更 |
|------|------|
| `gui/Cargo.toml` | iced features 添加 `"image"` |
| `gui/src/panel.rs` | 扩展 resize 能力（size + resize 事件 + `is_interacting()`） |
| `gui/src/preview.rs` | 实现 PreviewPanel（canvas Program + 图片加载） |
| `gui/src/lib.rs` | 集成预览面板到布局，统一 drag/resize 全屏捕获层 |
