# 节点目录

> v1 全量节点规格——AI 推理、图像处理、工具节点的完整定义。

## 总览

约 86 个节点，覆盖从模型加载到最终输出的完整 AI 图像创作工作流，含云端模型 API 节点。参考 ComfyUI 核心节点的功能粒度，但重新设计命名和分类体系。

**设计原则：**

- **自适应优先**：一个节点适配所有模型架构（SD1.5/SDXL/SD3/Flux），根据连入的模型类型动态调整参数，避免模型专属节点膨胀
- **粒度对齐 ComfyUI**：保持与 ComfyUI 相同的节点拆分粒度，用户可无缝迁移工作流
- **分类清晰**：按功能域扁平分类，不超过两级

---

## 分类体系

| 分类 | 说明 | 执行器 |
|------|------|--------|
| 模型加载 | 加载各类模型文件到 VRAM | AI |
| 文本编码 | 文本 prompt → Conditioning | AI |
| 采样 | 扩散采样核心节点 | AI |
| 采样组件 | Sampler / Scheduler / Guider / Noise 构建块 | AI |
| 潜空间 | Latent 空间操作（创建、缩放、裁剪、合成） | AI |
| VAE | 图像 ↔ Latent 编解码 | AI |
| ControlNet | 结构控制条件 + 预处理 | AI / GPU |
| Conditioning 操作 | Conditioning 的组合、区域、时间步控制 | AI |
| 蒙版 | Mask 创建和操作 | GPU/CPU |
| 模型高级 | 模型合并、采样配置、加速技巧 | AI |
| 图像生成 | 从无到有生成图像（纯色、渐变、噪声等） | GPU |
| 图像调色 | 颜色、色阶、曲线、LUT | GPU |
| 图像变换 | 缩放、裁剪、旋转、翻转 | GPU |
| 图像滤镜 | 模糊、锐化、降噪、特效 | GPU |
| 图像合成 | 混合、蒙版合成、通道操作 | GPU |
| 模型 API | 云端大厂 API 一键生图/编辑（OpenAI、Stability AI、通义万象） | API |
| 数据 | 文件 I/O（加载/保存图像） | CPU |
| 工具 | 预览、分析 | CPU |

---

## 数据类型

### Rust 可理解类型

| 类型 | 说明 |
|------|------|
| Image | 像素图像（RGBA） |
| Mask | 单通道蒙版 |
| Float / Int / String / Bool / Color | 标量参数 |

### Python 专属类型（通过 Handle 引用）

| 类型 | 说明 |
|------|------|
| Model | Diffusion 模型（UNet/DiT） |
| Clip | 文本编码器模型 |
| Vae | VAE 编解码模型 |
| Conditioning | 条件化信息（text embedding + metadata） |
| Latent | 潜空间张量 |
| ControlNet | ControlNet 模型 |
| ClipVision | CLIP Vision 模型 |
| ClipVisionOutput | CLIP Vision 编码输出 |
| StyleModel | Style/IP-Adapter 模型 |
| UpscaleModel | 超分辨率模型 |
| Sampler | 采样算法对象 |
| Sigmas | Sigma 调度序列 |
| Noise | 噪声对象 |
| Guider | 引导器对象 |

---

## 模型加载

### CheckpointLoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | model(Model), clip(Clip), vae(Vae) |
| 参数 | checkpoint_path: FilePath(["safetensors", "ckpt"]) |
| 行为 | 加载完整 checkpoint，自适应 SD1.5/SDXL/SD3/Flux 等架构 |

### UNETLoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | model(Model) |
| 参数 | unet_path: FilePath(["safetensors"]), weight_dtype: Enum(default/fp8_e4m3fn/fp8_e5m2) |
| 行为 | 单独加载 diffusion model，支持 fp8 量化 |

### CLIPLoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | clip(Clip) |
| 参数 | clip_path: FilePath(["safetensors"]), clip_path2: FilePath(可选), type: Enum(sd1.5/sdxl/sd3/flux) |
| 行为 | 加载 CLIP 文本编码器。type 为 sdxl/sd3/flux 时启用 clip_path2 加载双/三 CLIP |

### VAELoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | vae(Vae) |
| 参数 | vae_path: FilePath(["safetensors", "pt"]) |
| 行为 | 单独加载 VAE 模型 |

### LoRALoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model), clip(Clip) |
| 输出引脚 | model(Model), clip(Clip) |
| 参数 | lora_path: FilePath(["safetensors"]), strength_model: Float(0.0~2.0, 默认1.0), strength_clip: Float(0.0~2.0, 默认1.0) |
| 行为 | 加载并应用 LoRA 到模型和 CLIP，可分别调节强度 |

### ControlNetLoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | control_net(ControlNet) |
| 参数 | controlnet_path: FilePath(["safetensors"]) |
| 行为 | 加载 ControlNet 模型 |

### CLIPVisionLoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | clip_vision(ClipVision) |
| 参数 | clip_vision_path: FilePath(["safetensors"]) |
| 行为 | 加载 CLIP Vision 模型 |

### StyleModelLoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | style_model(StyleModel) |
| 参数 | style_model_path: FilePath(["safetensors"]) |
| 行为 | 加载 Style/IP-Adapter 模型 |

### UpscaleModelLoader

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | upscale_model(UpscaleModel) |
| 参数 | model_path: FilePath(["safetensors", "pth"]) |
| 行为 | 加载超分辨率模型（ESRGAN、RealESRGAN 等） |

---

## 文本编码

### CLIPTextEncode

| 项目 | 内容 |
|------|------|
| 输入引脚 | clip(Clip) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | text: String(multiline)。SDXL 时自适应显示：text_g + text_l + width/height/crop_w/crop_h/target_width/target_height |
| 行为 | 通用文本编码，根据 CLIP 类型自适应参数。SD1.5 单 prompt，SDXL 双 prompt + 分辨率条件 |

### CLIPSetLastLayer

| 项目 | 内容 |
|------|------|
| 输入引脚 | clip(Clip) |
| 输出引脚 | clip(Clip) |
| 参数 | stop_at_clip_layer: Int(-24 ~ -1, 默认 -1) |
| 行为 | 设置 CLIP 使用哪一层输出（CLIP skip） |

### CLIPVisionEncode

| 项目 | 内容 |
|------|------|
| 输入引脚 | clip_vision(ClipVision), image(Image) |
| 输出引脚 | clip_vision_output(ClipVisionOutput) |
| 参数 | crop: Enum(center/none, 默认 center) |
| 行为 | 用 CLIP Vision 模型编码图像 |

---

## 采样

### KSampler

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model), positive(Conditioning), negative(Conditioning), latent_image(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | seed: Int, steps: Int(1~150, 默认20), cfg: Float(1.0~30.0, 默认7.0), sampler_name: Enum(euler/euler_ancestral/dpm_2/dpm_2_ancestral/lms/heun/ddim/uni_pc), scheduler: Enum(normal/karras/exponential/sgm_uniform), denoise: Float(0.0~1.0, 默认1.0) |
| 行为 | 核心采样节点，txt2img（denoise=1.0）和 img2img（denoise<1.0）通用 |

### KSamplerAdvanced

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model), positive(Conditioning), negative(Conditioning), latent_image(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | add_noise: Enum(enable/disable), noise_seed: Int, steps: Int, cfg: Float, sampler_name: Enum, scheduler: Enum, start_at_step: Int, end_at_step: Int, return_with_leftover_noise: Enum(disable/enable) |
| 行为 | 高级采样，支持分步采样、禁用噪声添加、保留残余噪声 |

### SamplerCustom

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model), positive(Conditioning), negative(Conditioning), sampler(Sampler), sigmas(Sigmas), latent_image(Latent) |
| 输出引脚 | latent(Latent), denoised_output(Latent) |
| 参数 | add_noise: Bool(默认true), noise_seed: Int, cfg: Float |
| 行为 | 自定义采样，接受独立的 Sampler 和 Sigmas 对象 |

### SamplerCustomAdvanced

| 项目 | 内容 |
|------|------|
| 输入引脚 | noise(Noise), guider(Guider), sampler(Sampler), sigmas(Sigmas), latent_image(Latent) |
| 输出引脚 | latent(Latent), denoised_output(Latent) |
| 参数 | 无 |
| 行为 | 最灵活的采样节点，全部组件外部传入 |

