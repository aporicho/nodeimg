# targetv2 路线图

> "机制一次设计好,内容持续开放" 的落地顺序。

---

## §1 核心原则

> **机制冻结,内容开放。**

所有 **17 条机制**必须在写代码前**全部在文档中存在**。首版代码可以只实现"占位",但机制(字段、trait、层)必须真实存在。详见 `architecture-invariants.md` §6 和本文件 §2。

**产品宣言**: nodeimg **集 DaVinci / TouchDesigner / Nuke 三家之大成**——融合三家的核心模型(per-frame cook + ambient time context + 节点无时间感),首版用**单一 demo** 证明这套机制能承载完整工作流。

**首版 Demo · 实时视频调色工作流:**

```
load_video (input.mp4)
    ↓
[ai_lut_generate]              ← AI 节点(慢,realtime_capable=false,首次跑后缓存)
    ↓
primary_color_grade            ← 实时节点(可拖滑块,可加 keyframe 动画)
    ↓
gaussian_blur                  ← 实时节点
    ↓
save_video (output.mp4)
```

**首版要证明的能力:**

1. **视频读写**: load_video / save_video(MP4 + ffmpeg)
2. **统一节点模型**: 同一个 `gaussian_blur` 节点处理单图 / 序列帧 / 视频帧 —— 节点代码不变
3. **关键帧动画**: `primary_color_grade` 的 lift/gamma/gain 加 keyframe,从 frame 0 渐变到 frame 100
4. **60fps 实时预览**: 进入 Continuous 模式,拖滑块时实时看效果,持续 cook
5. **慢节点缓存复用**: AI LUT 节点跑一次后缓存,Continuous 模式跳过它直接用 cache
6. **完整导出**: 切到 OneShot Full 模式,导出整段视频

**不在首版的:**

- ❌ 多通道 EXR 读写(机制保留,文件 I/O 延后)
- ❌ 3D 几何 / 渲染(完全不在路线图)
- ❌ 音频处理(`AtomicType::Audio` 仍为占位)
- ❌ 实时摄像头 / MIDI / OSC 输入(Continuous mode 已支持,但输入源延后)
- ❌ OCIO 专业色彩管理(机制保留,首版只支持 SRgb / LinearRec709)

---

## §2 18 条必须在编码前冻结的机制

| # | 机制 | 权威文档 | 首版状态 |
|---|------|---------|---------|
| 1 | 8 条真源 | `architecture-invariants.md` | 全部必须存在 |
| 2 | 8 层分层 + 所有权清单 | `first-principles-architecture.md` | 每层必须有文档 |
| 3 | `AtomicType` 封闭枚举 + 首版宽清单 | `0.1.0-models.md` §1.1 | 占位类型无执行器支持 |
| 4 | `CookingContext` + 公式含 `cooking_context_hash` | `0.1.3-cooking-context.md` + `0.1.1` §3.1 | **首版含 frame 维度真实支持** |
| 5 | `Capability` 路由主键 + `NodeDef.requires` | `0.1.4-capability.md` | 首版 requires 可为空 |
| 6 | `Materializable` 协议 | `0.1.5-value-materialization.md` | 首版 `ImageValue` 实现 |
| 7 | `NodeRegistry` façade 单入口 | `extensibility.md` | 必须是唯一入口 |
| 8 | Session 层存在 | `4.12.0-session.md` | 首版可极简 |
| 9 | `ExecutionRequest.fidelity` 字段 | `0.1.1-execution-models.md` §1.1 | 首版 Preview=Full |
| 10 | Runtime 持有 single-flight 全局守卫 | `4.1.2-runtime.md` | 必须从首版就是 Runtime |
| 11 | `ExecSignature` 完整公式含所有输入 | `0.1.1-execution-models.md` §3.1 | 占位字段恒零 |
| 12 | 持久化三分法完整覆盖 | `architecture-invariants.md` §2 真源 ⑦ | 每个状态必须有归属 |
| 13(M1) | **多通道 `ImageValue` + `ImageMetadata`** | `0.1.0-models.md` §1.3.1 / §1.3.2 | **机制保留;首版只用 R/G/B/A + SRgb/LinearRec709;Z/ObjectID 通道占位** |
| 14(M2) | **`ParamValueMode` + `Curve` + 自动 cooking_sensitivity 推导** | `0.1.0-models.md` §2.5.1 / §2.5.2 | 首版支持 Constant + Animated{Linear/Constant};Expression 占位 |
| 15(M3) | **`ExecutionMode::OneShot` / `Continuous` 字段** | `0.1.1-execution-models.md` §1.4 | **首版 Continuous 真实实现**(慢节点 cache-only) |
| 16(M4) | **`NodeDef.realtime_capable: bool` 字段** | `0.1.0-models.md` §2.6 | 首版必填(默认 true);AI 节点显式 false |
| 17(M5) | **Executor lifecycle hooks: `on_plan_started` / `on_plan_finished`** | `4.11.2-executor-contract.md` + `0.1.1-execution-models.md` §1.5 | 首版必须存在(default no-op);视频 encoder 必须使用 |
| 18(M6) | **执行器持久化 disk cache 模式 + 标准目录约定** | `4.11.2-executor-contract.md` §长寿命内部状态 | 首版机制就位;ApiVideoGenerateExecutor 首版必须实现 |

