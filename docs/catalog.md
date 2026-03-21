# 节点目录

所有节点的分类定义和完整规格。每个节点通过 NodeRegistry 注册，此文档是注册内容的参考。

---

## 分类定义

| 分类标识 | 显示名称 | 说明 |
|---------|---------|------|
| data | 数据型 | 外部数据的输入输出 |
| generate | 生成型 | 从无到有生成图像 |
| color | 颜色处理型 | 调整颜色属性 |
| transform | 空间变换型 | 改变图像的几何形状和位置 |
| filter | 滤镜型 | 像素级效果处理 |
| composite | 合成型 | 多图像合并 |
| tool | 工具型 | 辅助功能（预览、统计等） |
| ai | AI 型 | AI 推理节点（通过计算后端执行） |

---

## 数据型

### Load Image

| 项目 | 内容 |
|------|------|
| 标题 | Load Image |
| 分类 | data |
| 输入引脚 | 无 |
| 输出引脚 | image (Image) |
| 参数 | path: String + FilePath(["png","jpg","jpeg","bmp","webp"]), 默认无 |
| 行为 | 加载本地图片文件，解码为 RGBA 图像输出 |

### Save Image

| 项目 | 内容 |
|------|------|
| 标题 | Save Image |
| 分类 | data |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | 无 |
| 参数 | path: String + FilePath(["png","jpg","bmp","webp"]), 默认无 |
| 行为 | 将输入图像编码并保存到指定路径 |

---

## 生成型

### Solid Color

| 项目 | 内容 |
|------|------|
| 标题 | Solid Color |
| 分类 | generate |
| 输入引脚 | 无 |
| 输出引脚 | image (Image) |
| 参数 | color: Color + 无约束, 默认白色; width: Int + Range(1, 8192), 默认 512; height: Int + Range(1, 8192), 默认 512 |
| 行为 | 生成指定尺寸的纯色图像 |

### Gradient

| 项目 | 内容 |
|------|------|
| 标题 | Gradient |
| 分类 | generate |
| 输入引脚 | 无 |
| 输出引脚 | image (Image) |
| 参数 | color_start: Color + 无约束, 默认黑色; color_end: Color + 无约束, 默认白色; direction: String + Enum(["horizontal","vertical","diagonal"]), 默认 horizontal; width: Int + Range(1, 8192), 默认 512; height: Int + Range(1, 8192), 默认 512 |
| 行为 | 生成两色渐变图像 |

### Noise

| 项目 | 内容 |
|------|------|
| 标题 | Noise |
| 分类 | generate |
| 输入引脚 | 无 |
| 输出引脚 | image (Image) |
| 参数 | seed: Int + Range(0, 99999), 默认 0; scale: Float + Range(0.01, 100.0), 默认 1.0; width: Int + Range(1, 8192), 默认 512; height: Int + Range(1, 8192), 默认 512 |
| 行为 | 生成 Perlin 噪声图像 |

### Checkerboard

| 项目 | 内容 |
|------|------|
| 标题 | Checkerboard |
| 分类 | generate |
| 输入引脚 | 无 |
| 输出引脚 | image (Image) |
| 参数 | color_a: Color + 无约束, 默认白色; color_b: Color + 无约束, 默认黑色; size: Int + Range(1, 512), 默认 32; width: Int + Range(1, 8192), 默认 512; height: Int + Range(1, 8192), 默认 512 |
| 行为 | 生成棋盘格图案 |

---

## 颜色处理型

### Color Adjustment

| 项目 | 内容 |
|------|------|
| 标题 | Color Adjustment |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | brightness: Float + Range(-1.0, 1.0), 默认 0.0; saturation: Float + Range(-1.0, 1.0), 默认 0.0; contrast: Float + Range(-1.0, 1.0), 默认 0.0 |
| 行为 | 调整亮度、饱和度、对比度 |

### Curves