---

## 采样组件

### KSamplerSelect

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | sampler(Sampler) |
| 参数 | sampler_name: Enum(euler/euler_ancestral/dpm_2/dpm_2_ancestral/dpmpp_2m/dpmpp_2m_sde/dpmpp_3m_sde/lms/heun/ddim/uni_pc) |
| 行为 | 选择采样算法 |

### BasicScheduler

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model) |
| 输出引脚 | sigmas(Sigmas) |
| 参数 | scheduler: Enum(normal/karras/exponential/sgm_uniform), steps: Int, denoise: Float |
| 行为 | 通用调度器 |

### KarrasScheduler

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | sigmas(Sigmas) |
| 参数 | steps: Int, sigma_max: Float, sigma_min: Float |
| 行为 | Karras 噪声调度 |

### ExponentialScheduler

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | sigmas(Sigmas) |
| 参数 | steps: Int, sigma_max: Float, sigma_min: Float |
| 行为 | 指数噪声调度 |

### SDTurboScheduler

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model) |
| 输出引脚 | sigmas(Sigmas) |
| 参数 | steps: Int, denoise: Float |
| 行为 | SD Turbo 快速调度 |

### BasicGuider

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model), conditioning(Conditioning) |
| 输出引脚 | guider(Guider) |
| 参数 | 无 |
| 行为 | 基础引导器（无 CFG，单一 conditioning） |

### CFGGuider

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model), positive(Conditioning), negative(Conditioning) |
| 输出引脚 | guider(Guider) |
| 参数 | cfg: Float |
| 行为 | Classifier-Free Guidance 引导器 |

### DualCFGGuider

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model), cond1(Conditioning), cond2(Conditioning), negative(Conditioning) |
| 输出引脚 | guider(Guider) |
| 参数 | cfg_conds: Float, cfg_cond2_negative: Float |
| 行为 | 双重 CFG 引导器 |

### RandomNoise

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | noise(Noise) |
| 参数 | noise_seed: Int |
| 行为 | 产生随机噪声 |

### DisableNoise

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | noise(Noise) |
| 参数 | 无 |
| 行为 | 产生全零噪声（禁用随机噪声） |

---

## 潜空间

### EmptyLatentImage

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | latent(Latent) |
| 参数 | width: Int(64~4096, 默认1024), height: Int(64~4096, 默认1024), batch_size: Int(1~16, 默认1) |
| 行为 | 创建空白 latent 张量（用于 txt2img） |

### LatentUpscale

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | upscale_method: Enum(nearest-exact/bilinear/area/bicubic/bislerp), width: Int, height: Int, crop: Enum(disabled/center) |
| 行为 | 缩放 latent 到指定尺寸 |

### LatentUpscaleBy

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | upscale_method: Enum, scale_by: Float |
| 行为 | 按倍率缩放 latent |

### LatentComposite