---

## §3 迁移阶段

### 阶段 A · 文档定稿

**目标:** targetv2 所有文档完成,通过 review,成为权威文档。

**A1~A14(基础文档,已完成):**

- [x] `spec.md`
- [x] `README.md`
- [x] `architecture-invariants.md`(8 真源)
- [x] `first-principles-architecture.md`(8 层)
- [x] `0.0.0-architecture.md`
- [x] `0.0.1-engine-contract.md`
- [x] `0.0.2-module-contract-template.md`
- [x] `0.1.0-models.md`
- [x] `0.1.1-execution-models.md`
- [x] `0.1.2-interaction-loop.md`(新)
- [x] `0.1.3-cooking-context.md`(新)
- [x] `0.1.4-capability.md`(新)
- [x] `0.1.5-value-materialization.md`(新)
- [x] `extensibility.md`
- [x] `4.0.0-engine.md`
- [x] `4.1.0-scheduler.md`
- [x] `4.1.1-planner.md`(新)
- [x] `4.1.2-runtime.md`(新)
- [x] `4.1.3-capability-registry.md`(新)
- [x] `4.11.2-executor-contract.md`
- [x] `4.12.0-session.md`(新)
- [x] `roadmap.md`(本文件)

**A15~A19(三位一体机制扩展,已完成):**

- [x] **A15** · M1 多通道 `ImageValue` + `ImageMetadata`(`0.1.0-models.md` §1.3.1/§1.3.2 + `0.1.5-value-materialization.md` §2.3 + `architecture-invariants.md` 真源 ③)
- [x] **A16** · M2 `ParamValueMode` + `Curve` + 自动 cooking_sensitivity 推导(`0.1.0-models.md` §2.5.1/§2.5.2 + `0.1.3-cooking-context.md` §2.4 + `0.1.1-execution-models.md` §3.1.1 + `4.1.1-planner.md` build_signature)
- [x] **A17** · M3 `ExecutionMode::OneShot/Continuous` + `TickSource`(`0.1.1-execution-models.md` §1.4 + `4.1.2-runtime.md` try_start_run + `architecture-invariants.md` 真源 ⑥)
- [x] **A18** · params_hash 与动画参数的采样次序(并入 A16)
- [x] **A19** · roadmap.md 阶段 B 扩展(本文件)

**C1~C9(单 Demo + 视频 + Continuous 真实 + 持久化 cache,完成中):**