| 项目 | 内容 |
|------|------|
| 标题 | Curves |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | channel: String + Enum(["rgb","red","green","blue"]), 默认 rgb |
| 行为 | 通过曲线调整色调映射 |
| 备注 | **v2** — 需要扩展 Curve 约束和 CurveEditor 控件后实现 |

### Levels

| 项目 | 内容 |
|------|------|
| 标题 | Levels |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | in_black: Float + Range(0.0, 1.0), 默认 0.0; in_white: Float + Range(0.0, 1.0), 默认 1.0; gamma: Float + Range(0.1, 10.0), 默认 1.0; out_black: Float + Range(0.0, 1.0), 默认 0.0; out_white: Float + Range(0.0, 1.0), 默认 1.0 |
| 行为 | 调整输入/输出色阶范围和 gamma |

### Hue/Saturation

| 项目 | 内容 |
|------|------|
| 标题 | Hue/Saturation |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | hue: Float + Range(-180.0, 180.0), 默认 0.0; saturation: Float + Range(-1.0, 1.0), 默认 0.0; lightness: Float + Range(-1.0, 1.0), 默认 0.0 |
| 行为 | 在 HSL 空间调整色相、饱和度、明度 |

### Color Balance

| 项目 | 内容 |
|------|------|
| 标题 | Color Balance |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | shadows_r: Float + Range(-1.0, 1.0), 默认 0.0; shadows_g: Float + Range(-1.0, 1.0), 默认 0.0; shadows_b: Float + Range(-1.0, 1.0), 默认 0.0; midtones_r: Float + Range(-1.0, 1.0), 默认 0.0; midtones_g: Float + Range(-1.0, 1.0), 默认 0.0; midtones_b: Float + Range(-1.0, 1.0), 默认 0.0; highlights_r: Float + Range(-1.0, 1.0), 默认 0.0; highlights_g: Float + Range(-1.0, 1.0), 默认 0.0; highlights_b: Float + Range(-1.0, 1.0), 默认 0.0 |
| 行为 | 分别调整暗部、中间调、高光的 RGB 偏移 |

### Invert

| 项目 | 内容 |
|------|------|
| 标题 | Invert |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | 无 |
| 行为 | 反转所有像素的 RGB 值（255 - value） |

### Threshold

| 项目 | 内容 |
|------|------|
| 标题 | Threshold |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | threshold: Float + Range(0.0, 1.0), 默认 0.5 |
| 行为 | 亮度高于阈值的像素变白，低于的变黑 |

### LUT Apply

| 项目 | 内容 |
|------|------|
| 标题 | LUT Apply |
| 分类 | color |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | path: String + FilePath(["cube"]), 默认无; intensity: Float + Range(0.0, 1.0), 默认 1.0 |
| 行为 | 加载 .cube 格式的 3D LUT 文件并应用色彩映射。intensity 控制原图与 LUT 结果的混合比例（0.0=原图，1.0=完全应用） |

---

## 空间变换型

### Resize

| 项目 | 内容 |
|------|------|
| 标题 | Resize |
| 分类 | transform |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | width: Int + Range(1, 8192), 默认 512; height: Int + Range(1, 8192), 默认 512; method: String + Enum(["nearest","bilinear","bicubic","lanczos3"]), 默认 bilinear |
| 行为 | 缩放图像到指定尺寸 |

### Crop

| 项目 | 内容 |
|------|------|
| 标题 | Crop |
| 分类 | transform |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | x: Int + Range(0, 8192), 默认 0; y: Int + Range(0, 8192), 默认 0; width: Int + Range(1, 8192), 默认 256; height: Int + Range(1, 8192), 默认 256 |
| 行为 | 裁剪图像的指定区域 |

### Rotate

| 项目 | 内容 |
|------|------|
| 标题 | Rotate |
| 分类 | transform |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | angle: Float + Range(-360.0, 360.0), 默认 0.0; fill: Color + 无约束, 默认透明 |
| 行为 | 旋转图像，空白区域填充指定颜色 |

### Flip