| 项目 | 内容 |
|------|------|
| 输入引脚 | samples_to(Latent), samples_from(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | x: Int, y: Int, feather: Int |
| 行为 | 将一个 latent 叠加到另一个上（可羽化边缘） |

### LatentCompositeMasked

| 项目 | 内容 |
|------|------|
| 输入引脚 | destination(Latent), source(Latent), mask(Mask, 可选) |
| 输出引脚 | latent(Latent) |
| 参数 | x: Int, y: Int, resize_source: Bool |
| 行为 | 带蒙版的 latent 合成 |

### LatentCrop

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | width: Int, height: Int, x: Int, y: Int |
| 行为 | 裁剪 latent |

### LatentFlip

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | flip_method: Enum(x-axis/y-axis) |
| 行为 | 翻转 latent |

### LatentRotate

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | rotation: Enum(none/90/180/270) |
| 行为 | 旋转 latent |

### LatentFromBatch

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | batch_index: Int, length: Int |
| 行为 | 从 batch 中取出指定切片 |

### RepeatLatentBatch

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | amount: Int |
| 行为 | 重复 latent batch 指定次数 |

### LatentAdd

| 项目 | 内容 |
|------|------|
| 输入引脚 | samples1(Latent), samples2(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | 无 |
| 行为 | 两个 latent 逐元素相加 |

### LatentSubtract

| 项目 | 内容 |
|------|------|
| 输入引脚 | samples1(Latent), samples2(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | 无 |
| 行为 | 两个 latent 逐元素相减 |

### LatentMultiply

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | multiplier: Float |
| 行为 | latent 乘以标量 |

### LatentInterpolate

| 项目 | 内容 |
|------|------|
| 输入引脚 | samples1(Latent), samples2(Latent) |
| 输出引脚 | latent(Latent) |
| 参数 | ratio: Float(0.0~1.0) |
| 行为 | 球面插值两个 latent |

### SetLatentNoiseMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent), mask(Mask) |
| 输出引脚 | latent(Latent) |
| 参数 | 无 |
| 行为 | 为 latent 设置 inpaint noise mask |

---

## VAE

### VAEDecode

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent), vae(Vae) |
| 输出引脚 | image(Image) |
| 参数 | 无 |
| 行为 | 将 latent 解码为像素图像 |

### VAEEncode

| 项目 | 内容 |
|------|------|
| 输入引脚 | image(Image), vae(Vae) |
| 输出引脚 | latent(Latent) |
| 参数 | 无 |
| 行为 | 将像素图像编码为 latent |

### VAEDecodeTiled

| 项目 | 内容 |
|------|------|
| 输入引脚 | latent(Latent), vae(Vae) |
| 输出引脚 | image(Image) |
| 参数 | tile_size: Int(默认512), overlap: Int(默认64) |
| 行为 | 分块解码 latent（减少 VRAM 占用，适用于超大图像） |

### VAEEncodeTiled

| 项目 | 内容 |
|------|------|
| 输入引脚 | image(Image), vae(Vae) |
| 输出引脚 | latent(Latent) |
| 参数 | tile_size: Int(默认512), overlap: Int(默认64) |
| 行为 | 分块编码图像 |

### VAEEncodeForInpaint

| 项目 | 内容 |
|------|------|
| 输入引脚 | image(Image), vae(Vae), mask(Mask) |
| 输出引脚 | latent(Latent) |
| 参数 | grow_mask_by: Int(默认6) |
| 行为 | 为 inpaint 场景编码图像（masked 区域填充中灰，附带 noise_mask） |

---

## ControlNet

### ControlNetApply

| 项目 | 内容 |
|------|------|
| 输入引脚 | positive(Conditioning), negative(Conditioning), control_net(ControlNet), image(Image) |
| 输出引脚 | positive(Conditioning), negative(Conditioning) |
| 参数 | strength: Float(0.0~2.0, 默认1.0), start_percent: Float(0.0~1.0, 默认0.0), end_percent: Float(0.0~1.0, 默认1.0) |
| 行为 | 将 ControlNet 应用到 conditioning，支持强度和生效范围控制 |

### Canny

| 项目 | 内容 |
|------|------|
| 执行器 | **GPU**（图像处理执行器，非 AI 执行器） |
| 输入引脚 | image(Image) |
| 输出引脚 | image(Image) |
| 参数 | low_threshold: Float(0.01~0.99, 默认0.4), high_threshold: Float(0.01~0.99, 默认0.8) |
| 行为 | Canny 边缘检测（ControlNet 常用预处理）。分类在 ControlNet 下，但本质是 Image→Image 的图像算法，走 GPU shader 执行 |

---

## Conditioning 操作

### ConditioningCombine

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning_1(Conditioning), conditioning_2(Conditioning) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | 无 |
| 行为 | 并列组合两个 conditioning（非插值） |

### ConditioningAverage

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning_to(Conditioning), conditioning_from(Conditioning) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | conditioning_to_strength: Float(0.0~1.0) |
| 行为 | 按权重混合两个 conditioning |

### ConditioningConcat

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning_to(Conditioning), conditioning_from(Conditioning) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | 无 |
| 行为 | 在 token 维度上拼接两个 conditioning |

### ConditioningSetArea

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning(Conditioning) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | width: Int, height: Int, x: Int, y: Int, strength: Float |
| 行为 | 将 conditioning 限制到指定像素区域（区域提示词） |

### ConditioningSetMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning(Conditioning), mask(Mask) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | strength: Float, set_cond_area: Enum(default/mask bounds) |
| 行为 | 用蒙版限定 conditioning 的作用区域 |

### ConditioningSetTimestepRange

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning(Conditioning) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | start: Float(0.0~1.0), end: Float(0.0~1.0) |
| 行为 | 设置 conditioning 生效的时间步范围 |

### ConditioningZeroOut

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning(Conditioning) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | 无 |
| 行为 | 将 conditioning 张量清零（用于无条件引��等场景） |

### StyleModelApply

| 项目 | 内容 |
|------|------|
| 输入引脚 | conditioning(Conditioning), style_model(StyleModel), clip_vision_output(ClipVisionOutput) |
| 输出引脚 | conditioning(Conditioning) |
| 参数 | strength: Float(0.0~1.0, 默认1.0) |
| 行为 | 应用 Style Model / IP-Adapter 到 conditioning |

### InpaintModelConditioning

| 项目 | 内容 |
|------|------|
| 输入引脚 | positive(Conditioning), negative(Conditioning), vae(Vae), image(Image), mask(Mask) |
| 输出引脚 | positive(Conditioning), negative(Conditioning), latent(Latent) |
| 参数 | noise_mask: Bool(默认true) |
| 行为 | 为 inpaint 专用模型准备 conditioning |

---

## 蒙版

### SolidMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | 无 |
| 输出引脚 | mask(Mask) |
| 参数 | value: Float(0.0~1.0), width: Int, height: Int |
| 行为 | 创建纯色蒙版 |

### InvertMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | mask(Mask) |
| 输出引脚 | mask(Mask) |
| 参数 | 无 |
| 行为 | 反转蒙版 |

### GrowMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | mask(Mask) |
| 输出引脚 | mask(Mask) |
| 参数 | expand: Int(负值=收缩), tapered_corners: Bool |
| 行为 | 膨胀或腐蚀蒙版 |

### FeatherMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | mask(Mask) |
| 输出引脚 | mask(Mask) |
| 参数 | left: Int, top: Int, right: Int, bottom: Int |
| 行为 | 为蒙版边缘添加羽化效果 |

### ThresholdMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | mask(Mask) |
| 输出引脚 | mask(Mask) |
| 参数 | value: Float(0.0~1.0) |
| 行为 | 二值化蒙版 |

### CropMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | mask(Mask) |
| 输出引脚 | mask(Mask) |
| 参数 | x: Int, y: Int, width: Int, height: Int |
| 行为 | 裁剪蒙版 |

### MaskComposite

| 项目 | 内容 |
|------|------|
| 输入引脚 | destination(Mask), source(Mask) |
| 输出引脚 | mask(Mask) |
| 参数 | x: Int, y: Int, operation: Enum(multiply/add/subtract/and/or/xor) |
| 行为 | 蒙版合成 |

### MaskToImage

| 项目 | 内容 |
|------|------|
| 输入引脚 | mask(Mask) |
| 输出引脚 | image(Image) |
| 参数 | 无 |
| 行为 | 将蒙版转换为灰度图像 |

### ImageToMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | image(Image) |
| 输出引脚 | mask(Mask) |
| 参数 | channel: Enum(red/green/blue/alpha) |
| 行为 | 提取图像指定通道为蒙版 |

### ImageColorToMask

| 项目 | 内容 |
|------|------|
| 输入引脚 | image(Image) |
| 输出引脚 | mask(Mask) |
| 参数 | color: Color |
| 行为 | 将指定颜色转换为蒙版（色键） |

---

## 模型高级

### ModelMergeSimple

| 项目 | 内容 |
|------|------|
| 输入引脚 | model1(Model), model2(Model) |
| 输出引脚 | model(Model) |
| 参数 | ratio: Float(0.0~1.0) |
| 行为 | 按比例融合两个模型 |

### ModelMergeSubtract

| 项目 | 内容 |
|------|------|
| 输入引脚 | model1(Model), model2(Model) |
| 输出引脚 | model(Model) |
| 参数 | multiplier: Float |
| 行为 | 模型差分（model1 - model2 * multiplier） |

### CLIPMergeSimple

| 项目 | 内容 |
|------|------|
| 输入引脚 | clip1(Clip), clip2(Clip) |
| 输出引脚 | clip(Clip) |
| 参数 | ratio: Float(0.0~1.0) |
| 行为 | 按比例融合两个 CLIP 模型 |

### ModelSamplingDiscrete

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model) |
| 输出引脚 | model(Model) |
| 参数 | sampling: Enum(eps/v_prediction/lcm/x0), zsnr: Bool |
| 行为 | 设置模型的离散采样方式 |

### ModelSamplingFlux

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model) |
| 输出引脚 | model(Model) |
| 参数 | max_shift: Float, base_shift: Float, width: Int, height: Int |
| 行为 | Flux 模型采样配置 |