- [x] **C1** · roadmap.md 单 Demo 重写 + §2 18 条机制 + §3 阶段 B 28 任务(本文件)
- [ ] **C2** · README.md 单 Demo 产品宣言更新
- [ ] **C3** · architecture-invariants.md 真源 ②/⑤/⑥ 更新(lifecycle hooks / realtime_capable / Continuous 真实)
- [ ] **C4** · 0.1.0-models.md `NodeDef.realtime_capable` 字段 + 通道首版状态(Z/ObjectID 改占位)
- [ ] **C5** · 0.1.1-execution-models.md Continuous 真实实现 + lifecycle hooks signatures
- [ ] **C6** · 4.1.2-runtime.md Continuous 流程 + 慢节点处理 + lifecycle 调用
- [ ] **C7** · 4.11.2-executor-contract.md lifecycle 方法 + 长寿命内部状态合法性
- [ ] **C8** · 0.1.4-capability.md 视频 Capability 首版(`video.decode/encode` + `ai.video_generate`;`exr.*` 占位)
- [ ] **C9** · 4.11.2-executor-contract.md 持久化 disk cache 模式 + 标准目录约定(并入 C7 文件)

### 阶段 B · 代码骨架

**目标:** 现有 `crates/nodeimg-engine` 代码按 targetv2 的 12 条机制添加**占位实现**,通过 `cargo test` 和 `cargo build`。

**任务:**

1. **B1 — 扩充 `AtomicType` 枚举**
   - 添加 `Video` / `Audio` / `VectorGraphic` / `LayoutDocument` / `BoundingBox` / `Keypoint` / `RemoteRef`
   - 每个占位类型实现 `Value` 变体
   - 占位类型在 `DataType::List` 白名单中不开放
   - **风险:** 低(纯 enum 扩充)

2. **B2 — 添加 `NodeDef.requires` 字段**
   - 字段首版可为空 `Vec<CapabilityId>`
   - 所有现有节点 `requires = vec![]`
   - **风险:** 低(字段新增,向后兼容)

3. **B3 — 添加 `Materializable` trait**
   - 为 `ImageValue` 实现
   - 现有代码逐步迁移: 所有直接访问 `ImageValue.cpu/gpu` 的地方改用 `materialize(target)`
   - **风险:** 中(需要改执行器代码)

4. **B4 — 添加 `CookingContext` 类型**
   - 首版 `dimensions` 字段为空 `HashMap`
   - `ExecutionRequest` 添加 `cooking_context: CookingContext`
   - Planner 签名加 `cooking_context: &CookingContext` 参数
   - **风险:** 中(接口变更,但全部参数可默认空)

5. **B5 — 添加 `EvaluationFidelity` 字段**
   - `ExecutionRequest.fidelity: EvaluationFidelity`
   - 首版所有构造点传 `Full`(或根据 UI 路径选 `Preview`)
   - 执行器忽略 fidelity(行为等同 Full)
   - **风险:** 低

6. **B6 — 扩充 `ExecSignature` 公式**
   - 添加 `cooking_context_hash: u64`(首版恒零)
   - 添加 `capability_version: u32`(首版恒零)
   - hash 算法不变(`xxHash3-128`)
   - **风险:** 中(**签名会变化一次**—— 从 targetv2 文档定稿日起;之后不再变)

7. **B7 — 创建 `CapabilityRegistry` 模块**
   - 首版空注册表 + freeze() 机制
   - 现有 3 个执行器(Image/AI/API)实现 `provides()`
   - Runtime 路由按 Capability 查表(未命中则 fallback 到 `executor_type`)
   - **风险:** 中(新增模块,需与 Scheduler 集成)

8. **B8 — 拆分 Scheduler 为 Scheduler facade + Planner + Runtime**
   - `4.1.0-scheduler.md` / `4.1.1-planner.md` / `4.1.2-runtime.md` 的代码实现
   - Scheduler facade 对外接口保持不变
   - Planner 拉出纯函数 `plan()`
   - Runtime 拉出 single-flight 守卫
   - **风险:** 高(大拆分,需要仔细单测)

9. **B9 — 创建 Session 层模块**
   - 首版最小骨架: `Session { undo_stack, preview_cache, current_run_ref }`
   - 暴露 `SessionHandle` trait
   - GUI 改走 Session 路径
   - CLI 保持直通 Runtime
   - **风险:** 中(需要重构 GUI 和 engine 的交互代码)