| 项目 | 内容 |
|------|------|
| 标题 | Flip |
| 分类 | transform |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | direction: String + Enum(["horizontal","vertical","both"]), 默认 horizontal |
| 行为 | 水平或垂直翻转图像 |

### Distort

| 项目 | 内容 |
|------|------|
| 标题 | Distort |
| 分类 | transform |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | type: String + Enum(["barrel","pincushion","swirl"]), 默认 barrel; strength: Float + Range(-2.0, 2.0), 默认 0.5 |
| 行为 | 对图像施加几何扭曲变形 |

---

## 滤镜型

### Blur

| 项目 | 内容 |
|------|------|
| 标题 | Blur |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | radius: Float + Range(0.0, 100.0), 默认 5.0; method: String + Enum(["gaussian","box"]), 默认 gaussian |
| 行为 | 对图像进行模糊处理 |

### Sharpen

| 项目 | 内容 |
|------|------|
| 标题 | Sharpen |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | amount: Float + Range(0.0, 10.0), 默认 1.0; radius: Float + Range(0.0, 50.0), 默认 1.0 |
| 行为 | 锐化图像边缘细节 |

### Edge Detect

| 项目 | 内容 |
|------|------|
| 标题 | Edge Detect |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | method: String + Enum(["sobel","canny","laplacian"]), 默认 sobel |
| 行为 | 提取图像边缘 |

### Emboss

| 项目 | 内容 |
|------|------|
| 标题 | Emboss |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | strength: Float + Range(0.0, 5.0), 默认 1.0; angle: Float + Range(0.0, 360.0), 默认 135.0 |
| 行为 | 生成浮雕效果 |

### Denoise

| 项目 | 内容 |
|------|------|
| 标题 | Denoise |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | strength: Float + Range(0.0, 30.0), 默认 10.0 |
| 行为 | 降低图像噪点，保留边缘 |

### Pixelate

| 项目 | 内容 |
|------|------|
| 标题 | Pixelate |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | block_size: Int + Range(1, 256), 默认 8 |
| 行为 | 像素化效果，将区域内像素合并为一个颜色 |

### Vignette

| 项目 | 内容 |
|------|------|
| 标题 | Vignette |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | strength: Float + Range(0.0, 2.0), 默认 0.5; radius: Float + Range(0.0, 2.0), 默认 1.0 |
| 行为 | 边缘暗角效果 |

### Film Grain

| 项目 | 内容 |
|------|------|
| 标题 | Film Grain |
| 分类 | filter |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | amount: Float + Range(0.0, 1.0), 默认 0.3; size: Float + Range(0.5, 3.0), 默认 1.0; seed: Int + Range(0, 99999), 默认 0 |
| 行为 | 在已有图像上叠加有机胶片颗粒感噪点（与 Noise 生成节点不同，Film Grain 不生成图像而是在输入图像上叠加）。噪点分布为高斯分布，亮部颗粒更少，暗部更明显，模拟真实胶片效果 |

---

## 合成型

### Blend

| 项目 | 内容 |
|------|------|
| 标题 | Blend |
| 分类 | composite |
| 输入引脚 | base (Image, 必需); layer (Image, 必需) |
| 输出引脚 | image (Image) |
| 参数 | mode: String + Enum(["normal","multiply","screen","overlay","add","subtract","difference"]), 默认 normal; opacity: Float + Range(0.0, 1.0), 默认 1.0 |
| 行为 | 将两张图像按混合模式合成 |

### Mask

| 项目 | 内容 |
|------|------|
| 标题 | Mask |
| 分类 | composite |
| 输入引脚 | image (Image, 必需); mask (Mask, 必需) |
| 输出引脚 | image (Image) |
| 参数 | invert: Boolean + 无约束, 默认 false |
| 行为 | 用遮罩控制图像的透明度，白色区域保留，黑色区域透明 |

### Channel Merge