### RescaleCFG

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model) |
| 输出引脚 | model(Model) |
| 参数 | multiplier: Float(0.0~1.0) |
| 行为 | CFG 缩放修正 |

### FreeU

| 项目 | 内容 |
|------|------|
| 输入引脚 | model(Model) |
| 输出引脚 | model(Model) |
| 参数 | b1: Float, b2: Float, s1: Float, s2: Float |
| 行为 | 应用 FreeU 方法提升质量 |

### ImageUpscaleWithModel

| 项目 | 内容 |
|------|------|
| 输入引脚 | upscale_model(UpscaleModel), image(Image) |
| 输出引脚 | image(Image) |
| 参数 | 无 |
| 行为 | 使用超分辨率模型放大图像 |

---

## 图像生成

| 节点 | 输出 | 参数 | 行为 |
|------|------|------|------|
| SolidColor | image | color, width, height | 纯色图像 |
| Gradient | image | color_start, color_end, direction, width, height | 渐变图像 |
| Noise | image | seed, scale, width, height | Perlin 噪声图像 |
| Checkerboard | image | color_a, color_b, size, width, height | 棋盘格图案 |

（规格与现有 `catalog.md` 一致，不重复展开。）

---

## 图像调色

| 节点 | 输入 | 输出 | 参数 | 行为 |
|------|------|------|------|------|
| ColorAdjustment | image | image | brightness, saturation, contrast | 亮度/饱和度/对比度 |
| Curves | image | image | channel | 曲线调色 |
| Levels | image | image | in_black, in_white, gamma, out_black, out_white | 色阶 |
| HueSaturation | image | image | hue, saturation, lightness | HSL 调整 |
| ColorBalance | image | image | shadows_rgb, midtones_rgb, highlights_rgb | 三区色彩平衡 |
| Invert | image | image | — | 反色 |
| Threshold | image | image | threshold | 二值化 |
| LUTApply | image | image | path, intensity | 3D LUT 应用 |