10. **B10 — 拆分 preview cache**
    - `CacheManager` 保持为正式 cache
    - Session 新建独立的 `PreviewCache`
    - 两者不共享存储
    - **风险:** 低

11. **B11 — 更新 `NodeRegistry` façade**
    - 所有外部扩展入口收敛到 `NodeRegistry::register(source)`
    - 内置执行器走引擎启动代码直接注册 `CapabilityRegistry`(不对外)
    - builtins / Python / Api provider 都走这个路径
    - **风险:** 低

12. **B12 — 填写真源 ⑦ 持久化三分法**
    - 为每个状态项明确归属
    - 不允许有"待定"
    - **风险:** 低(纯文档层)

13. **B13 — 修订 `4.2.0-graph-controller.md` 和 `4.7.0-project-manager.md`**
    - 明确 "GraphController 可脱离 ProjectManager 独立运行"(Project-Graph 正交)
    - GraphController 添加两种构造方式:
      - `new_with_undo_tracking()` —— GUI / Session 路径使用
      - `new_without_undo()` —— CLI / Server / 测试路径使用
    - `without_undo` 模式下 `undo()` / `redo()` 返回 `Err(UndoDisabled)`
    - ProjectManager 明说: "无 Project 的 Graph" 是合法路径,不是边界情况
    - **风险:** 低(纯文档 + 小规模 GraphController 构造函数修改)

14. **B14 — 更新 `4.11.2-executor-contract.md` 对 fidelity 的响应规则**
    - 执行器 `execute(cap_id, req)` 接收 `req.fidelity`
    - Preview 模式: 执行器**可以**降分辨率/降精度
    - Full 模式: 执行器**绝不**偷偷降级
    - 首版允许 Preview 行为等同 Full
    - **风险:** 低

### 阶段 B 扩展(单 Demo · 实时视频调色)—— B15~B28

**目标:** 让首版完成单个 Demo——**实时视频调色工作流**。Demo 描述见 §1。该 Demo 一个跑下来,验证 18 条机制全部就位 + DaVinci/TouchDesigner/Nuke 三家核心模型都被覆盖。

15. **B15 — 多通道 ImageValue 机制(机制保留,首版只用 RGBA)**
    - `ImageValueInner` 改为 `channels: BTreeMap<ChannelId, ChannelData>` + `metadata: ImageMetadata`
    - **首版只启用通道**: `R/G/B/A`
    - **首版只启用色彩空间**: `SRgb` 和 `LinearRec709`
    - `Z/ObjectID/N.X/N.Y/N.Z/MV.X/MV.Y/AOV.*` 在类型系统中**保留**(`ChannelId` 是开放字符串),但首版**没有节点使用**
    - 通过 `image::DynamicImage` 实现 CPU 端;`wgpu::Texture` 实现 GPU 端
    - **风险:** 中(`ImageValue` 内部结构变化,所有现有执行器需要适配 `channel(...)` access API)

16. **B16 — `ParamValueMode` + `Curve` + 关键帧动画**
    - `ParamValueMode` 三个变体: `Constant` / `Animated(Curve)` / `Expression(占位)`
    - `Curve` 首版支持 `InterpolationType::{Constant, Linear}`,Bezier/Smooth 占位
    - `Curve::sample(ctx)` 实现按 keyframe 排序查找 + 线性插值
    - `Node.params` 类型从 `HashMap<String, Value>` 改为 `HashMap<String, ParamValueMode>`
    - 首版所有 `NodeDef` 用 `Constant` 默认 mode,**用户在 UI 加 keyframe 后**升级为 `Animated`
    - **风险:** 中(影响 Node 序列化、UI 参数控件、所有节点测试)