| 项目 | 内容 |
|------|------|
| 标题 | Channel Merge |
| 分类 | composite |
| 输入引脚 | red (Mask, 可选); green (Mask, 可选); blue (Mask, 可选); alpha (Mask, 可选) |
| 输出引脚 | image (Image) |
| 参数 | 无 |
| 行为 | 将独立的灰度通道合并为一张 RGBA 图像，未连线的通道默认填充白色（alpha 默认 255） |

---

## 工具型

### Preview

| 项目 | 内容 |
|------|------|
| 标题 | Preview |
| 分类 | tool |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | 无 |
| 参数 | 无 |
| 行为 | 在节点主体内显示缩略图，同时更新左侧预览面板 |

### Histogram

| 项目 | 内容 |
|------|------|
| 标题 | Histogram |
| 分类 | tool |
| 输入引脚 | image (Image, 必需) |
| 输出引脚 | 无 |
| 参数 | channel: String + Enum(["rgb","red","green","blue","luminance"]), 默认 rgb |
| 行为 | 在节点主体内绘制图像的色彩分布直方图 |

---

## AI 型

AI 型节点通过 BackendClient 与计算后端通信。前端不执行推理，只传递句柄 ID 和参数。

### Load Checkpoint

| 项目 | 内容 |
|------|------|
| 标题 | Load Checkpoint |
| 分类 | ai |
| 输入引脚 | 无 |
| 输出引脚 | model (Model), clip (Clip), vae (Vae) |
| 参数 | checkpoint_path: String + FilePath(["safetensors","ckpt"]), 默认无 |
| 行为 | 请求后端加载 SDXL 模型文件，返回 model/clip/vae 三个句柄 |
| 端点 | POST /load_checkpoint |

### CLIP Text Encode

| 项目 | 内容 |
|------|------|
| 标题 | CLIP Text Encode |
| 分类 | ai |
| 输入引脚 | clip (Clip, 必需) |
| 输出引脚 | conditioning (Conditioning) |
| 参数 | text: String + 无约束, 默认空 |
| 行为 | 请求后端用 CLIP 编码文本，返回 conditioning 句柄 |
| 端点 | POST /clip_encode |

### Empty Latent Image

| 项目 | 内容 |
|------|------|
| 标题 | Empty Latent Image |
| 分类 | ai |
| 输入引脚 | 无 |
| 输出引脚 | latent (Latent) |
| 参数 | width: Int + Range(64, 4096), 默认 1024; height: Int + Range(64, 4096), 默认 1024; batch_size: Int + Range(1, 16), 默认 1 |
| 行为 | 请求后端创建空白隐空��张量，返回 latent 句柄 |
| 端点 | POST /empty_latent |

### KSampler

| 项目 | 内容 |
|------|------|
| 标题 | KSampler |
| 分类 | ai |
| 输入引脚 | model (Model, 必需), positive (Conditioning, 必需), negative (Conditioning, 必需), latent (Latent, 必需) |
| 输出引脚 | latent (Latent) |
| 参数 | seed: Int + Range(0, 2147483647), 默认 0; steps: Int + Range(1, 150), 默认 20; cfg: Float + Range(1.0, 30.0), 默认 7.0; sampler_name: String + Enum(["euler","euler_ancestral","dpm_2","dpm_2_ancestral","lms","heun","ddim","uni_pc"]), 默认 euler; scheduler: String + Enum(["normal","karras","exponential","sgm_uniform"]), 默认 normal |
| 行为 | 请求后端执行扩散采样，返回采样后的 latent 句柄 |
| 端点 | POST /ksample |

### VAE Decode

| 项目 | 内容 |
|------|------|
| 标题 | VAE Decode |
| 分类 | ai |
| 输入引脚 | vae (Vae, 必需), latent (Latent, 必需) |
| 输出引脚 | image (Image) |
| 参数 | 无 |
| 行为 | 请求后端用 VAE 解码 latent 为图像，返回 base64 PNG，前端解码为 Image |
| 端点 | POST /vae_decode |