（规格与现有 `catalog.md` 一致。）

---

## 图像变换

| 节点 | 输入 | 输出 | 参数 | 行为 |
|------|------|------|------|------|
| Resize | image | image | width, height, method | 缩放 |
| Crop | image | image | x, y, width, height | 裁剪 |
| Rotate | image | image | angle, fill | 旋转 |
| Flip | image | image | direction | 翻转 |
| Distort | image | image | type, strength | 几何扭曲 |
| ImagePadForOutpaint | image | image, mask | left, top, right, bottom, feathering | 扩展画布 + 生成 outpaint 蒙版 |

（前 5 个与现有 `catalog.md` 一致，ImagePadForOutpaint 为新增。）

---

## 图像滤镜

| 节点 | 输入 | 输出 | 参数 | 行为 |
|------|------|------|------|------|
| Blur | image | image | radius, method | 模糊 |
| Sharpen | image | image | amount, radius | 锐化 |
| EdgeDetect | image | image | method | 边缘检测 |
| Emboss | image | image | strength, angle | 浮雕 |
| Denoise | image | image | strength | 降噪 |
| Pixelate | image | image | block_size | 像素化 |
| Vignette | image | image | strength, radius | 暗角 |
| FilmGrain | image | image | amount, size, seed | 胶片颗粒 |

（规格与现有 `catalog.md` 一致。）

---

## 图像合成

| 节点 | 输入 | 输出 | 参数 | 行为 |
|------|------|------|------|------|
| Blend | base, layer | image | mode, opacity | 混合模式合成 |
| Mask | image, mask | image | invert | 蒙版控制透明度 |
| ChannelMerge | red?, green?, blue?, alpha? | image | — | 通道合并 |
| ImageCompositeMasked | destination, source, mask? | image | x, y, resize_source | 带蒙版的图像合成 |