17. **B17 — Planner: build_params_hash 按 ctx 采样 + effective_cooking_sensitivity 自动推导**
    - 实现 `0.1.1-execution-models.md` §3.1.1 的采样规则: 跳过参数接口、按 ctx 采样 Animated、按 key 字典序 hash
    - 实现 `0.1.0-models.md` §2.5.2 的合并规则: NodeDef 静态声明 ∪ 节点实例的 Animated 参数依赖维度
    - 单元测试: 同 curve 在 frame=0 与 frame=100 产出不同 hash;节点加 keyframe 后自动获得 frame sensitivity
    - **风险:** 低

18. **B18 — `CookingContext.frame` 维度真实支持**
    - `CookingContext.dimensions["frame"]: DimensionValue::Int` 启用
    - Planner 的 `enumerate_cartesian` 实现 frame range 笛卡尔积(OneShot 模式下)
    - Scheduler facade 加 `set_cooking_range(range: HashMap<DimensionId, RangeSpec>)` 接口
    - **风险:** 中(Planner 的核心循环修改)

19. **B19 — `NodeDef.realtime_capable: bool` 字段**(M4)
    - 添加字段,所有现有 NodeDef 默认 `true`
    - AI / API / 慢节点显式标 `false`
    - 单元测试: NodeRegistry 注册时校验 + Runtime 读取
    - **风险:** 极低(纯字段添加)

20. **B20 — Executor lifecycle hooks: `on_plan_started` / `on_plan_finished`**(M5)
    - Executor trait 添加两个 default no-op 方法
    - Runtime 在执行 Plan 边界调用对应 hook
    - 单元测试: 自定义 Executor 验证 hook 被正确调用
    - **风险:** 低(接口添加)

21. **B21 — 执行器持久化 disk cache 模式**(M6)
    - `4.11.2-executor-contract.md` §长寿命内部状态 完整说明
    - 提供 `nodeimg_engine::cache_dir(executor_name)` helper(返回 `~/.cache/nodeimg/<executor>/`)
    - 标准约定: cache key = `xxhash3_128(stable_serialize(input))`,cache file = `<key>.<ext>`
    - LRU 淘汰策略 + 默认大小限制(配置文件控制,默认 10GB)
    - **风险:** 低(库函数 + 约定)

22. **B22 — `ExecutionMode::Continuous` 真实实现**(M3 转正)
    - Runtime 的 tick loop 实现(`OneShot` 之外的第二种长生存期 run 状态)
    - 进入: `try_start_run(plan, Continuous { tick_source, target_fps })`
    - 退出: `stop_continuous()` 显式调用 / `cancel`
    - tick 内: 用最新 plan + clock-driven context 执行一遍
    - frame 维度: 按 `target_fps` 推进,默认循环播放
    - **风险:** 中(新的 Runtime 状态机分支)

23. **B23 — 慢节点在 Continuous 模式下的 cache-only 行为 + `NodeMissingInContinuous` 事件**
    - Runtime 在 Continuous 模式遍历节点时,对 `realtime_capable=false` 的节点:
      - 命中正式 cache → 用缓存
      - 未命中 → 不重新执行;发 `NodeMissingInContinuous { node_id }` 事件;用占位(黑帧)输出
    - GUI 收到事件后提示"AI 节点未生成,先 OneShot warm 一次"
    - **风险:** 低(简单分支)

24. **B24 — `load_video` / `save_video` 节点(FFmpeg 集成)**
    - 通过 `ffmpeg-next` crate 读写 MP4 等常见容器
    - `load_video`: `cooking_sensitivity=["frame"]`, `realtime_capable=true`(顺序访问), seek-based decoder pool 由执行器内部状态管理
    - `save_video`: `cooking_sensitivity=["frame"]`, `realtime_capable=false`(写文件), 使用 `on_plan_started` 打开 encoder, 每帧 execute 写入, `on_plan_finished` 调用 encoder.finalize() 关闭文件
    - 注册 `Capability::video.decode` / `Capability::video.encode`
    - **风险:** 中-高(FFmpeg API 的工程量较大,首版只支持常见 codec)

