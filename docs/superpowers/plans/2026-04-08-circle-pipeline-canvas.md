# 圆形支持（CirclePipeline）+ 无限画布

## Context

画布点阵需要画圆点，后续节点引脚也需要画圆。当前 renderer 没有圆形图元。新增独立的 CirclePipeline，用 SDF shader 实现亚像素抗锯齿。然后基于此实现无限画布。

## 第一部分：CirclePipeline

### 新增文件

| 文件 | 职责 |
|------|------|
| `renderer/circle.rs` | CirclePipeline + CircleRequest + CircleVertex |
| `renderer/shaders/circle.wgsl` | SDF 圆形 shader |

### 改动文件

| 文件 | 改动 |
|------|------|
| `renderer/command.rs` | 新增 `DrawCommand::Circle(CircleRequest)` |
| `renderer/prepare.rs` | 新增 `flush_circle_batch()` + `DrawOp::Circle` + `PreparedFrame` 加 circle 字段 |
| `renderer/dispatch.rs` | 新增 `DrawOp::Circle` 分支，circle_pipeline.bindm/draw |
| `renderer/renderer.rs` | 新增 `circle_pipeline` 字段 + `draw_circle()` 方法 |
| `renderer/mod.rs` | 新增 `mod circle` |

### circle.wgsl

SDF 方式：每个圆形生成一个轴对齐矩形（4 顶点 6 索引），fragment shader 算像素到圆心的距离，smoothstep 抗锯齿边缘。

```wgsl
// vertex: 传入 position(矩形顶点), center, radius, color
// fragment: dist = length(frag_pos - center) - radius
//           alpha = 1.0 - smoothstep(-1.0, 1.0, dist)  // 1px 软边
```

### CircleRequest / CircleVertex

```rust
pub struct CircleRequest {
    pub center: Point,
    pub radius: f32,
    pub color: Color,
}

struct CircleVertex {
    position: [f32; 2],   // 矩形顶点坐标
    center: [f32; 2],     // 圆心
    radius: f32,
    color: [f32; 4],
}
```

### prepare.rs 中的批处理

每个 CircleRequest 生成 4 个顶点（center ± radius 的矩形）+ 6 个索引。连续的 Circle 命令合并为一个 draw call。

---

## 第二部分：无限画布

### 新增文件

| 文件 | 职责 |
|------|------|
| `canvas/mod.rs` | Canvas 结构体 + 模块导出 |
| `canvas/camera.rs` | 相机状态（offset_x, offset_y, zoom）+ 屏幕 ↔ 画布坐标转换 |
| `canvas/background.rs` | 背景颜色 + 点阵网格渲染 |
| `canvas/pan.rs` | 拖拽平移交互 |

### camera.rs

```rust
pub struct Camera {
    pub x: f32,      // 画布原点在屏幕上的 x
    pub y: f32,      // 画布原点在屏幕上的 y
    pub zoom: f32,   // 缩放倍率，1.0 = 100%
}

impl Camera {
    pub fn screen_to_canvas(&self, sx: f32, sy: f32) -> (f32, f32)
    pub fn canvas_to_screen(&self, cx: f32, cy: f32) -> (f32, f32)
    pub fn zoom_at(&mut self, sx: f32, sy: f32, delta: f32)  // 以鼠标位置为中心缩放
}
```

### background.rs

```rust
pub fn render(renderer: &mut Renderer, camera: &Camera, viewport_w: f32, viewport_h: f32) {
    // 1. 全屏深色背景
    // 2. 根据 camera 计算可见区域内的点阵坐标
    // 3. 每个点 draw_circle(小圆点，2px 半径，淡灰色)
    //    点间距 = 20px 画布坐标，缩放后在屏幕上大小变化
    //    缩放太小时点不画（避免密集）
}
```

### pan.rs

和 panel/drag.rs 类似的状态机：

```rust
pub struct PanState {
    active: bool,
    last_x: f32,
    last_y: f32,
}

impl PanState {
    pub fn start(&mut self, x, y)
    pub fn update(&mut self, x, y, camera: &mut Camera)
    pub fn end(&mut self)
}
```

### demo.rs 集成

- 先渲染画布（背景 + 点阵）
- 再渲染面板（面板在画布上方）
- 事件：面板优先拦截，没命中面板的鼠标事件给画布
- 中键/右键拖拽 = 画布平移，滚轮 = 缩放

## 实现顺序

1. circle.rs + circle.wgsl — 圆形管线
2. command/prepare/dispatch/renderer — 接入圆形
3. canvas/camera.rs — 相机
4. canvas/background.rs — 背景 + 点阵
5. canvas/pan.rs — 平移
6. canvas/mod.rs — 组装
7. demo.rs — 画布 + 面板共存

## 验证

运行 demo：
- 深色背景 + 点阵网格
- 右键/中键拖拽平移画布
- 滚轮缩放（以鼠标为中心）
- 面板浮在画布上方，可拖拽 resize
- 点阵圆点边缘平滑（SDF 抗锯齿）
