# 阴影管线设计

> 为渲染器添加高斯模糊阴影能力，对齐 Figma drop shadow 效果。

## 方案选择

**方案 A（业界标准）**：每个阴影独立临时纹理 + 两遍可分离高斯模糊 + 合成。Chrome/Skia/Flutter 对非矩形阴影的标准做法。

降采样和缓存作为内置能力一起实现。

后续如果性能不够，可升级为方案 C（按 blur 半径分组，共享纹理 + 批量 blur）。

## 核心流程

```
draw_rect(rect, style) — style.shadow 存在时：
  → 收集 DrawCommand::Shadow(ShadowRequest)
  → 收集 DrawCommand::Rect(QuadRequest)

end_frame dispatch 遇到 Shadow：
  1. 查缓存（key = rect大小 + radius + blur + spread + color）
     → 命中：跳到第 5 步
     → miss：继续
  2. 创建临时 RGBA 纹理（rect + offset + spread + blur 扩展范围，按 downsample_factor 缩放）
  3. 用 lyon 圆角矩形路径画阴影形状到临时纹理，颜色烘焙
  4. 两遍高斯模糊 compute shader（水平 → 中间纹理 → 垂直 → 回临时纹理）
  5. shadow_composite shader 把模糊结果合成到主画面（alpha blend）
  6. 缓存模糊后的纹理
```

## 数据类型

### Shadow（style.rs 改动）

```rust
pub struct Shadow {
    pub color: Color,
    pub offset: [f32; 2],  // x, y 偏移
    pub blur: f32,          // 模糊半径
    pub spread: f32,        // 扩展距离
}
```

### RectStyle（style.rs 改动）

```rust
pub struct RectStyle {
    pub color: Color,
    pub border: Option<Border>,
    pub radius: [f32; 4],
    pub shadow: Option<Shadow>,  // 新增
}
```

### ShadowRequest（shadow.rs）

```rust
struct ShadowRequest {
    pub rect: Rect,           // 原矩形
    pub radius: [f32; 4],     // 圆角
    pub shadow: Shadow,       // 阴影参数
}
```

### DrawCommand（command.rs 改动）

```rust
enum DrawCommand {
    Shadow(ShadowRequest),  // 新增
    Rect(QuadRequest),
    Text(TextRequest),
    Image { rect: Rect, view: Arc<TextureView> },
    Curve(CurveRequest),
    PushClip { rect: Rect, radius: f32 },
    PopClip,
}
```

## 文件结构

### 新增

| 文件 | 职责 |
|------|------|
| `shadow.rs` | ShadowPipeline：临时纹理管理、lyon 形状绘制、blur 调度、缓存、合成 |
| `shaders/gaussian_blur.wgsl` | 两遍可分离高斯模糊 compute shader（16x16 workgroup） |
| `shaders/shadow_composite.wgsl` | 阴影纹理合成到主画面 |

### 改动

| 文件 | 改动 |
|------|------|
| `style.rs` | Shadow 加 spread 字段；RectStyle 加 shadow 字段 |
| `command.rs` | DrawCommand 新增 Shadow 变体 |
| `dispatch.rs` | 处理 Shadow 指令（blur pass 在主 render pass 外执行，合成在主 render pass 内） |
| `renderer.rs` | draw_rect 拆阴影指令；Renderer 持有 ShadowPipeline |
| `mod.rs` | 导出 shadow 模块 |
| `demo.rs` | 阴影效果测试 |

## gaussian_blur.wgsl

- 一个 compute shader，通过 uniform 的 `direction` 参数控制水平/垂直方向
- workgroup size 16x16（项目约定）
- 高斯核大小由 blur 半径决定，采样范围 = 3σ（σ = blur / 2）
- 输入纹理 → 输出纹理

## shadow_composite.wgsl

- 全屏 quad + 纹理采样
- 把模糊后的阴影 RGBA 纹理 alpha blend 到主画面指定位置

## 缓存

- `HashMap<CacheKey, CachedShadow>`
- key = `(rect.w, rect.h, radius, blur, spread, color)`，不含 offset（合成时偏移）
- 每帧标记使用的 key，连续未使用的条目延迟释放

## 降采样

- `downsample_factor` 参数，默认 1（不降采样）
- 以后改为 2 即可启用半分辨率模糊
- 合成时自动上采样

## 阴影形状

复用 `quad.rs` 的 `build_rounded_rect_path` 生成 Figma 风格圆角矩形路径。阴影形状 = 原矩形按 spread 扩展后的圆角矩形。