25. **B25 — `ApiVideoGenerateExecutor` 示例(AI 节点 + disk cache)**
    - 节点声明: `cooking_sensitivity=["frame"]`, `realtime_capable=false`, `purity=Impure`
    - 参数: `prompt: String`, `duration: Int`, `seed: Int`
    - 执行器内部三级 cache:
      - L1: 进程内 `Mutex<HashMap<GenerationKey, Arc<GeneratedVideo>>>`
      - L2: 持久化磁盘 cache(`~/.cache/nodeimg/ai.video_generate/<key>.mp4`)
      - L3: 调外部 API(慢且贵)
    - 首版可以用 mock(本地生成 96 帧渐变图)代替真实 API,仅证明机制
    - **风险:** 低(机制证明,不要求真实 API 集成)

26. **B26 — `primary_color_grade` 节点(DaVinci 风格)**
    - 参数: lift / gamma / gain(每个 `[f32; 3]` RGB 三元组),全部支持 `Animated(Curve)`
    - 实现 GPU shader 做 lift/gamma/gain 变换
    - 颜色空间感知: 在 LinearRec709 空间应用变换,SRgb 输入自动 to-linear / from-linear(读 `metadata.color_space`)
    - 注册 `Capability::raster.primary_grade`
    - **风险:** 低(纯 GPU shader)

27. **B27 — `pixel_fluid_animation` 节点(TouchDesigner 风格,执行器内部状态)**
    - 输入: `Image`(每帧的输入)
    - 输出: `Image`(每帧的输出)
    - `cooking_sensitivity=["frame"]`(输出随 frame 变化)
    - `realtime_capable=true`(GPU 单步 ~3-5ms)
    - 执行器内部状态: `Mutex<HashMap<NodeId, FluidSimState>>`,每个节点实例独立
    - 顺序访问优化: 当请求 frame N+1 时,只增量推进一步;乱序访问时重置从 frame 0 开始
    - 注册 `Capability::raster.fluid_sim`
    - **风险:** 中(GPU 状态管理)

28. **B28 — 单 Demo 集成测试**

    完整 demo 工作流:

    ```
    [api_video_generate prompt="ocean waves" duration=4]   ← realtime=false, disk cache
            ↓
    [primary_color_grade lift/gamma/gain w/ keyframes]      ← realtime=true, animated params
            ↓
    [gaussian_blur sigma=2]                                 ← realtime=true
            ↓
            ├──→ [save_video output.mp4]                    ← realtime=false, lifecycle hooks
            │
            └──→ [pixel_fluid_animation viscosity=0.5]      ← realtime=true, internal GPU state
                    ↓
                [preview_output]
    ```

    **测试验证清单:**

    - [ ] 第一次跑 OneShot Full → API 同步等待 → 96 帧入 disk cache + Runtime cache + 写 output.mp4
    - [ ] 重启 nodeimg → 同 prompt 再跑 → disk cache 命中,无 API 调用
    - [ ] 进入 Continuous 模式 → 60fps 实时播放(预热后)
    - [ ] 拖动 color_grade 滑块 → 实时看到效果(掉到 ~30fps 可接受)
    - [ ] 加 keyframe(frame 0 lift=蓝;frame 95 lift=橙)→ 播放看到颜色渐变
    - [ ] 切回 OneShot Full → 导出整段动画视频
    - [ ] 改 prompt → 触发新 API 调用 + 新 disk cache 条目

    **同时验证的机制:**

    | 机制 | Demo 中如何用到 |
    |------|---------------|
    | M1 多通道 ImageValue | 视频帧本质是多通道(首版只用 RGBA) |
    | M2 ParamValueMode + 自动 sensitivity | color_grade 加 keyframe 后自动按 frame 分缓存 |
    | M3 Continuous 模式 | 60fps 实时预览 |
    | M4 realtime_capable | 慢节点(api/save_video)与快节点(grade/blur/fluid)的区分 |
    | M5 lifecycle hooks | save_video 用 on_plan_started/finished 管理 encoder |
    | M6 持久化 disk cache | api_video_generate 跨 session 缓存 |
    | CookingContext.frame | 视频按帧驱动 |
    | 三级 cache | preview_cache / 正式 cache / 执行器内部 cache 协同 |
    | 签名传播 | color_grade 自动跟随 api_video_generate 的 per-frame 签名 |

    - **风险:** 中(集成验证整个 stack;FFmpeg 是最大风险点)

