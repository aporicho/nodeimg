当前工作区：
/home/aporicho/nodeimg/.claude/worktrees/ui-engine-integration

任务目标：
先制定一份完整的 affine transform rendering 设计和实施计划，不要继续 scale policy，不要直接写代码。我要的是能指导后续实现的完整工程方案：接口清晰、模块化、文件结构明确、测试覆盖明确、不留下代码债务。

重要要求：
- 先读代码和现有文档，再制定计划。
- 不要实现代码。
- 不要继续做 ScaleBehavior / scale policy 方向。
- affine transform rendering 是本次主题：完整设计 paint、renderer、prepare、GPU pipeline、hit test、clip、recording tests 的统一 transform 方案。
- 计划必须能落地到当前代码库，不要停留在抽象架构描述。
- 计划要包含接口化、模块化、文件结构、迁移步骤、验收标准、测试策略。
- 允许并建议使用逻辑推理 MCP 多轮审查计划。请在输出最终计划前，用 sequential thinking / 逻辑推理工具做至少三轮自查：
  1. 架构一致性审查
  2. 渲染管线完整性审查
  3. 代码债务和可实施性审查
- 自查结果不需要暴露完整思维链，但最终计划里要体现审查后的修正结论和风险处理。

当前背景：
- 当前 Transform 类型已有 translate、scale、rotate，但 rotate 目前是 reserved/inert。
- 当前 paint/hit 只支持 translate + uniform scale。
- 之前 scale policy 方向先暂停，不要继续扩展 world-space vs screen-space sizing。
- 现在要先把完整 affine transform rendering 的设计做扎实，避免后续 rotate 支持变成到处打补丁。

请重点阅读这些文件：
- docs/todo/2026-04-23-gui-primitive-rendering-gaps.md
- docs/superpowers/specs/2026-04-11-phase-c5-paint-extensions-design.md
- gui/src/tree/layout/types.rs
- gui/src/tree/paint_helpers.rs
- gui/src/tree/paint.rs
- gui/src/tree/paint_target.rs
- gui/src/tree/paint_ops.rs
- gui/src/tree/hit.rs
- gui/src/renderer/types.rs
- gui/src/renderer/command.rs
- gui/src/renderer/renderer.rs
- gui/src/renderer/prepare.rs
- gui/src/renderer/path.rs
- gui/src/renderer/vector_tessellator.rs
- gui/src/renderer/pipeline/quad.rs
- gui/src/renderer/pipeline/circle.rs
- gui/src/renderer/pipeline/image.rs
- gui/src/renderer/pipeline/text.rs
- gui/src/renderer/pipeline/shadow.rs
- gui/src/renderer/pipeline/stencil.rs
- gui/src/renderer/svg/vector.rs
- gui/src/renderer/svg/raster.rs
- gui/src/renderer/shaders/quad.wgsl
- gui/src/renderer/shaders/vector.wgsl
- gui/src/renderer/shaders/circle.wgsl
- gui/src/renderer/shaders/image.wgsl
- gui/src/renderer/shaders/stencil.wgsl
- app/src/workspace/view.rs
- gui/src/canvas/camera.rs

当前代码状态概述：
- gui/src/tree/layout/types.rs 的 Transform：
  - translate: [f32; 2]
  - scale: f32
  - rotate: f32
- gui/src/tree/paint_helpers.rs 当前 PaintTransform 只有：
  - tx
  - ty
  - scale
- gui/src/tree/hit.rs 当前 inverse_supported_transform 只处理 translate + scale，rotate 被忽略。
- gui/src/tree/paint.rs 当前在 paint 阶段把 rect/path/circle/text/image 转成 screen-space 几何。
- renderer 的 DrawCommand 目前大多接收已经是 screen-space 的 Rect / Point / PathData。
- renderer prepare 阶段会 CPU tessellate quad/vector/circle/stencil。
- image shader 当前基于 axis-aligned rect 生成 6 个顶点。
- clip 当前是 PushClip { rect, radius }，使用 stencil tessellate axis-aligned rounded rect。
- text 当前走 glyphon，基本是水平文本布局。
- SVG vector 当前解析成 PathRequest，SVG raster fallback 当前画成 rect image。

