# 内置节点全面实现设计

日期：2026-03-17

## 目标

实现 catalog.md 中定义的全部未实现内置节点（26 个），加上 Film Grain 和 LUT Apply（2 个），共 28 个新节点。同时将 `processing.rs` 重构为 `processing/` 目录模块。

---

## 范围

### 新增节点（28 个）

| 分类 | 节点 | 数量 |
|------|------|------|
| 生成型 (generate) | Solid Color, Gradient, Noise, Checkerboard | 4 |
| 颜色处理型 (color) | Levels, Hue/Saturation, Color Balance, Invert, Threshold, LUT Apply | 6 |
| 空间变换型 (transform) | Resize, Crop, Rotate, Flip, Distort | 5 |
| 滤镜型 (filter) | Blur, Sharpen, Edge Detect, Emboss, Denoise, Pixelate, Vignette, Film Grain | 8 |
| 合成型 (composite) | Blend, Mask, Channel Merge | 3 |
| 工具型 (tool) | Histogram | 1 |

**不在本次范围：**
- Curves（需要新增 Curve 约束类型和 CurveEditor 控件，标记为 v2）
- AI 型节点（需要 Python 后端）

### 重构

将 `src/processing.rs` 升级为 `src/processing/` 模块目录。

---

## 代码组织

### processing/ 目录

```
src/processing/
├── mod.rs          # 统一导出所有公开函数
├── color.rs        # color_adjust, hsl_adjust, levels, color_balance, invert, threshold, lut_apply
├── filter.rs       # gaussian_blur, box_blur, sharpen, edge_detect_sobel, edge_detect_laplacian, emboss, denoise, pixelate, vignette, film_grain
├── transform.rs    # （使用 image crate 内置功能：resize, crop, rotate, flip）+ distort
├── composite.rs    # blend, mask_apply, channel_split, channel_merge
└── generate.rs     # solid_color, gradient, perlin_noise, checkerboard
```

**原则：**
- 所有函数只接受 `image` crate 类型（`DynamicImage`、`ImageBuffer` 等），不依赖 `node/` 中的任何类型
- 每个函数是纯函数：输入图像 + 参数 → 输出图像
- `builtins/xxx.rs` 中的 `process` 函数负责从 `Value` 提取参数并调用 `processing::` 函数

### builtins/ 新增文件

每个节点一个文件，严格按 catalog.md 规格注册 `NodeDef`：

```
src/node/builtins/
├── mod.rs              # 更新：注册所有新节点
├── load_image.rs       # 已有
├── save_image.rs       # 已有
├── color_adjust.rs     # 已有
├── preview.rs          # 已有
├── solid_color.rs      # 新增
├── gradient.rs         # 新增
├── noise.rs            # 新增
├── checkerboard.rs     # 新增
├── levels.rs           # 新增
├── hue_saturation.rs   # 新增
├── color_balance.rs    # 新增
├── invert.rs           # 新增
├── threshold.rs        # 新增
├── lut_apply.rs        # 新增（catalog 外，新增）
├── resize.rs           # 新增
├── crop.rs             # 新增
├── rotate.rs           # 新增
├── flip.rs             # 新增
├── distort.rs          # 新增
├── blur.rs             # 新增
├── sharpen.rs          # 新增
├── edge_detect.rs      # 新增
├── emboss.rs           # 新增
├── denoise.rs          # 新增
├── pixelate.rs         # 新增
├── vignette.rs         # 新增
├── film_grain.rs       # 新增（catalog 外，新增）
├── blend.rs            # 新增
├── mask.rs             # 新增
├── channel_merge.rs    # 新增
└── histogram.rs        # 新增
```

---

## 节点规格（补充 catalog.md 缺失的两个）

### Film Grain（电影噪点）

| 项目 | 内容 |
|------|------|
| 标题 | Film Grain |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | amount: Float + Range(0.0, 1.0), 默认 0.3; size: Float + Range(0.5, 3.0), 默认 1.0; seed: Int + Range(0, 99999), 默认 0 |
| 行为 | 添加有机胶片颗粒感噪点。与 Noise 生成节点不同，Film Grain 在已有图像上叠加颗粒，且噪点分布更接近真实胶片（高斯分布，亮部噪点更少） |

### LUT Apply（LUT 应用）

| 项目 | 内容 |
|------|------|
| 标题 | LUT Apply |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | path: String + FilePath(["cube"]), 默认无; intensity: Float + Range(0.0, 1.0), 默认 1.0 |
| 行为 | 加载 .cube 格式的 3D LUT 文件并应用色彩映射。intensity 参数控制原图与 LUT 结果的混合比例（0.0=原图，1.0=完全应用 LUT） |

---

## 算法实现要点

### 无新依赖

全部使用 `image` crate 现有功能 + 手写算法。不引入新的 crate。

### 关键算法说明

| 算法 | 实现方式 |
|------|---------|
| Gaussian Blur | 生成高斯核，可分离卷积（水平+垂直两趟） |
| USM Sharpen | 原图 + amount * (原图 - blur(原图)) |
| Sobel 边缘检测 | 3x3 Sobel 核水平+垂直卷积，取幅值 |
| Canny 边缘检测 | Gaussian blur → Sobel → 非极大值抑制 → 双阈值 |
| Laplacian 边缘检测 | 3x3 Laplacian 核卷积 |
| Emboss 浮雕 | 方向性卷积核 + 128 偏移 |
| Denoise 双边滤波 | 空间+色彩双权重滤波 |
| Distort | 逆映射 + 双线性插值 |
| Perlin Noise | 经典 Perlin 噪声算法 |
| HSL 转换 | RGB→HSL→调整→HSL→RGB |
| Levels 色阶 | 输入映射 → gamma 校正 → 输出映射 |
| Blend 混合模式 | 逐像素应用混合公式（Normal/Multiply/Screen/Overlay/Add/Subtract/Difference） |
| LUT .cube 解析 | 文本逐行解析，支持 TITLE/DOMAIN_MIN/DOMAIN_MAX/LUT_3D_SIZE，三线性插值查表 |
| Film Grain | 高斯随机噪声 + 亮度权重（暗部颗粒更明显） |
| Histogram | 统计 256 级灰度分布，渲染到 PreviewArea（需要特殊处理） |

---

## 文档更新

1. **architecture.md**：更新文件结构部分，将 `processing.rs` 改为 `processing/` 目录
2. **catalog.md**：添加 Film Grain 和 LUT Apply 的规格