### 阶段 C · 垂直切片验证

**目标:** 用一个简单场景验证 targetv2 的 12 条机制真的能一起工作。

**场景:** 一张图 = `load_image → gaussian_blur → save_image`,完整走一遍:

1. GUI 打开 Project
2. 用户添加三个节点,连线
3. 用户改 `gaussian_blur.sigma` 参数
4. Session 发 `ParamChanged` → 构造 `ExecutionRequest(fidelity = Preview)`
5. Runtime 抢占(如果有 Full 在跑)或直接启动
6. Planner 产出 plan(CookingContext 为空,cooking_context_hash = 0)
7. Runtime 按 Capability 路由到 GpuRasterExecutor
8. 执行器通过 `materialize(GpuResident)` 获取输入
9. 执行器返回结果
10. Runtime 写正式 cache + 不写 artifact(因为 Preview)
11. Session 更新 preview_cache
12. GUI 收到 `SessionEvent::PreviewUpdated`,刷新
13. 用户点"导出" → Session 发 `ExecutionRequest(fidelity = Full)`
14. Runtime 走 Full 路径,写 artifact
15. `save_image` 执行器(有 `FileWrite` 副作用)写文件

**验证点:**

- [ ] Scheduler facade 对外接口与 target 时代无回归
- [ ] Planner 可以被单元测试为纯函数
- [ ] Runtime 的 single-flight 守卫可以拒绝并发请求
- [ ] Preview 抢占 Full 的行为正确
- [ ] `cooking_context_hash` 恒零时签名与 target 时代兼容
- [ ] `capability_version` 恒零时签名与 target 时代兼容
- [ ] CLI 不经过 Session 可以执行完整图
- [ ] 执行器无落盘权限的约束通过 `save_image` 的副作用声明满足
- [ ] preview cache 与正式 cache 独立淘汰

### 阶段 D · 水平扩展(可选)

**目标:** 在稳定的骨架上扩展内容,不动核心。每一项都是"填槽"。

- D1 — 加 `CookingContext.frame` 维度实现(真实序列帧)
- D2 — 加 `VideoExecutor` + `video.decode` / `video.encode` Capability
- D3 — 加 `VectorExecutor` + `vector.render` / `vector.boolean` Capability
- D4 — 加 `LayoutExecutor` + `layout.compose` Capability
- D5 — 加 preview 降分辨率实现
- D6 — 加 `DiskFile` MaterializationTarget
- D7 — 加热插拔 Capability(破坏性变更,需改 M3 决议)
- D8 — 加 MCP / HTTP Server 前端

**关键:** D 阶段的每一项都**不**改 `architecture-invariants.md`、`first-principles-architecture.md`、`0.1.x` 系列文档。如果某个 D 任务要求动这些文档,说明机制清单不完整——先补机制。

---

## §4 向后兼容策略

### 4.1 `ExecSignature` 兼容性

- target 时代的签名公式: `hash(upstream + params + node_def)`
- targetv2 的签名公式: `hash(upstream + params + node_def + cooking_ctx_hash + cap_version)`
- **迁移策略:** targetv2 首版 `cooking_ctx_hash = 0`,`cap_version = 0`,但与 target 签名**位级别不同**——因为 xxHash3 对 `(a, b, c)` 和 `(a, b, c, 0, 0)` 的输出完全不同
- **结论:** targetv2 首版需要**一次性** 缓存清空(通过 `clear_cache`),之后签名稳定

**这是唯一一次破坏:**