需要解决的问题：
完整 affine transform rendering 不是只把 PaintTransform 改成矩阵。它必须定义从 tree 到 renderer 到 GPU 的统一 transform 语义，并覆盖：
- Rect
- Rounded rect
- Border
- Shadow
- Circle
- Path
- Image
- Text
- Clip / stencil
- SVG vector
- SVG raster fallback
- CustomPaint
- PaintOp recording
- Hit test inverse transform
- batching / cache / performance
- workspace canvas camera 的兼容性

请输出一份完整设计计划，结构必须包含以下部分：

1. 现状分析
说明当前 paint -> PaintTarget -> Renderer -> DrawCommand -> prepare -> pipeline 的路径。
说明 transform 当前在哪一层应用。
说明为什么当前不能直接启用 rotate。
说明哪些 renderer pipeline 假设了 axis-aligned rect。

2. 设计目标
明确 affine transform rendering 的目标。
明确本阶段必须消除什么代码债务。
明确不做什么：
- 不做 scale policy。
- 不做 screen-space fixed affordance。
- 不做 unrelated layout 重构。
- 不做 shape-aware hit testing，除非作为后续边界说明。

3. 核心数学设计
设计 Affine2D / Transform2D 类型。
要求说明：
- 类型放在哪个文件。
- 字段布局。
- identity。
- translation。
- scale。
- rotation。
- compose。
- inverse。
- transform_point。
- transform_rect_corners。
- transform_path。
- approx_uniform_scale 或 effective_scale 的定义。
- 是否支持 non-uniform / skew。
- 如果不开放 non-uniform / skew，也要让内部结构未来可扩展。
- 矩阵乘法顺序必须明确。
- 坐标系必须明确。
- rotate 角度单位必须明确。

4. Transform 语义设计
说明 tree Transform 如何转换为 Affine2D。
说明父子 transform 如何组合。
说明 node.rect、child rect、leaf local path 的坐标关系。
说明 transform origin 策略：
- 当前是否默认以 node rect origin 为 transform origin。
- 是否需要 transform_origin。
- 如果暂不引入 transform_origin，要说明原因和未来扩展点。
说明 paint 和 hit 必须共用同一套 affine 语义。

5. 模块化和文件结构
提出推荐文件结构，至少覆盖：
- gui/src/geometry/ 或 gui/src/renderer/geometry/ 是否应新增模块。
- Affine2D 放在哪里。
- renderer transform helper 放在哪里。
- tree paint transform adapter 放在哪里。
- hit transform adapter 放在哪里。
- 测试 helper 放在哪里。
说明为什么这样分层，避免 tree 反向依赖 renderer 内部 pipeline 细节。

6. Renderer API 设计
设计 DrawCommand 是否携带 transform，还是 paint 阶段预变换。
必须比较两种方案：
A. paint 阶段预变换所有几何到 screen-space。
B. DrawCommand 携带 affine，由 prepare / pipeline 处理。
给出推荐方案。
推荐方案必须考虑：
- path tessellation cache 是否会被 screen-space transform 破坏。
- image 是否需要 affine quad instance。
- clip 是否需要 transformed stencil path。
- text 是否能支持 rotate。
- shadow 缓存如何处理 rotate。
- batching 是否还能工作。
- PaintOp recording 是否能清楚表达 transform。

7. Primitive 设计细节
逐项给出方案，不要只列标题。

Rect / Rounded Rect：
- axis-aligned rect 如何升级为 transformed rounded rect path。
- border 如何处理。
- radius 在 rotate 下如何处理。
- non-uniform scale 如果暂不支持，如何限制。

Path / Vector：
- PathData 是否保持 local-space。
- PathRequest 是否新增 transform。
- tessellation 在 local-space 还是 transformed-space。
- stroke width 如何处理。
- cache key 如何设计。

Circle：
- rotate 下圆不变。
- uniform scale 下半径缩放。
- non-uniform 下是否变 ellipse，若不支持要明确限制。

Image：
- image pipeline 从 rect instance 改为 affine quad / four corners。
- uv_rect 如何保留。
- contain / cover 在 local rect 下先 resolve 还是 screen-space 后 resolve。
- SVG raster fallback 如何复用 image affine quad。

Text：
- glyphon 对 rotated text 的限制。
- v1 如何处理 rotated text，必须明确策略：
  - 禁止/忽略 rotation？
  - fallback 为 unrotated？
  - 转成 texture 再 affine draw？
  - 延后但必须有显式 guard 和测试？
- 不允许悄悄行为不一致。