（前 3 个与现有 `catalog.md` 一致，ImageCompositeMasked 为新增。）

---

## 模型 API

云端大厂推理 API 节点。每个节点封装一次完整的 API 调用（无 Handle、无中间状态），通过模型 API 执行器路由到对应的 `ApiProvider` 实现。所有节点 `executor: API`。

**与 AI 节点的核心区别：** AI 节点拆分为 LoadCheckpoint → KSampler → VAEDecode 等多步，Handle 跨节点复用；API 节点单节点完成整个生成，输入文本/图像，输出图像，无需用户理解模型内部流程。

**框架设计：**

节点通过 `provider` 参数选择厂商，执行时由 `ApiProvider` trait 分发（见 `03-executors.md` D11）。新增厂商只需实现 `ApiProvider`，再在节点的 `provider` 枚举中加一项，节点本身不变。

```
节点定义（node! 宏, executor: API）
  │
  ├─ provider 参数 → 选择 ApiProvider 实现
  │    ├─ OpenAiProvider     （API Key 来自配置）
  │    ├─ StabilityProvider
  │    └─ TongyiProvider
  │
  └─ ApiProvider trait
       ├─ authenticate()     → 从 ProviderConfig 构建认证 Client
       ├─ execute()          → 发起 HTTP 请求，返回 Value::Image
       └─ rate_limit_status() → 速率限制状态，供 EvalEngine 退避
```

**配置扩展（`10-config.md`）：** 各 provider 的 API Key 和 endpoint 在配置文件中按厂商分段：

```toml
[api.openai]
api_key = "sk-..."
# endpoint 可选覆盖，默认为官方地址

[api.stability]
api_key = "sk-..."

[api.tongyi]
api_key = "sk-..."
```

**v1 仅实现一个节点**，验证完整的 API 执行器框架（认证、速率限制、重试、超时）。后续按需扩展 img2img、inpaint、upscale 等变体。

### APIGenerate

| 项目 | 内容 |
|------|------|
| 输入引脚 | image(Image, 可选) |
| 输出引脚 | image(Image) |
| 参数 | provider: Enum(openai/stability/tongyi), prompt: String(multiline), negative_prompt: String(multiline, 可选), size: Enum(1024x1024/1024x1792/1792x1024, 默认 1024x1024), seed: Int(可选), strength: Float(0.0~1.0, 默认 0.7) |
| 行为 | 云端文生图 / 图生图。无 image 输入时为文生图，有 image 输入时为图生图（strength 控制重绘强度）。参数由 `ApiProvider` 映射为各厂商 API 的对应字段，厂商不支持的参数静默忽略 |

**参数映射示例：**

| 参数 | OpenAI | Stability AI | 通义万象 |
|------|--------|-------------|---------|
| prompt | prompt | text_prompts[0].text | prompt |
| negative_prompt | — (不支持) | text_prompts[1].text (weight: -1) | negative_prompt |
| size | size | aspect_ratio (换算) | size |
| seed | — (不支持) | seed | seed |
| strength | — (不支持, 仅 edit 接口) | image_strength | strength |

**扩展路线（v2+）：**

| 节点 | 功能 | 说明 |
|------|------|------|
| APIInpaint | 局部重绘 | 增加 mask 输入引脚 |
| APIUpscale | 超分辨率 | 仅接受 image 输入，provider 限 Stability |
| APIRemoveBg | 背景移除 | 输出 image + mask |

---

## 数据

| 节点 | 输入 | 输出 | 参数 | 行为 |
|------|------|------|------|------|
| LoadImage | — | image, mask | path: FilePath | 加载图像文件，提取 alpha 作为 mask |
| SaveImage | image | — | path: FilePath | 保存图像 |

（LoadImage 输出增加 mask，对齐 ComfyUI。其余与现有一致。）

---

## 工具

| 节点 | 输入 | 输出 | 参数 | 行为 |
|------|------|------|------|------|
| Preview | image | — | — | 在节点内显示缩略图 + 更新预览面板 |
| Histogram | image | — | channel | 绘制色彩分布直方图 |