| 迁移点 | 签名兼容? | 代价 | 级别 |
|--------|----------|------|------|
| target → targetv2 首版 | **否** | 一次性 `clear_cache` | 破坏性(唯一) |
| targetv2 首版 → 未来加 CookingContext 维度 | **是**(旧节点 `cooking_sensitivity = []` → context hash 仍为 0) | 零代价 | 零代价 |
| targetv2 首版 → 未来加 Capability 版本 | **是**(旧 Capability 版本仍为 0) | 零代价 | 零代价 |
| targetv2 首版 → 未来 **占位类型转正**(`Video` 从占位变真实) | **是** | 零代价 | 零代价 |
| targetv2 首版 → 未来 **新增全新 AtomicType**(不在占位清单里) | **否** | 需要先改真源 ③ + 所有 match 更新 | 轻破坏 |
| targetv2 首版 → 未来加新 `MaterializationTarget` 变体 | **是**(不影响签名) | 所有 Materializable 实现更新一次 | 轻破坏 |

**"扩展代价分级"** 详见 `extensibility.md` §0。

**关键: 这一次性破坏必须对用户明示。**

- targetv2 首版启动时 **自动清空** target 遗留的 cache 条目(通过版本标记识别)
- 项目文件中不包含 cache(真源 ⑦),所以项目文件本身不受影响
- 用户打开旧项目后,首次执行会触发一次"全图重算"——这是预期行为
- **此后**增量失效恢复正常,加任何新维度都不破坏

**权衡理由:** 这是从 target 到 targetv2 的**唯一一次**破坏性缓存清零。之后加任何新维度 / 新 Capability 都不再触发清零——这是占位字段的价值。

### 4.2 `NodeDef.requires` 兼容性

- target 时代的 `NodeDef` 没有 `requires` 字段
- targetv2 的 `NodeDef` 有 `requires: Vec<CapabilityId>`,首版可空
- **迁移策略:** 项目文件 schema 不变(首版不存 `requires`),运行时从 NodeManager 读 static_def
- **向前兼容:** 旧项目文件可以被 targetv2 打开,所有节点走 `executor_type` fallback

### 4.3 `ExecutionRequest.fidelity` 兼容性

- target 时代无此字段
- targetv2 新增
- **迁移策略:** `ExecutionRequest` 是引擎内部对象,不持久化——可直接变更

---

## §5 验证与冻结

每个阶段完成后,核对:

**阶段 A 完成标准:**

- [ ] 12 条机制全部在 targetv2 文档中存在
- [ ] 所有真源的"权威文档"字段指向的 `.md` 都已创建
- [ ] `README.md` 的 "与 target 的关系" 与实际文档一致
- [ ] 没有"待确认"字段留在关键路径上

**阶段 B 完成标准:**

- [ ] `cargo build --release` 通过
- [ ] `cargo test --workspace` 通过
- [ ] target 时代的所有单元测试仍然通过(可能需要一次性 cache 清空)
- [ ] 新增模块(`4.1.1-planner.md` / `4.1.2-runtime.md` / `4.1.3-capability-registry.md` / `4.12.0-session.md`)有对应的代码模块

**阶段 C 完成标准:**

- [ ] §3 阶段 C 的 14 个验证点全部通过
- [ ] GUI 可以交互式使用(参数变化 → preview 刷新)
- [ ] CLI 可以执行完整图

---

## §6 不是目标

以下内容**不在** targetv2 路线图内,避免 scope creep:

- GUI 侧的交互体验重构(归 `2.x.x`)
- Python 后端架构变化(归 `6.x.x`)
- 项目文件格式变化(归 `5.x.x`;targetv2 不改项目文件)
- 具体节点库的扩充(内容开放,不是机制变化)
- 具体执行器的性能优化(内容开放)
- 跨进程 / 跨机器的分布式执行(未来工作)
- 实时流式处理(破坏 `Planner` 纯函数范式,未来工作)

---

## §7 与其他文档的关系

- `architecture-invariants.md` —— 8 条真源与 12 条机制表
- `first-principles-architecture.md` —— 8 层分层
- `README.md` —— targetv2 全局索引
- `extensibility.md` —— 内容扩展(不是核心变更)
- `docs/target/` —— 被继承的旧文档体系;迁移完毕前双 live