Clip / Stencil：
- PushClip 从 rect/radius 升级为 clip shape 或 transformed rounded rect。
- nested clip 如何保持 stencil depth。
- hit overflow clip 如何跟 paint clip 保持一致。

Shadow：
- 当前 shadow cache 基于 rect/w/h/radius/blur/spread/color。
- rotate 后应该如何合成。
- v1 是否先将 shadow 标记为 unsupported under rotation，或设计 texture cache + affine composite。
- 不允许静默错误。

SVG Vector：
- SVG vector path 如何继承 affine。
- Icon layout 和 affine 的组合顺序。

SVG Raster Fallback：
- raster pixel size 如何根据 transformed bounds 估算。
- raster result 如何 affine composite。

CustomPaint：
- CustomPaintCx 是否需要包含 transform。
- PaintTarget 是否暴露 push_transform/pop_transform 或 current_transform。
- 自定义 painter 如何避免拿到错误 screen_rect。

8. PaintTarget 和 PaintOp 设计
说明 PaintTarget trait 是否需要新增：
- push_transform / pop_transform
- draw_rect with transform
- draw_path with transform
- draw_image with transform
或者保持 draw commands 携带 transform。
说明 RecordingPaintTarget 如何记录 affine。
说明测试如何断言 transform，不要只断言已经变换后的浮点点位。
说明 RendererPaintTarget 如何适配。

9. Hit Test 设计
说明 hit_recursive 如何使用 accumulated inverse affine。
说明 overflow visible/hidden/scroll 在 affine 下如何判断。
说明 axis-aligned bounds 和 transformed bounds 的关系。
说明 hit chain 顺序和 stacking 不应改变。
说明 rotate 启用条件：只有 paint 支持同一语义后 hit 才能启用。

10. 迁移计划
必须给出详细阶段，每阶段列：
- 目标
- 修改文件
- 新增文件
- 具体改动
- 测试
- 验收标准
- 风险
- 回滚方式

阶段建议：
Phase 0：设计文档和 invariants。
Phase 1：新增 Affine2D 数学模块和测试，不改变行为。
Phase 2：tree paint/hit 内部改用 Affine2D，但 rotate 仍 gated，行为不变。
Phase 3：Renderer command/prepare 引入 transform-aware 数据结构。
Phase 4：Path/Rect/Clip/Image 支持 affine，启用非文本基础图元 rotate。
Phase 5：SVG vector/raster、circle、custom paint recording 完整接入。
Phase 6：Text 和 Shadow 明确最终策略，不能留下 silent mismatch。
Phase 7：启用 Transform::rotate 的 paint/hit 一致行为。
Phase 8：清理旧 PaintTransform tx/ty/scale，更新文档和回归测试。

11. 测试计划
必须具体到测试类型和关键断言：
- Affine2D compose/inverse/apply tests。
- rotate 90/180 degrees tests。
- paint recording tests。
- hit inverse consistency tests。
- prepare geometry tests。
- image affine quad instance tests。
- transformed stencil clip tests。
- SVG vector transform tests。
- raster fallback transformed bounds tests。
- text/shadow limitation tests。
- workspace canvas camera regression tests。
说明哪些测试无需 GPU，哪些用 gui/src/renderer/test_support.rs。

12. 代码债务清理清单
列出最终必须删除或替换的旧概念：
- PaintTransform tx/ty/scale
- inverse_supported_transform
- rotate ignored tests
- comments saying rotate reserved/inert
- any duplicated transform math
- screen-space-only assumptions where no longer valid
说明每项在哪个阶段清理。

13. 风险和决策点
列出必须在实现前确认的关键决策：
- transform origin 是否加入。
- text rotation 策略。
- shadow rotation 策略。
- renderer command 携带 affine 还是预变换。
- path cache key 策略。
- non-uniform scale/skew 是否内部支持但外部不开放。
每个决策要给推荐选项和理由。

14. 最终输出格式
请用中文输出。
先给“推荐总体方案”。
然后给“模块和接口设计”。
然后给“分阶段执行计划表”。
然后给“测试计划”。
然后给“代码债务清理清单”。
最后给“开放问题与推荐决策”。

注意：
- 不要写代码。
- 不要泛泛而谈。
- 不要留下“以后再说”的模糊项。如果某项暂不实现，必须有显式 guard、文档、测试和后续阶段。
- 计划必须能直接作为后续实现任务拆分依据。
